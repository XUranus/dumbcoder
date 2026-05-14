use std::process::Command;

use crate::config::Config;
use crate::context::CodeContext;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::session;
use crate::tool;
use tokio::sync::mpsc;

use super::app::{App, AppAction, AppMode, AppStatus};

const TOOL_SYSTEM_PROMPT: &str = r#"You are a code assistant with access to tools. When you need to read files, write code, or run commands, use tool calls.

To use a tool, output a JSON block in this exact format:
```tool
{"name": "tool_name", "args": {"arg1": "value1"}}
```

Available tools:
- read_file: {"name": "read_file", "args": {"path": "path/to/file"}}
- write_file: {"name": "write_file", "args": {"path": "path/to/file", "content": "file content here"}}
- run_command: {"name": "run_command", "args": {"command": "ls -la"}}
- search_code: {"name": "search_code", "args": {"query": "search term"}}
- git_diff: {"name": "git_diff", "args": {"staged": false}}
- git_status: {"name": "git_status", "args": {}}

After receiving tool results, continue your analysis or implementation.
When you are done, provide your final answer without any tool calls."#;

const PLAN_SYSTEM_PROMPT: &str = r#"You are in PLAN mode. Generate a step-by-step implementation plan for the user's task.

Rules:
1. Output a numbered list of concrete, actionable steps.
2. Each step should describe a specific file to create/modify and what to do.
3. Keep steps focused — one action per step.
4. After the plan, add a line: "Type /approve to execute this plan, /edit to modify, or /cancel to exit PLAN mode."

To use tools during implementation, output tool calls in this format:
```tool
{"name": "read_file", "args": {"path": "src/main.rs"}}
```

Available tools: read_file, write_file, run_command, search_code, git_diff, git_status"#;

const MAX_TOOL_ITERATIONS: usize = 5;

pub async fn execute(
    action: AppAction,
    app: &mut App,
    config: &Config,
    client: &ModelClient,
    root: &std::path::Path,
    security: &SecurityFilter,
    store: &Option<IndexStore>,
    model_tx: mpsc::Sender<anyhow::Result<String>>,
) {
    match action {
        AppAction::Send => {
            let input: String = app.input.drain(..).collect();
            app.input_cursor = 0;
            app.completions.clear();
            app.completion_active = false;

            if input.starts_with('/') {
                handle_slash_command(app, config, client, root, security, store, model_tx, &input).await;
                return;
            }

            app.push_user_message(input.clone());
            app.status = AppStatus::Thinking;

            // Build context (synchronous, fast)
            let search_query = extract_keywords(&input);
            let rg_output = Command::new("rg")
                .arg("--line-number").arg("--color=never")
                .arg("--max-count=10").arg("--context=2")
                .arg(&search_query).arg(root)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();

            let mut context = CodeContext::from_search_results(&rg_output, root, security, 10, 200)
                .unwrap_or_else(|_| CodeContext { matches: Vec::new(), file_contents: Vec::new() });

            if let Some(ref idx) = store {
                let keywords: Vec<String> = search_query.split('|').map(|s| s.to_string()).collect();
                let mut seen = std::collections::HashSet::new();
                let mut all_symbols = Vec::new();
                for kw in &keywords {
                    if let Ok(symbols) = idx.search_symbols(kw, 5) {
                        for sym in symbols {
                            if seen.insert(sym.name.clone()) {
                                all_symbols.push(sym);
                            }
                        }
                    }
                }
                if !all_symbols.is_empty() {
                    if let Ok(sym_ctx) = CodeContext::from_symbols(&all_symbols, root, security, 4000) {
                        context.merge(sym_ctx);
                    }
                }
            }

            let context_text = context.format_for_prompt(config.model.context_limit);

            let system_prompt = if app.mode == AppMode::Plan {
                plugin::resolve_prompt(config, "plan", PLAN_SYSTEM_PROMPT)
            } else {
                plugin::resolve_prompt(config, "ask", TOOL_SYSTEM_PROMPT)
            };

            let prompt = if context_text.is_empty() {
                format!("Task: {input}\n\nNo relevant code context was found in the repository.")
            } else {
                format!("Task: {input}\n\nRelevant code context:\n{context_text}")
            };

            // Auto-save session with user message
            save_session(root, app);

            // Spawn async model call
            let client = client.clone();
            let root = root.to_path_buf();
            tokio::spawn(async move {
                let result = run_tool_loop(&client, &system_prompt, &prompt, &root).await;
                let _ = model_tx.send(result).await;
            });
        }

        AppAction::Quit => {
            save_session(root, app);
            app.running = false;
        }

        AppAction::ScrollPageUp => {
            for _ in 0..10 { app.scroll_up(); }
        }
        AppAction::ScrollPageDown => {
            for _ in 0..10 { app.scroll_down(); }
        }
        AppAction::ClearChat => app.clear_chat(),
        AppAction::None => {}
    }
}

