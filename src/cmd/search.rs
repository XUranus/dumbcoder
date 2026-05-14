use anyhow::Result;
use colored::Colorize;
use std::process::Command;

use crate::config::{Config, DUMBCODER_DIR};
use crate::index::IndexStore;
use crate::security::SecurityFilter;

pub fn run(query: &str, lang: Option<&str>) -> Result<()> {
    let (root, _is_project) = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    // Use keyword alternation for rg search
    let keywords = extract_keywords_vec(query);
    let search_pattern = keywords.join("|");

    let mut cmd = Command::new("rg");
    cmd.arg("--line-number")
        .arg("--color=never")
        .arg("--max-count=5");

    if let Some(l) = lang {
        cmd.arg("--type").arg(l);
    }

    cmd.arg(&search_pattern).arg(&root);

    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut count = 0;

    for line in stdout.lines() {
        if let Some((file_part, rest)) = line.split_once(':') {
            let file_path = std::path::Path::new(file_part);
            if !security.is_path_allowed(file_path, &root) {
                continue;
            }
            if let Some((line_num, content)) = rest.split_once(':') {
                count += 1;
                println!(
                    "  {}:{}: {}",
                    file_part.blue(),
                    line_num.yellow(),
                    content
                );
            }
        }
    }

    // Also search the index
    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    if db_path.exists() {
        if let Ok(store) = IndexStore::open(&db_path) {
            let mut found_index = false;
            for kw in &keywords {
                if let Ok(symbols) = store.search_symbols(kw, 5) {
                    for sym in &symbols {
                        if !found_index {
                            eprintln!("\n  Index symbols:");
                            found_index = true;
                        }
                        count += 1;
                        println!(
                            "  {}:{} [{}] {}",
                            sym.path.blue(),
                            sym.start_line.to_string().yellow(),
                            sym.kind.as_str().green(),
                            sym.name
                        );
                    }
                }
            }
        }
    }

    if count == 0 {
        println!("  No results found for: {query}");
    } else {
        eprintln!("\n  {} result(s) found", count);
    }

    Ok(())
}

fn extract_keywords_vec(text: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "what", "where", "how", "is", "the", "a", "an", "in", "of", "for",
        "and", "or", "to", "do", "does", "did", "can", "could", "would",
        "should", "will", "are", "was", "were", "been", "be", "have", "has",
        "had", "that", "this", "it", "its", "my", "your", "our", "their",
        "i", "we", "you", "they", "he", "she",
    ]
    .iter()
    .copied()
    .collect();

    let keywords: Vec<String> = text
        .split_whitespace()
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
        .collect();

    if keywords.is_empty() {
        vec![text.to_string()]
    } else {
        keywords
    }
}
