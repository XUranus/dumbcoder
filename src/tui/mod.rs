mod action;
mod app;
mod event;
pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

use crate::config::{Config, DUMBCODER_DIR};
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::security::SecurityFilter;

use self::app::{App, AppEvent};
use self::event::EventHandler;

pub async fn run() -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());
    let client = ModelClient::new(&config.model)?;

    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    let store = if db_path.exists() {
        IndexStore::open(&db_path).ok()
    } else {
        None
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new();
    let events = EventHandler::new(Duration::from_millis(50));

    let result = run_loop(&mut terminal, &mut app, &events, &client, &root, &security, store).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: &EventHandler,
    client: &ModelClient,
    root: &std::path::Path,
    security: &SecurityFilter,
    store: Option<IndexStore>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        match events.next()? {
            AppEvent::Tick => {}
            AppEvent::Key(key) => {
                let action = app.handle_key_event(key);
                action::execute(action, app, client, root, security, &store).await;
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