/// Save current session to disk. Public for use by mod.rs on exit.
pub fn save_session(root: &std::path::Path, app: &App) {
    if app.messages.is_empty() {
        return;
    }
    let mut sess = session::Session::new();
    sess.id = app.session_id.clone();
    sess.messages = app.messages.clone();
    sess.plan = app.plan_content.clone();
    sess.mode = format!("{:?}", app.mode).to_lowercase();
    if let Err(e) = session::save_session(root, &sess) {
        eprintln!("Warning: failed to save session: {e}");
    }
}

async fn run_tool_loop(
    client: &ModelClient,
    system_prompt: &str,
    prompt: &str,
    root: &std::path::Path,
) -> anyhow::Result<String> {
    let mut current_prompt = prompt.to_string();
    let mut current_system = system_prompt.to_string();
    let mut full_response = String::new();

    for _iteration in 0..MAX_TOOL_ITERATIONS {
        let response = client.generate(&current_system, &current_prompt).await?;
        let calls = tool::parse_tool_calls(&response);

        if calls.is_empty() {
            full_response = response;
            break;
        }

        let mut results = Vec::new();
        for call in &calls {
            results.push(tool::execute_tool(call, root));
        }

        let tool_output = tool::format_tool_results(&results);
        current_prompt = format!(
            "Previous response:\n{response}\n\nTool results:\n{tool_output}\n\nContinue. If done, provide your final answer without tool calls."
        );
        current_system = TOOL_SYSTEM_PROMPT.to_string();
        full_response = response;
    }

    Ok(full_response)
}

