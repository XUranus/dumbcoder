use std::process::Command;

use crate::config::Config;
use crate::context::CodeContext;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::session;
use crate::tool;

use super::app::{App, AppAction, AppMode, AppStatus, ModelResponse, Panel};

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a code assistant. Answer using ONLY the code snippets in the "Relevant code context" section below.
RULES:
- ONLY mention files and functions that appear in the provided context. Do NOT invent or hallucinate file names, function names, or line numbers.
- Quote or reference the actual code from the context when explaining.
- If the context contains relevant code, describe it in detail with file paths from the context.
- If the context truly does not contain relevant code, say "The provided context does not contain relevant code"."#;

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

const MAX_TOOL_ITERATIONS: usize = 5;

pub async fn execute(
    action: AppAction,
    app: &mut App,
    config: &Config,
    client: &ModelClient,
    root: &std::path::Path,
    security: &SecurityFilter,
    store: &Option<IndexStore>,
) {
    match action {
        AppAction::Send => {
            let input: String = app.input.drain(..).collect();
            app.input_cursor = 0;

            // Check for slash commands
            if input.starts_with('/') {
                handle_slash_command(app, config, client, root, security, store, &input).await;
                return;
            }

            app.push_user_message(input.clone());
            app.status = AppStatus::Thinking;

            // Use tool-enabled prompt
            let system_prompt = if app.mode == AppMode::Plan {
                plugin::resolve_prompt(config, "plan", PLAN_SYSTEM_PROMPT)
            } else {
                plugin::resolve_prompt(config, "ask", TOOL_SYSTEM_PROMPT)
            };

            // Build context
            let search_query = extract_keywords(&input);
            let rg_output = Command::new("rg")
                .arg("--line-number")
                .arg("--color=never")
                .arg("--max-count=10")
                .arg("--context=2")
                .arg(&search_query)
                .arg(root)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();

            let mut context =
                CodeContext::from_search_results(&rg_output, root, security, 10, 200)
                    .unwrap_or_else(|_| CodeContext {
                        matches: Vec::new(),
                        file_contents: Vec::new(),
                    });

            if let Some(ref idx) = store {
                let keywords: Vec<String> = search_query
                    .split('|')
                    .map(|s| s.to_string())
                    .collect();
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
                    if let Ok(sym_ctx) =
                        CodeContext::from_symbols(&all_symbols, root, security, 4000)
                    {
                        context.merge(sym_ctx);
                    }
                }
            }

            let context_text = context.format_for_prompt(config.model.context_limit);
            let context_files = context.file_contents.clone();
            let mut context_symbols = Vec::new();
            if let Some(ref idx) = store {
                if let Ok(symbols) = idx.search_symbols(&search_query, 10) {
                    context_symbols = symbols;
                }
            }

            let client = client.clone();
            let prompt = if context_text.is_empty() {
                format!("Task: {input}\n\nNo relevant code context was found in the repository.")
            } else {
                format!("Task: {input}\n\nRelevant code context:\n{context_text}")
            };

            // Tool-call loop
            let mut full_response = String::new();
            let mut current_prompt = prompt.clone();
            let mut current_system = system_prompt.clone();

            for iteration in 0..MAX_TOOL_ITERATIONS {
                let result = client.generate(&current_system, &current_prompt).await;

                match result {
                    Ok(response) => {
                        let calls = tool::parse_tool_calls(&response);

                        if calls.is_empty() {
                            // No tool calls — this is the final response
                            full_response = response;
                            break;
                        }

                        // Execute tool calls
                        app.push_system_message(&format!(
                            "[Tool call] Executing {} tool(s)...",
                            calls.len()
                        ));

                        let mut results = Vec::new();
                        for call in &calls {
                            let result = tool::execute_tool(call, root);
                            results.push(result);
                        }

                        let tool_output = tool::format_tool_results(&results);
                        app.push_system_message(&tool_output);

                        // Feed results back to model
                        current_prompt = format!(
                            "Previous response:\n{response}\n\nTool results:\n{tool_output}\n\nContinue. If done, provide your final answer without tool calls."
                        );
                        current_system = TOOL_SYSTEM_PROMPT.to_string();

                        if iteration == MAX_TOOL_ITERATIONS - 1 {
                            full_response = format!(
                                "{response}\n\n(Max tool iterations reached. Tool results:\n{tool_output})"
                            );
                        }
                    }
                    Err(e) => {
                        app.receive_model_response(Err(e));
                        return;
                    }
                }
            }

            if app.mode == AppMode::Plan {
                app.plan_content = Some(full_response.clone());
            }

            app.receive_model_response(Ok(ModelResponse {
                answer: full_response,
                context_files,
                context_symbols,
            }));
        }

        AppAction::Quit => {
            // Auto-save session on quit
            if !app.messages.is_empty() {
                let mut session = session::Session::new();
                if let Some(ref id) = app.session_id {
                    session.id = id.clone();
                }
                session.messages = app.messages.clone();
                session.plan = app.plan_content.clone();
                session.mode = format!("{:?}", app.mode).to_lowercase();
                if let Err(e) = session::save_session(root, &session) {
                    eprintln!("Warning: failed to save session: {e}");
                }
            }
            app.running = false;
        }

        AppAction::ScrollUp => app.scroll_context_up(),
        AppAction::ScrollDown => app.scroll_context_down(),
        AppAction::ScrollPageUp => {
            for _ in 0..10 {
                app.scroll_context_up();
            }
        }
        AppAction::ScrollPageDown => {
            for _ in 0..10 {
                app.scroll_context_down();
            }
        }
        AppAction::SwitchPanel => {
            app.active_panel = match app.active_panel {
                Panel::Chat => Panel::Context,
                Panel::Context => Panel::Chat,
            };
        }
        AppAction::ClearChat => app.clear_chat(),
        AppAction::None => {}
    }
}

