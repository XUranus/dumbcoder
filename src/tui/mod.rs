mod action;
mod app;
mod event;
mod markdown;
pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::config::{Config, DUMBCODER_DIR};
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::security::SecurityFilter;
use crate::session;

use self::app::{App, AppEvent, AppMode};
use self::event::EventHandler;

pub async fn run(resume: Option<&str>) -> Result<()> {
    let (root, _is_project) = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());
    let client = ModelClient::new(&config.model)?;

    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    let store = if db_path.exists() {
        IndexStore::open(&db_path).ok()
    } else {
        None
    };

    // Determine session ID: resume existing or create new
    let session_id = if let Some(id) = resume {
        id.to_string()
    } else {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        format!("{}", now.as_secs())
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and optionally restore session
    let mut app = App::new(session_id.clone());

    if let Some(id) = resume {
        // Load specific session
        match session::load_session(&root, id) {
            Ok(sess) => {
                app.messages = sess.messages;
                app.plan_content = sess.plan;
                app.mode = if sess.mode == "plan" { AppMode::Plan } else { AppMode::Chat };
                app.push_system_message(&format!("Resumed session: {id}"));
            }
            Err(e) => {
                app.push_system_message(&format!("Could not load session '{id}': {e}"));
            }
        }
    } else {
        // Auto-restore latest session if exists and has messages
        if let Some(latest) = session::latest_session(&root) {
            if !latest.messages.is_empty() {
                app.messages = latest.messages;
                app.plan_content = latest.plan;
                app.mode = if latest.mode == "plan" { AppMode::Plan } else { AppMode::Chat };
                app.push_system_message(&format!("Restored session: {} ({} msgs)", latest.id, app.messages.len()));
            }
        }
    }

    // Channel for async model results
    let (model_tx, model_rx) = mpsc::channel::<Result<String>>(4);
    let mut events = EventHandler::new(Duration::from_millis(50), model_rx);

    // Run the event loop
    let result = run_loop(&mut terminal, &mut app, &mut events, &config, &client, &root, &security, store, model_tx).await;

    // ── Exit: restore terminal and print conversation ──
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print full conversation to terminal scroll buffer
    print_conversation(&app);

    // Final session save
    action::save_session(&root, &app);

    result
}

/// Print conversation to stdout so it remains in the terminal scroll buffer.
fn print_conversation(app: &App) {
    if app.messages.is_empty() {
        return;
    }
    println!();
    println!("──── dumbcoder session {} ────", app.session_id);
    println!();
    for msg in &app.messages {
        match msg.role.as_str() {
            "user" => {
                println!("  ▸ {}", msg.content);
                println!();
            }
            "assistant" => {
                for line in msg.content.lines() {
                    println!("    {line}");
                }
                println!();
            }
            "system" => {
                for line in msg.content.lines() {
                    println!("  ℹ {line}");
                }
            }
            _ => {}
        }
    }
    println!("──── end of session ────");
    println!();
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &mut EventHandler,
    config: &Config,
    client: &ModelClient,
    root: &std::path::Path,
    security: &SecurityFilter,
    store: Option<IndexStore>,
    model_tx: mpsc::Sender<Result<String>>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        match events.next().await? {
            AppEvent::Tick => {
                app.tick_spinner();
            }
            AppEvent::Key(key) => {
                let action = app.handle_key_event(key);
                action::execute(action, app, config, client, root, security, &store, model_tx.clone()).await;
            }
            AppEvent::ModelResult(result) => {
                app.receive_model_response(result);
            }
        }

        if !app.running {
            break;
        }
    }
    Ok(())
}
