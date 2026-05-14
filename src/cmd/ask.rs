use anyhow::Result;
use std::process::Command;

use crate::config::{Config, DUMBCODER_DIR};
use crate::context::CodeContext;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::util;

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI coding assistant. You answer questions about a codebase.
IMPORTANT: You MUST use the "Relevant code context" provided below to answer the question. Do NOT give generic answers.
When answering:
1. Reference specific file paths and line numbers from the context.
2. Describe what the code actually does, based on the provided snippets.
3. Be concise and direct.
4. If the context does not contain enough information, say what you found and what is missing."#;

pub async fn run(question: &str) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    // Step 1: Search codebase for relevant context
    util::header("Searching codebase");

    // Extract keywords from question for rg search
    let keywords = extract_keywords_vec(question);
    let search_query = keywords.join("|");
    eprintln!("  Searching for: {search_query}");

    let rg_output = Command::new("rg")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count=10")
        .arg("--context=2")
        .arg("--engine=auto")
        .arg(&search_query)
        .arg(&root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Step 2: Build context from rg search
    let mut context = CodeContext::from_search_results(&rg_output, &root, &security, 10, 200)?;

    // Step 2b: Query the index for matching symbols (per-keyword for better recall)
    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    if db_path.exists() {
        if let Ok(store) = IndexStore::open(&db_path) {
            let mut all_symbols = Vec::new();
            let mut seen_names = std::collections::HashSet::new();
            for kw in &keywords {
                if let Ok(symbols) = store.search_symbols(kw, 5) {
                    for sym in symbols {
                        if seen_names.insert(sym.name.clone()) {
                            all_symbols.push(sym);
                        }
                    }
                }
            }
            if !all_symbols.is_empty() {
                if let Ok(symbol_ctx) = CodeContext::from_symbols(&all_symbols, &root, &security, 4000) {
                    context.merge(symbol_ctx);
                }
            }
        }
    }

    let context_text = context.format_for_prompt(config.model.context_limit);

    util::header("Asking model");
    eprintln!("  Model: {} ({}) @ {}", config.model.model, config.model.provider, config.model.base_url);

    // Step 3: Call model
    let client = ModelClient::new(&config.model)?;
    let user_prompt = if context_text.is_empty() {
        format!("Question: {question}\n\nNo relevant code context was found in the repository.")
    } else {
        format!(
            "Question: {question}\n\nRelevant code context:\n{context_text}"
        )
    };

    let system_prompt = plugin::resolve_prompt(&config, "ask", DEFAULT_SYSTEM_PROMPT);
    let answer = client.generate(&system_prompt, &user_prompt).await?;

    // Step 4: Display answer
    util::header("Answer");
    println!("{answer}");

    if !context.matches.is_empty() {
        util::header("References");
        let mut seen = std::collections::HashSet::new();
        for m in &context.matches {
            if seen.insert(&m.file) {
                util::info("file", &m.file);
            }
        }
    }

    Ok(())
}

fn extract_keywords_vec(text: &str) -> Vec<String> {
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

    text.split_whitespace()
        .filter_map(|w| {
            let clean = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if !stop_words.contains(clean.as_str()) && clean.len() > 1 {
                Some(clean)
            } else {
                None
            }
        })
        .collect()
}
