use anyhow::Result;
use std::path::Path;

/// Get the staged diff (git diff --cached).
pub fn get_staged_diff(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached"])
        .current_dir(root)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get diff for a specific range (e.g. "main...HEAD").
pub fn get_diff_range(root: &Path, range: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["diff", range])
        .current_dir(root)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get unstaged working tree diff.
pub fn get_unstaged_diff(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["diff"])
        .current_dir(root)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Detect test commands for the project.
pub fn detect_test_command(root: &Path) -> Vec<String> {
    let mut commands = Vec::new();

    if root.join("Cargo.toml").is_file() {
        commands.push("cargo test".into());
    }
    if root.join("go.mod").is_file() {
        commands.push("go test ./...".into());
    }
    if root.join("pyproject.toml").is_file()
        || root.join("setup.py").is_file()
        || root.join("requirements.txt").is_file()
    {
        commands.push("pytest".into());
    }
    if root.join("package.json").is_file() {
        commands.push("npm test".into());
    }
    if root.join("pom.xml").is_file() {
        commands.push("mvn test".into());
    }
    if root.join("build.gradle").is_file() || root.join("build.gradle.kts").is_file() {
        commands.push("gradle test".into());
    }
    if root.join("Makefile").is_file() {
        commands.push("make test".into());
    }

    if commands.is_empty() {
        commands.push("unknown".into());
    }

    commands
}

/// List files changed in the diff string (parses `diff --git a/... b/...` lines).
pub fn parse_changed_files(diff: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            // Format: diff --git a/path b/path
            if let Some(b_part) = rest.split(" b/").last() {
                let file = b_part.trim().to_string();
                if !file.is_empty() && !files.contains(&file) {
                    files.push(file);
                }
            }
        }
    }
    files
}
