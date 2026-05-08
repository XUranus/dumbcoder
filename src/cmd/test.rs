use anyhow::{bail, Result};
use std::path::Path;

use crate::config::{Config, DUMBCODER_DIR};
use crate::git;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::security::SecurityFilter;
use crate::util;

const SYSTEM_PROMPT: &str = r#"You are a unit test generator. Given source code, generate comprehensive unit tests.
Rules:
1. Cover normal cases, edge cases, and error cases.
2. Use the project's test framework and conventions.
3. Output ONLY valid test code, no explanations.
4. Use descriptive test function names.
5. Follow the language's testing conventions (#[test] for Rust, func Test for Go, def test_ for Python, etc.).
6. Include necessary imports and setup code.
7. Test the function's public behavior, not implementation details."#;

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

    util::header("Generating tests");
    util::info("file", &full_path.display().to_string());
    if let Some(sym) = symbol {
        util::info("symbol", sym);
    }

    // Extract target code
    let target_code = if let Some(sym) = symbol {
        extract_symbol_for_test(&root, &full_path, sym, &content)
    } else {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 300 {
            lines[..300].join("\n")
        } else {
            content.clone()
        }
    };

    // Detect test framework
    let test_commands = git::detect_test_command(&root);
    let test_framework = test_commands.first().unwrap_or(&"unknown".into()).clone();

    // Find existing test file for conventions
    let existing_test = find_existing_test_file(&root, &full_path, &security);

    util::header("Asking model");
    eprintln!("  Model: {} @ {}", config.model.model, config.model.base_url);
    eprintln!("  Test framework: {test_framework}");

    let mut user_prompt = format!(
        "Generate unit tests for the following code.\n\nTest framework: {test_framework}\n\nTarget code:\n```\n{target_code}\n```"
    );

    if let Some(existing) = &existing_test {
        user_prompt.push_str(&format!(
            "\n\nExisting test file for reference ({existing}):\n```\n{existing}\n```"
        ));
    }

    let client = ModelClient::new(&config.model);
    let tests = client.generate(SYSTEM_PROMPT, &user_prompt).await?;

    util::header("Generated Tests");
    println!("{tests}");

    Ok(())
}

fn extract_symbol_for_test(
    root: &Path,
    full_path: &Path,
    sym: &str,
    content: &str,
) -> String {
    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    let rel_path = full_path
        .strip_prefix(root)
        .unwrap_or(full_path)
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
                return lines[start..end].join("\n");
            }
        }
    }

    // Fallback: search for lines containing the symbol
    let matching: Vec<&str> = content
        .lines()
        .filter(|l| l.contains(sym))
        .take(50)
        .collect();
    matching.join("\n")
}

fn find_existing_test_file(
    root: &Path,
    source_path: &Path,
    security: &SecurityFilter,
) -> Option<String> {
    let stem = source_path.file_stem()?.to_str()?;
    let parent = source_path.parent()?;

    // Common test file patterns
    let candidates = vec![
        parent.join(format!("{stem}_test.rs")),
        parent.join(format!("test_{stem}.rs")),
        parent.join(format!("{stem}_test.go")),
        parent.join(format!("test_{stem}.py")),
        parent.join(format!("{stem}.test.ts")),
        parent.join(format!("{stem}.spec.ts")),
        parent.join("tests").join(format!("{stem}.rs")),
        parent.join("tests").join(format!("{stem}_test.go")),
    ];

    for candidate in &candidates {
        if candidate.exists() && security.is_path_allowed(candidate, root) {
            if let Ok(content) = std::fs::read_to_string(candidate) {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() > 100 {
                    return Some(lines[..100].join("\n"));
                }
                return Some(content);
            }
        }
    }

    None
}
