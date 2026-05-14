use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub output: String,
}

/// Execute a parsed tool call.
pub fn execute_tool(call: &ToolCall, root: &Path) -> ToolResult {
    let result = match call.name.as_str() {
        "read_file" => exec_read_file(&call.args, root),
        "write_file" => exec_write_file(&call.args, root),
        "run_command" => exec_run_command(&call.args, root),
        "search_code" => exec_search_code(&call.args, root),
        "git_diff" => exec_git_diff(&call.args, root),
        "git_status" => exec_git_status(root),
        _ => Err(anyhow::anyhow!("Unknown tool: {}", call.name)),
    };

    match result {
        Ok(output) => ToolResult {
            tool: call.name.clone(),
            success: true,
            output,
        },
        Err(e) => ToolResult {
            tool: call.name.clone(),
            success: false,
            output: format!("Error: {e}"),
        },
    }
}

/// Parse tool calls from model response text.
/// Looks for ```tool\n{...}\n``` blocks.
pub fn parse_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut in_block = false;
    let mut block_content = String::new();

    for line in response.lines() {
        if line.trim() == "```tool" {
            in_block = true;
            block_content.clear();
            continue;
        }
        if in_block && line.trim() == "```" {
            in_block = false;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&block_content) {
                let name = value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = value.get("args").cloned().unwrap_or(serde_json::Value::Null);
                if !name.is_empty() {
                    calls.push(ToolCall { name, args });
                }
            }
            continue;
        }
        if in_block {
            block_content.push_str(line);
            block_content.push('\n');
        }
    }

    calls
}

/// Format tool results for feeding back to the model.
pub fn format_tool_results(results: &[ToolResult]) -> String {
    let mut output = String::from("Tool execution results:\n\n");
    for r in results {
        let status = if r.success { "OK" } else { "FAILED" };
        output.push_str(&format!("[{}] {}: \n{}\n\n", status, r.tool, r.output));
    }
    output
}

// --- Tool implementations ---

fn exec_read_file(args: &serde_json::Value, root: &Path) -> Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

    let full_path = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        root.join(path)
    };

    if !full_path.exists() {
        anyhow::bail!("File not found: {}", full_path.display());
    }

    let content = std::fs::read_to_string(&full_path)?;
    let line_count = content.lines().count();

    // Truncate if too long
    let truncated = if content.len() > 10000 {
        format!("{}...(truncated, {} total chars)", &content[..10000], content.len())
    } else {
        content
    };

    Ok(format!(
        "File: {} ({} lines)\n\n{}",
        full_path.display(),
        line_count,
        truncated
    ))
}

fn exec_write_file(args: &serde_json::Value, root: &Path) -> Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

    let full_path = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        root.join(path)
    };

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&full_path, content)?;
    let line_count = content.lines().count();

    Ok(format!(
        "Written {} lines to {}",
        line_count,
        full_path.display()
    ))
}

fn exec_run_command(args: &serde_json::Value, root: &Path) -> Result<String> {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

    // Safety: only allow whitelisted command prefixes
    let allowed = ["ls", "cat", "head", "tail", "wc", "find", "grep", "rg", "git", "python", "pip", "cargo", "npm", "node"];
    let first_word = command.split_whitespace().next().unwrap_or("");
    if !allowed.iter().any(|&a| first_word == a) {
        anyhow::bail!("Command not allowed: {first_word}. Allowed: {allowed:?}");
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push_str("\n--- stderr ---\n");
        }
        result.push_str(&stderr);
    }
    if result.is_empty() {
        result = "(no output)".into();
    }

    // Truncate
    if result.len() > 5000 {
        result = format!("{}...(truncated)", &result[..5000]);
    }

    Ok(result)
}

fn exec_search_code(args: &serde_json::Value, root: &Path) -> Result<String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;

    let output = Command::new("rg")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count=10")
        .arg("--context=2")
        .arg(query)
        .arg(root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.is_empty() {
        Ok(format!("No results for: {query}"))
    } else {
        let truncated = if stdout.len() > 5000 {
            format!("{}...(truncated)", &stdout[..5000])
        } else {
            stdout.to_string()
        };
        Ok(truncated)
    }
}

fn exec_git_diff(args: &serde_json::Value, root: &Path) -> Result<String> {
    let staged = args
        .get("staged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut cmd = Command::new("git");
    cmd.arg("diff");
    if staged {
        cmd.arg("--cached");
    }
    cmd.current_dir(root);

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.is_empty() {
        Ok("No changes.".into())
    } else {
        let truncated = if stdout.len() > 5000 {
            format!("{}...(truncated)", &stdout[..5000])
        } else {
            stdout.to_string()
        };
        Ok(truncated)
    }
}

fn exec_git_status(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.is_empty() {
        Ok("Working tree clean.".into())
    } else {
        Ok(stdout.to_string())
    }
}
