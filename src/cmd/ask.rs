use anyhow::Result;
use std::process::Command;

use crate::config::{Config, DUMBCODER_DIR};
use crate::context::CodeContext;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::security::SecurityFilter;
use crate::util;

const SYSTEM_PROMPT: &str = r#"You are a helpful AI coding assistant. You answer questions about a codebase.
When answering:
1. Reference specific files and line numbers when possible.
2. Be concise and direct.
3. If you are unsure, say so.
4. Focus on the code provided as context."#;

pub async fn run(question: &str) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    // Step 1: Search codebase for relevant context
    util::header("Searching codebase");

    // Extract keywords from question for rg search
    let search_query = extract_keywords(question);
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

    // Step 2: Build context from rg search
    let mut context = CodeContext::from_search_results(&rg_output, &root, &security, 10, 200)?;

    // Step 2b: Query the index for matching symbols
    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    if db_path.exists() {
        if let Ok(store) = IndexStore::open(&db_path) {
            if let Ok(symbols) = store.search_symbols(&search_query, 10) {
                if !symbols.is_empty() {
                    if let Ok(symbol_ctx) = CodeContext::from_symbols(&symbols, &root, &security, 4000) {
                        context.merge(symbol_ctx);
                    }
                }
            }
        }
    }

    let context_text = context.format_for_prompt(8000);

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

    let answer = client.generate(SYSTEM_PROMPT, &user_prompt).await?;

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

fn extract_keywords(question: &str) -> String {
    // Simple keyword extraction: remove common words, keep meaningful terms
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
            let clean = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
            !stop_words.contains(clean.as_str()) && clean.len() > 1
        })
        .collect();

    if words.is_empty() {
        question.to_string()
    } else {
        words.join(" ")
    }
}
