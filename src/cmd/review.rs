use anyhow::{bail, Result};
use std::process::Command;

use crate::config::{Config, DUMBCODER_DIR};
use crate::context::CodeContext;
use crate::git;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::util;

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a senior code reviewer. Analyze the following git diff and provide a structured review.

For each changed file, provide:
1. **Risk level**: Low / Medium / High
2. **Issues found**: potential bugs, missing edge cases, logic errors, security concerns
3. **Suggestions**: improvements, test coverage recommendations

Format your review as:

## Review Summary

### <file path>
- **Risk**: <Low/Medium/High>
- **Issues**: <list of issues>
- **Suggestions**: <list of suggestions>

Be specific with line numbers when possible. Focus on real issues, not style preferences."#;

pub async fn run(staged: bool, diff: Option<&str>) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    util::header("Reviewing changes");

    // Get the diff
    let (diff_content, diff_label) = if staged {
        eprintln!("  Mode: staged changes");
        let d = git::get_staged_diff(&root)?;
        (d, "staged changes".to_string())
    } else if let Some(range) = diff {
        eprintln!("  Mode: diff range: {range}");
        let d = git::get_diff_range(&root, range)?;
        (d, format!("diff {range}"))
    } else {
        eprintln!("  Mode: unstaged changes");
        let d = git::get_unstaged_diff(&root)?;
        (d, "unstaged changes".to_string())
    };

    if diff_content.trim().is_empty() {
        println!("  No changes found ({diff_label}).");
        return Ok(());
    }

    // Parse changed files
    let changed_files = git::parse_changed_files(&diff_content);
    util::info("changed files", &changed_files.len().to_string());
    for f in &changed_files {
        eprintln!("    - {f}");
    }

    // Get context for changed files from index
    let mut context_text = String::new();
    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    if db_path.exists() {
        if let Ok(store) = IndexStore::open(&db_path) {
            for file in &changed_files {
                if let Ok(symbols) = store.get_file_symbols(file) {
                    if let Ok(ctx) =
                        CodeContext::from_symbols(&symbols, &root, &security, 3000)
                    {
                        context_text.push_str(&ctx.format_for_prompt(3000));
                    }
                }
            }
        }
    }

    util::header("Asking model");
    eprintln!("  Model: {} ({}) @ {}", config.model.model, config.model.provider, config.model.base_url);

    let mut user_prompt = format!(
        "Review the following git diff ({diff_label}):\n\n```diff\n{}\n```",
        &diff_content[..std::cmp::min(diff_content.len(), 15000)]
    );

    if !context_text.is_empty() {
        user_prompt.push_str(&format!(
            "\n\nAdditional context (symbols in changed files):\n{context_text}"
        ));
    }

    let client = ModelClient::new(&config.model)?;
    let system_prompt = plugin::resolve_prompt(&config, "review", DEFAULT_SYSTEM_PROMPT);
    let review = client.generate(&system_prompt, &user_prompt).await?;

    util::header("Review");
    println!("{review}");

    Ok(())
}