async fn handle_slash_command(
    app: &mut App,
    config: &Config,
    client: &ModelClient,
    root: &std::path::Path,
    _security: &SecurityFilter,
    store: &Option<IndexStore>,
    input: &str,
) {
    let parts: Vec<&str> = input.trim().splitn(3, ' ').collect();
    let cmd = parts[0];
    let args = if parts.len() > 1 { parts[1] } else { "" };
    let args2 = if parts.len() > 2 { parts[2] } else { "" };

    match cmd {
        "/help" => {
            app.push_system_message(
                "Available commands:\n\
                 /help          Show this help\n\
                 /clear         Clear chat history\n\
                 /model         Show current model config\n\
                 /status        Show project status\n\
                 /commit        Generate commit message from staged diff\n\
                 /plan          Enter PLAN mode\n\
                 /approve       Execute current plan\n\
                 /cancel        Exit PLAN mode\n\
                 /session save [name]  Save current session\n\
                 /session load [name]  Load a saved session\n\
                 /session list  List saved sessions\n\
                 /read <file>   Read file into context\n\
                 /exec <cmd>    Run a shell command",
            );
        }

        "/clear" => {
            app.clear_chat();
        }

        "/model" => {
            let info = format!(
                "Model: {} ({})\nURL: {}\nContext limit: {}\nTimeout: {}s",
                config.model.model,
                config.model.provider,
                config.model.base_url,
                config.model.context_limit,
                config.model.timeout_seconds.unwrap_or(120),
            );
            app.push_system_message(&info);
        }

        "/status" => {
            let mut status = String::new();

            // Git status
            if let Ok(output) = Command::new("git")
                .args(["status", "--short"])
                .current_dir(root)
                .output()
            {
                let s = String::from_utf8_lossy(&output.stdout);
                status.push_str(&format!("Git status:\n{s}\n"));
            }

            // Index stats
            let db_path = root
                .join(crate::config::DUMBCODER_DIR)
                .join("index")
                .join("symbols.db");
            if let Ok(store) = IndexStore::open(&db_path) {
                if let Ok((files, symbols)) = store.total_stats() {
                    status.push_str(&format!("Index: {files} files, {symbols} symbols\n"));
                }
            }

            app.push_system_message(&status);
        }

        "/commit" => {
            app.status = AppStatus::Thinking;
            // Get staged diff
            let diff = Command::new("git")
                .args(["diff", "--cached"])
                .current_dir(root)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();

            if diff.is_empty() {
                app.push_system_message("No staged changes. Use `git add` to stage files first.");
                app.status = AppStatus::Ready;
                return;
            }

            let prompt = format!(
                "Generate a concise git commit message for the following staged diff:\n\n```diff\n{}\n```\n\nOutput ONLY the commit message, nothing else.",
                &diff[..std::cmp::min(diff.len(), 5000)]
            );

            let system = "You are a git commit message generator. Output ONLY the commit message, no explanations.";
            match client.generate(system, &prompt).await {
                Ok(msg) => {
                    app.push_system_message(&format!("Suggested commit message:\n\n{msg}\n\nRun: git commit -m \"{msg}\""));
                }
                Err(e) => {
                    app.last_error = Some(e.to_string());
                }
            }
            app.status = AppStatus::Ready;
        }

        "/plan" => {
            app.mode = AppMode::Plan;
            app.plan_content = None;
            app.push_system_message(
                "Entered PLAN mode. Describe your task and I'll generate a step-by-step plan.\n\
                 Commands: /approve (execute plan), /cancel (exit PLAN mode)",
            );
        }

        "/approve" => {
            if app.mode != AppMode::Plan || app.plan_content.is_none() {
                app.push_system_message("No plan to approve. Use /plan first.");
                return;
            }
            app.mode = AppMode::Chat;
            let plan = app.plan_content.take().unwrap();
            app.push_system_message(&format!("Executing plan...\n\n{plan}"));

            // Ask model to implement the plan
            app.status = AppStatus::Thinking;
            let prompt = format!(
                "Implement the following plan step by step. Use tool calls to read/write files and run commands.\n\nPlan:\n{plan}"
            );
            let result = client.generate(TOOL_SYSTEM_PROMPT, &prompt).await;
            app.receive_model_response(result.map(|answer| ModelResponse {
                answer,
                context_files: Vec::new(),
                context_symbols: Vec::new(),
            }));
        }

        "/cancel" => {
            app.mode = AppMode::Chat;
            app.plan_content = None;
            app.push_system_message("Exited PLAN mode.");
        }

        "/session" => {
            match args {
                "save" => {
                    let name = if args2.is_empty() { None } else { Some(args2.to_string()) };
                    let mut sess = session::Session::new();
                    if let Some(ref id) = app.session_id {
                        sess.id = id.clone();
                    }
                    sess.name = name;
                    sess.messages = app.messages.clone();
                    sess.plan = app.plan_content.clone();
                    sess.mode = format!("{:?}", app.mode).to_lowercase();

                    match session::save_session(root, &sess) {
                        Ok(()) => {
                            app.session_id = Some(sess.id.clone());
                            app.push_system_message(&format!("Session saved: {}", sess.id));
                        }
                        Err(e) => {
                            app.push_system_message(&format!("Failed to save session: {e}"));
                        }
                    }
                }
                "load" => {
                    if args2.is_empty() {
                        app.push_system_message("Usage: /session load <id_or_name>");
                        return;
                    }
                    match session::load_session(root, args2) {
                        Ok(sess) => {
                            app.messages = sess.messages.clone();
                            app.plan_content = sess.plan.clone();
                            app.session_id = Some(sess.id.clone());
                            app.mode = if sess.mode == "plan" {
                                AppMode::Plan
                            } else {
                                AppMode::Chat
                            };
                            app.scroll_chat = app.messages.len() as u16 * 3;
                            app.push_system_message(&format!(
                                "Session loaded: {} ({} messages)",
                                sess.id,
                                app.messages.len()
                            ));
                        }
                        Err(e) => {
                            app.push_system_message(&format!("Failed to load session: {e}"));
                        }
                    }
                }
                "list" => {
                    match session::list_sessions(root) {
                        Ok(sessions) if sessions.is_empty() => {
                            app.push_system_message("No saved sessions.");
                        }
                        Ok(sessions) => {
                            let mut msg = String::from("Saved sessions:\n");
                            for s in &sessions {
                                msg.push_str(&format!(
                                    "  {} ({}) — {} messages, mode: {}\n",
                                    s.id, s.created_at, s.message_count, s.mode
                                ));
                            }
                            app.push_system_message(&msg);
                        }
                        Err(e) => {
                            app.push_system_message(&format!("Error: {e}"));
                        }
                    }
                }
                _ => {
                    app.push_system_message("Usage: /session save|load|list [name]");
                }
            }
        }

        "/read" => {
            if args.is_empty() {
                app.push_system_message("Usage: /read <file_path>");
                return;
            }
            let path = if args2.is_empty() { args } else { &format!("{args} {args2}") };
            let full_path = if std::path::Path::new(path).is_absolute() {
                std::path::PathBuf::from(path)
            } else {
                root.join(path)
            };
            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    let lines = content.lines().count();
                    app.push_system_message(&format!(
                        "File: {} ({} lines)\n\n{}",
                        full_path.display(),
                        lines,
                        if content.len() > 5000 {
                            format!("{}...(truncated)", &content[..5000])
                        } else {
                            content
                        }
                    ));
                }
                Err(e) => {
                    app.push_system_message(&format!("Failed to read {}: {e}", full_path.display()));
                }
            }
        }

        "/exec" => {
            let cmd_str = if args2.is_empty() {
                args.to_string()
            } else {
                format!("{args} {args2}")
            };
            if cmd_str.is_empty() {
                app.push_system_message("Usage: /exec <command>");
                return;
            }
            match Command::new("sh").arg("-c").arg(&cmd_str).current_dir(root).output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = format!("$ {cmd_str}\n");
                    if !stdout.is_empty() {
                        result.push_str(&stdout);
                    }
                    if !stderr.is_empty() {
                        result.push_str(&format!("stderr: {stderr}"));
                    }
                    app.push_system_message(&result);
                }
                Err(e) => {
                    app.push_system_message(&format!("Command failed: {e}"));
                }
            }
        }

        _ => {
            app.push_system_message(&format!(
                "Unknown command: {cmd}. Type /help for available commands."
            ));
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
    ]
    .iter()
    .copied()
    .collect();

    let keywords: Vec<&str> = question
        .split_whitespace()
        .filter(|w| {
            let clean = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            !stop_words.contains(clean.as_str()) && clean.len() > 1
        })
        .collect();

    if keywords.is_empty() {
        question.to_string()
    } else {
        keywords.join("|")
    }
}
