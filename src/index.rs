use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tree_sitter::{Parser, TreeCursor};

use crate::security::SecurityFilter;

// ── Types ──

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub path: String,
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: usize,
    pub end_line: usize,
    pub signature: String,
    pub body_snippet: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Class,
    Enum,
    Trait,
    Impl,
    Import,
    Interface,
    Constant,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Class => "class",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Impl => "impl",
            SymbolKind::Import => "import",
            SymbolKind::Interface => "interface",
            SymbolKind::Constant => "constant",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "function" => SymbolKind::Function,
            "method" => SymbolKind::Method,
            "struct" => SymbolKind::Struct,
            "class" => SymbolKind::Class,
            "enum" => SymbolKind::Enum,
            "trait" => SymbolKind::Trait,
            "impl" => SymbolKind::Impl,
            "import" => SymbolKind::Import,
            "interface" => SymbolKind::Interface,
            "constant" => SymbolKind::Constant,
            _ => SymbolKind::Function,
        }
    }
}

#[derive(Debug)]
pub struct IndexStats {
    pub files_scanned: usize,
    pub files_indexed: usize,
    pub symbols_found: usize,
    pub elapsed_ms: u128,
}

// ── SQLite Store ──

pub struct IndexStore {
    conn: Connection,
}

impl IndexStore {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create index directory")?;
        }

        let conn = Connection::open(db_path).context("Failed to open index database")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                language TEXT NOT NULL,
                size INTEGER,
                modified_at INTEGER,
                indexed_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY,
                file_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                signature TEXT,
                body_snippet TEXT,
                FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;",
        )?;

        Ok(Self { conn })
    }

    /// Get file ID if it exists and hasn't been modified since last index.
    fn get_file_cached_id(&self, rel_path: &str, modified_at: i64) -> Option<i64> {
        self.conn
            .query_row(
                "SELECT id, modified_at FROM files WHERE path = ?1",
                params![rel_path],
                |row| {
                    let id: i64 = row.get(0)?;
                    let cached_mtime: i64 = row.get(1)?;
                    Ok((id, cached_mtime))
                },
            )
            .ok()
            .and_then(|(id, cached_mtime)| {
                if cached_mtime >= modified_at {
                    Some(id)
                } else {
                    None
                }
            })
    }

    /// Upsert a file and its symbols.
    fn upsert_file(
        &self,
        rel_path: &str,
        language: &str,
        size: i64,
        modified_at: i64,
        symbols: &[ParsedSymbol],
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Delete old symbols for this file
        if let Ok(Some(file_id)) = self.get_file_id(rel_path) {
            self.conn
                .execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
            self.conn
                .execute("DELETE FROM files WHERE id = ?1", params![file_id])?;
        }

        // Insert file
        self.conn.execute(
            "INSERT INTO files (path, language, size, modified_at, indexed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rel_path, language, size, modified_at, now],
        )?;
        let file_id = self.conn.last_insert_rowid();

        // Insert symbols
        for sym in symbols {
            self.conn.execute(
                "INSERT INTO symbols (file_id, name, kind, start_line, end_line, signature, body_snippet)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    file_id,
                    sym.name,
                    sym.kind.as_str(),
                    sym.start_line as i64,
                    sym.end_line as i64,
                    sym.signature,
                    sym.body_snippet,
                ],
            )?;
        }

        Ok(())
    }

    fn get_file_id(&self, rel_path: &str) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT id FROM files WHERE path = ?1",
                params![rel_path],
                |row| row.get(0),
            )
            .ok()
            .map_or(Ok(None), |id: i64| Ok(Some(id)))
    }

    /// Index a single file.
    pub fn index_file(&self, abs_path: &Path, root: &Path) -> Result<IndexStats> {
        let start = SystemTime::now();
        let mut stats = IndexStats {
            files_scanned: 1,
            files_indexed: 0,
            symbols_found: 0,
            elapsed_ms: 0,
        };

        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .to_string();

        let language = match detect_language(abs_path) {
            Some(l) => l,
            None => return Ok(stats),
        };

        let meta = std::fs::metadata(abs_path)?;
        let size = meta.len() as i64;
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Check cache
        if let Some(_id) = self.get_file_cached_id(&rel_path, modified_at) {
            stats.elapsed_ms = start.elapsed().unwrap().as_millis();
            return Ok(stats);
        }

        let source = std::fs::read_to_string(abs_path)?;
        let symbols = parse_symbols(&source, language)?;

        self.upsert_file(&rel_path, language.as_str(), size, modified_at, &symbols)?;

        stats.files_indexed = 1;
        stats.symbols_found = symbols.len();
        stats.elapsed_ms = start.elapsed().unwrap().as_millis();
        Ok(stats)
    }

    /// Full index of all source files under root.
    pub fn index_all(&self, root: &Path, security: &SecurityFilter) -> Result<IndexStats> {
        let start = SystemTime::now();
        let mut stats = IndexStats {
            files_scanned: 0,
            files_indexed: 0,
            symbols_found: 0,
            elapsed_ms: 0,
        };

        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();
                security.is_path_allowed(path, root)
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let abs_path = entry.path();
            if detect_language(abs_path).is_none() {
                continue;
            }

            stats.files_scanned += 1;

            match self.index_file(abs_path, root) {
                Ok(s) => {
                    stats.files_indexed += s.files_indexed;
                    stats.symbols_found += s.symbols_found;
                }
                Err(e) => {
                    eprintln!("  Warning: failed to index {}: {e}", abs_path.display());
                }
            }
        }

        stats.elapsed_ms = start.elapsed().unwrap().as_millis();
        Ok(stats)
    }

    /// Index only changed files (via git diff).
    pub fn index_changed(&self, root: &Path, security: &SecurityFilter) -> Result<IndexStats> {
        let start = SystemTime::now();
        let mut stats = IndexStats {
            files_scanned: 0,
            files_indexed: 0,
            symbols_found: 0,
            elapsed_ms: 0,
        };

        let changed_files = get_git_changed_files(root)?;

        if changed_files.is_empty() {
            stats.elapsed_ms = start.elapsed().unwrap().as_millis();
            return Ok(stats);
        }

        for rel_path in &changed_files {
            let abs_path = root.join(rel_path);
            if !abs_path.exists() {
                // File deleted — remove from index
                self.remove_file(rel_path)?;
                continue;
            }
            if !security.is_path_allowed(&abs_path, root) {
                continue;
            }
            if detect_language(&abs_path).is_none() {
                continue;
            }

            stats.files_scanned += 1;

            match self.index_file(&abs_path, root) {
                Ok(s) => {
                    stats.files_indexed += s.files_indexed;
                    stats.symbols_found += s.symbols_found;
                }
                Err(e) => {
                    eprintln!("  Warning: failed to index {}: {e}", abs_path.display());
                }
            }
        }

        stats.elapsed_ms = start.elapsed().unwrap().as_millis();
        Ok(stats)
    }

    /// Search symbols by name (fuzzy LIKE query).
    pub fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<SymbolInfo>> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT f.path, s.name, s.kind, s.start_line, s.end_line, s.signature, s.body_snippet
             FROM symbols s JOIN files f ON s.file_id = f.id
             WHERE s.name LIKE ?1
             ORDER BY
               CASE WHEN s.name = ?2 THEN 0
                    WHEN s.name LIKE ?3 THEN 1
                    ELSE 2 END,
               s.kind, s.name
             LIMIT ?4",
        )?;

        let rows = stmt.query_map(params![pattern, query, format!("{query}%"), limit as i64], |row| {
            Ok(SymbolInfo {
                path: row.get(0)?,
                name: row.get(1)?,
                kind: SymbolKind::from_str(&row.get::<_, String>(2)?),
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
                signature: row.get(5)?,
                body_snippet: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get a specific symbol by file path and name.
    pub fn get_symbol(&self, file: &str, name: &str) -> Result<Option<SymbolInfo>> {
        self.conn
            .query_row(
                "SELECT f.path, s.name, s.kind, s.start_line, s.end_line, s.signature, s.body_snippet
                 FROM symbols s JOIN files f ON s.file_id = f.id
                 WHERE f.path = ?1 AND s.name = ?2",
                params![file, name],
                |row| {
                    Ok(SymbolInfo {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        kind: SymbolKind::from_str(&row.get::<_, String>(2)?),
                        start_line: row.get::<_, i64>(3)? as usize,
                        end_line: row.get::<_, i64>(4)? as usize,
                        signature: row.get(5)?,
                        body_snippet: row.get(6)?,
                    })
                },
            )
            .ok()
            .map_or(Ok(None), |s| Ok(Some(s)))
    }

    /// Get all symbols in a file.
    pub fn get_file_symbols(&self, file: &str) -> Result<Vec<SymbolInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.path, s.name, s.kind, s.start_line, s.end_line, s.signature, s.body_snippet
             FROM symbols s JOIN files f ON s.file_id = f.id
             WHERE f.path = ?1
             ORDER BY s.start_line",
        )?;

        let rows = stmt.query_map(params![file], |row| {
            Ok(SymbolInfo {
                path: row.get(0)?,
                name: row.get(1)?,
                kind: SymbolKind::from_str(&row.get::<_, String>(2)?),
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
                signature: row.get(5)?,
                body_snippet: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Remove a file and its symbols from the index.
    pub fn remove_file(&self, rel_path: &str) -> Result<()> {
        if let Some(file_id) = self.get_file_id(rel_path)? {
            self.conn
                .execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
            self.conn
                .execute("DELETE FROM files WHERE id = ?1", params![file_id])?;
        }
        Ok(())
    }

    /// Get total stats.
    pub fn total_stats(&self) -> Result<(usize, usize)> {
        let file_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM files",
            [],
            |row| row.get(0),
        )?;
        let symbol_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols",
            [],
            |row| row.get(0),
        )?;
        Ok((file_count, symbol_count))
    }
}

// ── Language Detection ──

fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some(Language::Rust),
        "go" => Some(Language::Go),
        "py" => Some(Language::Python),
        "ts" | "tsx" => Some(Language::TypeScript),
        "java" => Some(Language::Java),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
        "js" | "jsx" | "mjs" => Some(Language::JavaScript),
        "rb" | "rake" => Some(Language::Ruby),
        "kt" | "kts" => Some(Language::Kotlin),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Language {
    Rust,
    Go,
    Python,
    TypeScript,
    Java,
    C,
    Cpp,
    JavaScript,
    Ruby,
    Kotlin,
}

impl Language {
    fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Java => "java",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::JavaScript => "javascript",
            Language::Ruby => "ruby",
            Language::Kotlin => "kotlin",
        }
    }
}

// ── Tree-sitter Parsing ──

struct ParsedSymbol {
    name: String,
    kind: SymbolKind,
    start_line: usize,
    end_line: usize,
    signature: String,
    body_snippet: String,
}

fn parse_symbols(source: &str, language: Language) -> Result<Vec<ParsedSymbol>> {
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
    };
    parser.set_language(&lang)?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse source")?;

    let mut symbols = Vec::new();
    let mut cursor = tree.walk();
    extract_symbols(&mut cursor, source, language, &mut symbols);

    Ok(symbols)
}

fn extract_symbols(
    cursor: &mut TreeCursor,
    source: &str,
    language: Language,
    symbols: &mut Vec<ParsedSymbol>,
) {
    let source_bytes = source.as_bytes();
    let lines: Vec<&str> = source.lines().collect();

    loop {
        let node = cursor.node();
        let kind_str = node.kind();

        if let Some(sym_kind) = node_kind_to_symbol(kind_str, language) {
            let name = extract_name(node, source_bytes, language);
            if !name.is_empty() {
                let start_line = node.start_position().row;
                let end_line = node.end_position().row;
                let signature = node
                    .utf8_text(source_bytes)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let body_snippet = extract_snippet(&lines, start_line, end_line, 10);

                symbols.push(ParsedSymbol {
                    name,
                    kind: sym_kind,
                    start_line: start_line + 1, // 1-indexed
                    end_line: end_line + 1,
                    signature,
                    body_snippet,
                });
            }
        }

        // Recurse into children
        if cursor.goto_first_child() {
            extract_symbols(cursor, source, language, symbols);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn node_kind_to_symbol(kind: &str, language: Language) -> Option<SymbolKind> {
    match language {
        Language::Rust => match kind {
            "function_item" => Some(SymbolKind::Function),
            "struct_item" => Some(SymbolKind::Struct),
            "enum_item" => Some(SymbolKind::Enum),
            "trait_item" => Some(SymbolKind::Trait),
            "impl_item" => Some(SymbolKind::Impl),
            "use_declaration" => Some(SymbolKind::Import),
            "const_item" | "static_item" => Some(SymbolKind::Constant),
            _ => None,
        },
        Language::Go => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "method_declaration" => Some(SymbolKind::Method),
            "type_declaration" => Some(SymbolKind::Struct),
            "import_declaration" => Some(SymbolKind::Import),
            _ => None,
        },
        Language::Python => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "class_definition" => Some(SymbolKind::Class),
            "import_statement" | "import_from_statement" => Some(SymbolKind::Import),
            _ => None,
        },
        Language::TypeScript => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "class_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "type_alias_declaration" => Some(SymbolKind::Struct),
            "import_statement" => Some(SymbolKind::Import),
            "enum_declaration" => Some(SymbolKind::Enum),
            _ => None,
        },
        Language::Java => match kind {
            "method_declaration" => Some(SymbolKind::Method),
            "constructor_declaration" => Some(SymbolKind::Method),
            "class_declaration" => Some(SymbolKind::Class),
            "interface_declaration" => Some(SymbolKind::Interface),
            "enum_declaration" => Some(SymbolKind::Enum),
            "import_declaration" => Some(SymbolKind::Import),
            _ => None,
        },
        Language::C => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "struct_specifier" => Some(SymbolKind::Struct),
            "enum_specifier" => Some(SymbolKind::Enum),
            "type_definition" => Some(SymbolKind::Struct),
            "preproc_include" => Some(SymbolKind::Import),
            _ => None,
        },
        Language::Cpp => match kind {
            "function_definition" => Some(SymbolKind::Function),
            "class_specifier" => Some(SymbolKind::Class),
            "struct_specifier" => Some(SymbolKind::Struct),
            "enum_specifier" => Some(SymbolKind::Enum),
            "namespace_definition" => Some(SymbolKind::Impl),
            "preproc_include" => Some(SymbolKind::Import),
            _ => None,
        },
        Language::JavaScript => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "method_definition" => Some(SymbolKind::Method),
            "class_declaration" => Some(SymbolKind::Class),
            "lexical_declaration" => Some(SymbolKind::Constant),
            "import_statement" => Some(SymbolKind::Import),
            _ => None,
        },
        Language::Ruby => match kind {
            "method" => Some(SymbolKind::Function),
            "singleton_method" => Some(SymbolKind::Method),
            "class" => Some(SymbolKind::Class),
            "module" => Some(SymbolKind::Impl),
            _ => None,
        },
        Language::Kotlin => match kind {
            "function_declaration" => Some(SymbolKind::Function),
            "class_declaration" => Some(SymbolKind::Class),
            "object_declaration" => Some(SymbolKind::Class),
            "property_declaration" => Some(SymbolKind::Constant),
            "import_header" => Some(SymbolKind::Import),
            _ => None,
        },
    }
}

