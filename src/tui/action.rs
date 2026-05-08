use std::process::Command;

use crate::context::CodeContext;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::security::SecurityFilter;

use super::app::{App, AppAction, AppStatus, ModelResponse, Panel};

const SYSTEM_PROMPT: &str = r#"You are a helpful AI coding assistant. You answer questions about a codebase.
When answering:
1. Reference specific files and line numbers when possible.
2. Be concise and direct.
3. If you are unsure, say so.
4. Focus on the code provided as context."#;

pub async fn execute(
    action: AppAction,
    app: &mut App,
    client: &ModelClient,
    root: &std::path::Path,
    security: &SecurityFilter,
    store: &Option<IndexStore>,
) {
    match action {
        AppAction::Send => {
            let question = app.input.drain(..).collect::<String>();
            app.input_cursor = 0;
            app.push_user_message(question.clone());
            app.status = AppStatus::Thinking;

            // Build context synchronously (rg + index)
            let search_query = extract_keywords(&question);

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
                if let Ok(symbols) = idx.search_symbols(&search_query, 10) {
                    if let Ok(sym_ctx) =
                        CodeContext::from_symbols(&symbols, root, security, 4000)
                    {
                        context.merge(sym_ctx);
                    }
                }
            }

            let context_text = context.format_for_prompt(8000);
            let context_files = context.file_contents.clone();
            let mut context_symbols = Vec::new();

            if let Some(ref idx) = store {
                if let Ok(symbols) = idx.search_symbols(&search_query, 10) {
                    context_symbols = symbols;
                }
            }

            // Spawn model call in background
            let client = client.clone();
            let prompt = if context_text.is_empty() {
                format!("Question: {question}\n\nNo relevant code context was found in the repository.")
            } else {
                format!("Question: {question}\n\nRelevant code context:\n{context_text}")
            };

            // We need to handle the model call. Since we can't easily spawn async
            // with cross-thread communication in this simple setup, we'll do it inline
            // but the TUI will show "Thinking" status.
            let result = client.generate(SYSTEM_PROMPT, &prompt).await;
            let resp = result.map(|answer| ModelResponse {
                answer,
                context_files,
                context_symbols,
            });
            app.receive_model_response(resp);
        }
        AppAction::Quit => {
            app.running = false;
        }
        AppAction::ScrollUp => {
            app.scroll_context_up();
        }
        AppAction::ScrollDown => {
            app.scroll_context_down();
        }
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
        AppAction::ClearChat => {
            app.clear_chat();
        }
        AppAction::None => {}
    }
}

fn extract_keywords(question: &str) -> String {
    let stop_words: std::collections::HashSet<&str> = [
        "what", "where", "how", "is", "the", "a", "an", "in", "of", "for",
        "and", "or", "to", "do", "does", "did", "can", "could", "would",
        "should", "will", "are", "was", "were", "been", "be", "have", "has",
        "had", "that", "this", "it", "its", "my", "your", "our", "their",
        "i", "we", "you", "they", "he", "she", "吗", "呢", "什么", "哪里",
        "怎么", "如何", "的", "了", "在", "是", "有", "和", "与",
    ]
    .iter()
    .copied()
    .collect();

    let words: Vec<&str> = question
        .split_whitespace()
        .filter(|w| {
            let clean = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            !stop_words.contains(clean.as_str()) && clean.len() > 1
        })
        .collect();

    if words.is_empty() {
        question.to_string()
    } else {
        words.join(" ")
    }
}
