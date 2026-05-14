use anyhow::{bail, Result};
use std::io::{self, Write};
use std::process::Command;

use crate::audit::{self, AuditEntry};
use crate::config::{Config, DUMBCODER_DIR};
use crate::context::CodeContext;
use crate::git;
use crate::index::IndexStore;
use crate::model::ModelClient;
use crate::plugin;
use crate::security::SecurityFilter;
use crate::util;

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a code modification assistant. Given a description of a fix and relevant code context, generate a unified diff patch.

Rules:
1. Output ONLY a valid unified diff (--- / +++ format with @@ hunks).
2. The diff must apply cleanly with `git apply`.
3. Make minimal, focused changes that directly address the description.
4. Do not include any explanations, markdown formatting, or commentary — only the raw diff.
5. Use standard unified diff format:
   --- a/path/to/file
   +++ b/path/to/file
   @@ -line,count +line,count @@
   -old line
   +new line"#;

pub async fn run(description: &str) -> Result<()> {
    let root = Config::find_project_root()?;
    let config = Config::load(&root)?;
    let security = SecurityFilter::new(config.index.ignore.clone());

    let mut audit = AuditEntry::new("patch", description);

    util::header("Patch");
    util::info("description", description);

    // Step 1: Search for relevant code
    util::header("Searching codebase");

    let search_query = extract_keywords(description);
    eprintln!("  Searching for: {search_query}");

    let rg_output = Command::new("rg")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count=10")
        .arg("--context=3")
        .arg(&search_query)
        .arg(&root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Step 2: Build context
    let mut context = CodeContext::from_search_results(&rg_output, &root, &security, 10, 200)?;

    let db_path = root.join(DUMBCODER_DIR).join("index").join("symbols.db");
    if db_path.exists() {
        if let Ok(store) = IndexStore::open(&db_path) {
            if let Ok(symbols) = store.search_symbols(&search_query, 10) {
                if let Ok(sym_ctx) = CodeContext::from_symbols(&symbols, &root, &security, 4000) {
                    context.merge(sym_ctx);
                }
            }
        }
    }

    let context_text = context.format_for_prompt(10000);

    // Record files read
    let mut seen_files = std::collections::HashSet::new();
    for fc in &context.file_contents {
        seen_files.insert(fc.path.clone());
    }
    audit.files_read = seen_files.into_iter().collect();

    if context_text.is_empty() {
        bail!("No relevant code found for: {description}");
    }

    // Step 3: Generate diff
    util::header("Generating patch");
    eprintln!("  Model: {} ({}) @ {}", config.model.model, config.model.provider, config.model.base_url);

    let client = ModelClient::new(&config.model)?;
    let user_prompt = format!(
        "Generate a patch for the following fix:\n\nDescription: {description}\n\nRelevant code:\n{context_text}"
    );

    let system_prompt = plugin::resolve_prompt(&config, "patch", DEFAULT_SYSTEM_PROMPT);
    let response = client.generate(&system_prompt, &user_prompt).await?;

    // Step 4: Extract diff from response
    let diff = extract_diff(&response);
    if diff.trim().is_empty() {
        eprintln!("  Model did not generate a valid diff.");
        eprintln!("  Raw response:");
        eprintln!("{response}");
        audit.error = Some("Model did not generate a valid diff".into());
        audit::log_entry(&root, &audit)?;
        bail!("No valid diff generated");
    }

    audit.diff_generated = diff.clone();

    // Step 5: Validate diff with git apply --check
    util::header("Validating patch");

    let check_result = Command::new("git")
        .args(["apply", "--check"])
        .current_dir(&root)
        .arg("--verbose")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(diff.as_bytes())?;
            }
            child.wait_with_output()
        });

    match check_result {
        Ok(output) if output.status.success() => {
            eprintln!("  Patch validation: OK");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  Patch validation FAILED:");
            eprintln!("  {stderr}");
            util::header("Generated diff (for inspection)");
            println!("{diff}");
            audit.error = Some(format!("git apply --check failed: {stderr}"));
            audit::log_entry(&root, &audit)?;
            bail!("Generated patch does not apply cleanly");
        }
        Err(e) => {
            eprintln!("  Warning: could not run git apply --check: {e}");
        }
    }

    // Step 6: Display diff
    util::header("Generated patch");
    println!("{diff}");

    // Step 7: Confirm with user
    util::header("Confirmation");
    print!("  Apply this patch? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
        eprintln!("  Patch cancelled.");
        audit.diff_applied = false;
        audit::log_entry(&root, &audit)?;
        return Ok(());
    }

    // Step 8: Apply patch
    util::header("Applying patch");

    let apply_result = Command::new("git")
        .args(["apply"])
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(diff.as_bytes())?;
            }
            child.wait_with_output()
        })?;

    if !apply_result.status.success() {
        let stderr = String::from_utf8_lossy(&apply_result.stderr);
        eprintln!("  Failed to apply patch: {stderr}");
        audit.error = Some(format!("git apply failed: {stderr}"));
        audit::log_entry(&root, &audit)?;
        bail!("Failed to apply patch");
    }

    eprintln!("  Patch applied successfully.");
    audit.diff_applied = true;

    // Step 9: Run tests
    let test_commands = git::detect_test_command(&root);
    let test_cmd = test_commands.first().map(|s| s.as_str()).unwrap_or("");

    if !test_cmd.is_empty() && test_cmd != "unknown" {
        util::header("Running tests");
        eprintln!("  Command: {test_cmd}");

        let parts: Vec<&str> = test_cmd.split_whitespace().collect();
        if !parts.is_empty() {
            let test_output = Command::new(parts[0])
                .args(&parts[1..])
                .current_dir(&root)
                .output();

            match test_output {
                Ok(output) if output.status.success() => {
                    eprintln!("  Tests PASSED.");
                    audit.test_result = Some("PASSED".into());
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let test_log = format!("{stdout}{stderr}");
                    let summary = if test_log.len() > 500 {
                        format!("{}...(truncated)", &test_log[..500])
                    } else {
                        test_log
                    };

                    eprintln!("  Tests FAILED.");
                    eprintln!("  {summary}");
                    audit.test_result = Some(format!("FAILED: {summary}"));

                    // Step 10: Rollback
                    util::header("Rolling back patch");
                    let rollback = Command::new("git")
                        .args(["apply", "--reverse"])
                        .current_dir(&root)
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .and_then(|mut child| {
                            if let Some(mut stdin) = child.stdin.take() {
                                stdin.write_all(diff.as_bytes())?;
                            }
                            child.wait_with_output()
                        });

                    match rollback {
                        Ok(r) if r.status.success() => {
                            eprintln!("  Patch rolled back successfully.");
                        }
                        _ => {
                            eprintln!("  WARNING: Failed to roll back patch!");
                            eprintln!("  You may need to manually run: git checkout -- .");
                        }
                    }

                    audit::log_entry(&root, &audit)?;
                    bail!("Tests failed, patch rolled back");
                }
                Err(e) => {
                    eprintln!("  Could not run tests: {e}");
                    audit.test_result = Some(format!("Error: {e}"));
                }
            }
        }
    } else {
        eprintln!("  No test command detected, skipping tests.");
        audit.test_result = Some("SKIPPED (no test command detected)".into());
    }

    // Step 11: Log
    audit::log_entry(&root, &audit)?;

    util::header("Done");
    eprintln!("  Patch applied and logged successfully.");

    Ok(())
}

fn extract_diff(response: &str) -> String {
    let mut diff_lines = Vec::new();
    let mut in_diff = false;

    for line in response.lines() {
        if line.starts_with("diff --git") || line.starts_with("--- a/") || line.starts_with("--- ") {
            in_diff = true;
        }

        if in_diff {
            diff_lines.push(line);
        }
    }

    // If no diff markers found, try to find lines that look like a diff
    if diff_lines.is_empty() {
        for line in response.lines() {
            if line.starts_with("@@ ") || line.starts_with("+++") || line.starts_with("--- ") {
                in_diff = true;
            }
            if in_diff {
                diff_lines.push(line);
            }
        }
    }

    diff_lines.join("\n")
}

fn extract_keywords(text: &str) -> String {
    let stop_words: std::collections::HashSet<&str> = [
        "fix", "the", "a", "an", "in", "of", "for", "and", "or", "to",
        "when", "is", "it", "this", "that", "should", "would", "could",
        "be", "been", "have", "has", "had", "do", "does", "did",
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
