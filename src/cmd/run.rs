use anyhow::{bail, Result};
use std::process::Command;

use crate::config::{Config, DUMBCODER_DIR};
use crate::context::CodeContext;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::util;

pub async fn run(name: &str, query: &str) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    // Load plugins
    let plugins = plugin::load_plugins(&root);
    let plugin = plugins
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| {
            let available: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
            if available.is_empty() {
                anyhow::anyhow!(
                    "Plugin '{name}' not found. No plugins in .dumbcoder/plugins/. \
                     Create a .toml file there to define a plugin."
                )
            } else {
                anyhow::anyhow!(
                    "Plugin '{name}' not found. Available plugins: {}",
                    available.join(", ")
                )
            }
        })?;

    util::header(&format!("Plugin: {}", plugin.name));
    util::info("description", &plugin.description);

    // Search codebase
    util::header("Searching codebase");
    let search_query = extract_keywords(query);
    eprintln!("  Searching for: {search_query}");

    let rg_output = Command::new("rg")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count=10")
        .arg("--context=2")
        .arg(&search_query)
        .arg(&root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Build context
    let mut context = CodeContext::from_search_results(&rg_output, &root, &security, 10, 200)?;

    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    if db_path.exists() {
        if let Ok(store) = IndexStore::open(&db_path) {
            if let Ok(symbols) = store.search_symbols(&search_query, 10) {
                if !symbols.is_empty() {
                    if let Ok(sym_ctx) =
                        CodeContext::from_symbols(&symbols, &root, &security, 4000)
                    {
                        context.merge(sym_ctx);
                    }
                }
            }
        }
    }

    let context_text = context.format_for_prompt(8000);

    // Call model
    util::header("Running plugin");
    eprintln!(
        "  Model: {} ({}) @ {}",
        config.model.model, config.model.provider, config.model.base_url
    );

    let client = ModelClient::new(&config.model)?;
    let user_prompt = if context_text.is_empty() {
        format!("Task: {query}\n\nNo relevant code context was found in the repository.")
    } else {
        format!("Task: {query}\n\nRelevant code context:\n{context_text}")
    };

    let answer = client
        .generate(&plugin.system_prompt, &user_prompt)
        .await?;

    util::header("Result");
    println!("{answer}");

    Ok(())
}

fn extract_keywords(text: &str) -> String {
    let stop_words: std::collections::HashSet<&str> = [
        "fix", "the", "a", "an", "in", "of", "for", "and", "or", "to",
        "when", "is", "it", "this", "that", "should", "would", "could",
        "be", "been", "have", "has", "had", "do", "does", "did",
        "what", "where", "how", "my", "your", "our", "their",
        "修复", "的", "了", "在", "是", "有", "和", "与", "当",
    ]
    .iter()
    .copied()
    .collect();

    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|w| {
            let clean = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            !stop_words.contains(clean.as_str()) && clean.len() > 1
        })
        .collect();

    if words.is_empty() {
        text.to_string()
    } else {
        words.join(" ")
    }
}
