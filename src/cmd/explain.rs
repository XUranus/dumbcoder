use anyhow::{bail, Result};
use std::path::Path;

use crate::config::{Config, DUMBCODER_DIR};
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::util;

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI coding assistant. You explain code clearly and concisely.
When explaining:
1. Describe the purpose and responsibility of the code.
2. Summarize inputs, outputs, and key logic.
3. Note any edge cases, potential issues, or risks.
4. Use clear, structured formatting."#;

pub async fn run(path: &str, symbol: Option<&str>) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    let file_path = Path::new(path);
    if !security.is_path_allowed(file_path, &root) {
        bail!("Access to {path} is not allowed by security policy");
    }

    let full_path = if file_path.is_relative() {
        root.join(file_path)
    } else {
        file_path.to_path_buf()
    };

    if !full_path.exists() {
        bail!("File not found: {}", full_path.display());
    }

    let content = std::fs::read_to_string(&full_path)?;

    util::header("Explaining");
    util::info("file", &full_path.display().to_string());
    if let Some(sym) = symbol {
        util::info("symbol", sym);
    }

    let code_to_explain = if let Some(sym) = symbol {
        // Try index first for precise extraction
        let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
        let rel_path = full_path
            .strip_prefix(&root)
            .unwrap_or(&full_path)
            .to_string_lossy()
            .to_string();

        if let Ok(store) = IndexStore::open(&db_path) {
            if let Ok(Some(symbol_info)) = store.get_symbol(&rel_path, sym) {
                let lines: Vec<&str> = content.lines().collect();
                let start = if symbol_info.start_line > 0 {
                    symbol_info.start_line - 1
                } else {
                    0
                };
                let end = std::cmp::min(symbol_info.end_line, lines.len());
                if start < lines.len() {
                    lines[start..end].join("\n")
                } else {
                    extract_symbol_code(&content, sym)
                }
            } else {
                extract_symbol_code(&content, sym)
            }
        } else {
            extract_symbol_code(&content, sym)
        }
    } else {
        // If file is too large, take first 300 lines
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 300 {
            lines[..300].join("\n")
        } else {
            content.clone()
        }
    };

    util::header("Asking model");
    eprintln!("  Model: {} ({}) @ {}", config.model.model, config.model.provider, config.model.base_url);

    let client = ModelClient::new(&config.model)?;
    let user_prompt = format!(
        "Explain the following code from {}:\n\n```\n{}\n```",
        path, code_to_explain
    );

    let system_prompt = plugin::resolve_prompt(&config, "explain", DEFAULT_SYSTEM_PROMPT);
    let explanation = client.generate(&system_prompt, &user_prompt).await?;

    util::header("Explanation");
    println!("{explanation}");

    Ok(())
}

/// Try to extract code for a specific symbol (function, method, struct).
fn extract_symbol_code(content: &str, symbol: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();
    let mut capturing = false;
    let mut brace_depth = 0;

    for line in &lines {
        if line.contains(symbol) && (line.contains("fn ")
            || line.contains("func ")
            || line.contains("def ")
            || line.contains("class ")
            || line.contains("function ")
            || line.contains("pub ")
            || line.contains("async ")
            || line.contains("let ")
            || line.contains("const "))
        {
            capturing = true;
        }

        if capturing {
            result_lines.push(*line);
            brace_depth += line.matches('{').count();
            brace_depth = brace_depth.saturating_sub(line.matches('}').count());

            if brace_depth == 0 && !result_lines.is_empty() {
                break;
            }
        }
    }

    if result_lines.is_empty() {
        // Fallback: search for lines containing the symbol
        let matching: Vec<&str> = lines
            .iter()
            .filter(|l| l.contains(symbol))
            .copied()
            .take(50)
            .collect();
        matching.join("\n")
    } else {
        result_lines.join("\n")
    }
}