fn extract_name(
    node: tree_sitter::Node,
    source: &[u8],
    language: Language,
) -> String {
    // Try to find a "name" child node
    let name_field = match language {
        Language::Rust => "name",
        Language::Go => "name",
        Language::Python => "name",
        Language::TypeScript => "name",
        Language::Java => "name",
        Language::C => "name",
        Language::Cpp => "name",
        Language::JavaScript => "name",
        Language::Ruby => "name",
        Language::Kotlin => "name",
    };

    // For imports, use the whole text
    if node.kind().contains("import") || node.kind().contains("use") {
        return node
            .utf8_text(source)
            .unwrap_or("")
            .trim()
            .chars()
            .take(100)
            .collect();
    }

    // Try field-based name lookup
    if let Some(name_node) = node.child_by_field_name(name_field) {
        return name_node
            .utf8_text(source)
            .unwrap_or("")
            .trim()
            .to_string();
    }

    // Fallback: scan children for identifier
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "identifier"
                || child.kind() == "type_identifier"
                || child.kind() == "property_identifier"
            {
                return child
                    .utf8_text(source)
                    .unwrap_or("")
                    .trim()
                    .to_string();
            }
        }
    }

    String::new()
}

fn extract_snippet(lines: &[&str], start: usize, end: usize, max_lines: usize) -> String {
    let end = std::cmp::min(start + max_lines, end + 1);
    let end = std::cmp::min(end, lines.len());
    if start >= lines.len() {
        return String::new();
    }
    lines[start..end].join("\n")
}

// ── Git Integration ──

fn get_git_changed_files(root: &Path) -> Result<Vec<String>> {
    // Get uncommitted changes (modified + untracked)
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(root)
        .output()?;

    let mut files = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            files.push(trimmed.to_string());
        }
    }

    // Also get staged files
    let staged = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(root)
        .output()?;

    for line in String::from_utf8_lossy(&staged.stdout).lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !files.contains(&trimmed.to_string()) {
            files.push(trimmed.to_string());
        }
    }

    // Also get untracked files
    let untracked = std::process::Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()?;

    for line in String::from_utf8_lossy(&untracked.stdout).lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !files.contains(&trimmed.to_string()) {
            files.push(trimmed.to_string());
        }
    }

    Ok(files)
}
