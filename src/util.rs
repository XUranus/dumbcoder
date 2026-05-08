use colored::Colorize;
use std::path::Path;

/// Print a section header.
pub fn header(text: &str) {
    println!("\n{}", text.bold().cyan());
    println!("{}", "─".repeat(60).dimmed());
}

/// Print a sub-section header.
pub fn sub_header(text: &str) {
    println!("\n{}", text.bold());
}

/// Print an info line.
pub fn info(label: &str, value: &str) {
    println!("  {}: {}", label.green(), value);
}

/// Print a file reference.
pub fn file_ref(path: &str, line: u32) {
    println!("  {}:{}:{}", path.blue(), line.to_string().yellow(), "");
}

/// Print a code block.
pub fn code_block(path: &str, content: &str) {
    println!("  {}", format!("── {path} ──").dimmed());
    for line in content.lines().take(50) {
        println!("  {line}");
    }
    println!();
}

/// Truncate a string to max_chars.
pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}...(truncated)", &s[..max_chars])
    }
}

/// Check if the current directory is a git repository.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").is_dir()
}

/// Detect project language from common project files.
pub fn detect_project_language(root: &Path) -> Vec<String> {
    let mut langs = Vec::new();
    let markers = [
        ("Cargo.toml", "rust"),
        ("go.mod", "go"),
        ("package.json", "javascript/typescript"),
        ("pyproject.toml", "python"),
        ("setup.py", "python"),
        ("requirements.txt", "python"),
        ("pom.xml", "java"),
        ("build.gradle", "java"),
        ("CMakeLists.txt", "c/cpp"),
        ("Makefile", "c/cpp"),
    ];

    for (file, lang) in &markers {
        if root.join(file).is_file() {
            langs.push(lang.to_string());
        }
    }

    if langs.is_empty() {
        langs.push("unknown".into());
    }

    langs
}