async fn handle_slash_command(
    app: &mut App,
    config: &Config,
    client: &ModelClient,
    root: &std::path::Path,
    _security: &SecurityFilter,
    store: &Option<IndexStore>,
    model_tx: mpsc::Sender<anyhow::Result<String>>,
    input: &str,
) {
    let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();
    let cmd = parts[0];
    let args = if parts.len() > 1 { parts[1] } else { "" };
    let args2 = if parts.len() > 2 { parts[2] } else { "" };

    match cmd {
        "/help" => {
            app.push_system_message(
                "Commands:\n\
                 /help       Show this help\n\
                 /clear      Clear chat\n\
                 /model      Show model config\n\
                 /status     Project status\n\
                 /commit     Generate commit message\n\
                 /plan       Enter PLAN mode\n\
                 /approve    Execute plan\n\
                 /cancel     Exit PLAN mode\n\
                 /read FILE  Read file\n\
                 /exec CMD   Run command\n\
                 /exit       Save & exit",
            );
        }

        "/clear" => app.clear_chat(),

        "/exit" => {
            save_session(root, app);
            app.running = false;
        }

        "/model" => {
            let info = format!(
                "Model: {} ({})\nURL: {}\nContext limit: {}\nTimeout: {}s\nSession: {}",
                config.model.model, config.model.provider, config.model.base_url,
                config.model.context_limit, config.model.timeout_seconds.unwrap_or(120),
                app.session_id,
            );
            app.push_system_message(&info);
        }

        "/status" => {
            let mut status = String::new();
            if let Ok(output) = Command::new("git").args(["status", "--short"]).current_dir(root).output() {
                let s = String::from_utf8_lossy(&output.stdout);
                if s.is_empty() {
                    status.push_str("Git: clean\n");
                } else {
                    status.push_str(&format!("Git:\n{s}\n"));
                }
            }
            let db_path = root.join(crate::config::DUMBCODER_DIR).join("index").join("symbols.db");
            if let Ok(store) = IndexStore::open(&db_path) {
                if let Ok((files, symbols)) = store.total_stats() {
                    status.push_str(&format!("Index: {files} files, {symbols} symbols"));
                }
            }
            app.push_system_message(&status);
        }

        "/commit" => {
            app.status = AppStatus::Thinking;
            save_session(root, app);
            let diff = Command::new("git").args(["diff", "--cached"]).current_dir(root).output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
            if diff.is_empty() {
                app.push_system_message("No staged changes. Use `git add` first.");
                app.status = AppStatus::Ready;
                return;
            }
            let client = client.clone();
            let truncated = if diff.len() > 5000 { diff[..5000].to_string() } else { diff };
            tokio::spawn(async move {
                let prompt = format!("Generate a concise git commit message for this diff:\n\n```diff\n{truncated}\n```\n\nOutput ONLY the commit message.");
                let result = client.generate("You are a git commit message generator. Output ONLY the commit message.", &prompt).await;
                let _ = model_tx.send(result.map(|msg| format!("Suggested commit message:\n\n{msg}"))).await;
            });
        }

        "/plan" => {
            app.mode = AppMode::Plan;
            app.plan_content = None;
            app.push_system_message("PLAN mode. Describe your task.\n/approve to execute | /cancel to exit");
        }

        "/approve" => {
            if app.mode != AppMode::Plan || app.plan_content.is_none() {
                app.push_system_message("No plan to approve. Use /plan first.");
                return;
            }
            app.mode = AppMode::Chat;
            let plan = app.plan_content.take().unwrap();
            app.push_system_message(&format!("Executing plan...\n\n{plan}"));
            app.status = AppStatus::Thinking;
            save_session(root, app);

            let client = client.clone();
            let root = root.to_path_buf();
            tokio::spawn(async move {
                let prompt = format!("Implement this plan. Use tool calls to read/write files and run commands.\n\nPlan:\n{plan}");
                let result = run_tool_loop(&client, TOOL_SYSTEM_PROMPT, &prompt, &root).await;
                let _ = model_tx.send(result).await;
            });
        }

        "/cancel" => {
            app.mode = AppMode::Chat;
            app.plan_content = None;
            app.push_system_message("Exited PLAN mode.");
        }

        "/read" => {
            let path_str = if args2.is_empty() { args } else { &format!("{args} {args2}") };
            if path_str.is_empty() {
                app.push_system_message("Usage: /read <file>");
                return;
            }
            let full_path = if std::path::Path::new(path_str).is_absolute() {
                std::path::PathBuf::from(path_str)
            } else {
                root.join(path_str)
            };
            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    let lines = content.lines().count();
                    let truncated = if content.len() > 8000 { format!("{}...", &content[..8000]) } else { content };
                    app.push_system_message(&format!("{} ({} lines)\n\n{truncated}", full_path.display(), lines));
                }
                Err(e) => app.push_system_message(&format!("Error: {e}")),
            }
        }

        "/exec" => {
            let cmd_str = if args2.is_empty() { args.to_string() } else { format!("{args} {args2}") };
            if cmd_str.is_empty() {
                app.push_system_message("Usage: /exec <command>");
                return;
            }
            match Command::new("sh").arg("-c").arg(&cmd_str).current_dir(root).output() {
                Ok(output) => {
                    let mut result = format!("$ {cmd_str}\n");
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stdout.is_empty() { result.push_str(&stdout); }
                    if !stderr.is_empty() { result.push_str(&format!("stderr: {stderr}")); }
                    app.push_system_message(&result);
                }
                Err(e) => app.push_system_message(&format!("Error: {e}")),
            }
        }

        _ => {
            app.push_system_message(&format!("Unknown: {cmd}. /help for commands."));
        }
    }
}

fn extract_keywords(question: &str) -> String {
    let stop_words: std::collections::HashSet<&str> = [
        "what", "where", "how", "is", "the", "a", "an", "in", "of", "for",
        "and", "or", "to", "do", "does", "did", "can", "could", "would",
        "should", "will", "are", "was", "were", "been", "be", "have", "has",
        "had", "that", "this", "it", "its", "my", "your", "our", "their",
        "i", "we", "you", "they", "he", "she", "not", "no", "if", "but",
        "with", "from", "by", "on", "at", "as", "so", "than", "very",
        "user", "code", "file", "data", "use", "used", "using", "get", "set",
    ].iter().copied().collect();

    let keywords: Vec<&str> = question.split_whitespace().filter(|w| {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        !stop_words.contains(clean.as_str()) && clean.len() > 1
    }).collect();

    if keywords.is_empty() { question.to_string() } else { keywords.join("|") }
}
