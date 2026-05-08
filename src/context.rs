use crate::security::SecurityFilter;
use anyhow::Result;
use std::path::Path;

pub struct SearchMatch {
    pub file: String,
    pub line_number: u32,
    pub line: String,
}

pub struct CodeContext {
    pub matches: Vec<SearchMatch>,
    pub file_contents: Vec<FileContent>,
}

pub struct FileContent {
    pub path: String,
    pub content: String,
}

impl CodeContext {
    /// Build context from rg search results.
    pub fn from_search_results(
        raw_output: &str,
        project_root: &Path,
        security: &SecurityFilter,
        max_files: usize,
        max_context_lines: usize,
    ) -> Result<Self> {
        let mut matches = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for line in raw_output.lines() {
            if let Some(parsed) = parse_rg_line(line) {
                let file_path = Path::new(&parsed.file);
                if !security.is_path_allowed(file_path, project_root) {
                    continue;
                }
                if seen_files.len() < max_files {
                    seen_files.insert(parsed.file.clone());
                }
                matches.push(parsed);
            }
        }

        let mut file_contents = Vec::new();
        for file in &seen_files {
            if let Ok(content) = std::fs::read_to_string(file) {
                let lines: Vec<&str> = content.lines().collect();
                let limited = if lines.len() > max_context_lines {
                    lines[..max_context_lines].join("\n")
                } else {
                    content
                };
                file_contents.push(FileContent {
                    path: file.clone(),
                    content: limited,
                });
            }
        }

        Ok(Self {
            matches,
            file_contents,
        })
    }

    /// Format context for inclusion in a model prompt.
    pub fn format_for_prompt(&self, max_chars: usize) -> String {
        let mut result = String::new();
        let mut total = 0;

        for fc in &self.file_contents {
            let block = format!("=== {} ===\n{}\n\n", fc.path, fc.content);
            if total + block.len() > max_chars {
                break;
            }
            total += block.len();
            result.push_str(&block);
        }

        result
    }
}

fn parse_rg_line(line: &str) -> Option<SearchMatch> {
    let mut parts = line.splitn(3, ':');
    let file = parts.next()?.to_string();
    let line_number: u32 = parts.next()?.parse().ok()?;
    let content = parts.next()?.to_string();

    if file.is_empty() || content.is_empty() {
        return None;
    }

    Some(SearchMatch {
        file,
        line_number,
        line: content,
    })
}
