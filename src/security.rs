use std::path::Path;

const DEFAULT_BLACKLISTED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
    ".dumbcoder",
];

const DEFAULT_BLACKLISTED_FILES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    "credentials.*",
    "secrets.*",
];

const DEFAULT_BLACKLISTED_EXTENSIONS: &[&str] = &["pem", "key", "p12", "pfx", "jks"];

pub struct SecurityFilter {
    blacklisted_dirs: Vec<String>,
    blacklisted_files: Vec<String>,
    blacklisted_extensions: Vec<String>,
    extra_ignore_dirs: Vec<String>,
}

impl SecurityFilter {
    pub fn new(extra_ignore_dirs: Vec<String>) -> Self {
        Self {
            blacklisted_dirs: DEFAULT_BLACKLISTED_DIRS.iter().map(|s| s.to_string()).collect(),
            blacklisted_files: DEFAULT_BLACKLISTED_FILES.iter().map(|s| s.to_string()).collect(),
            blacklisted_extensions: DEFAULT_BLACKLISTED_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            extra_ignore_dirs,
        }
    }

    /// Check if a path is allowed (not in blacklist, within project root).
    pub fn is_path_allowed(&self, path: &Path, project_root: &Path) -> bool {
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Try relative to project root
                match project_root.join(path).canonicalize() {
                    Ok(p) => p,
                    Err(_) => return false,
                }
            }
        };

        let root_canonical = match project_root.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Must be within project root
        if !canonical.starts_with(&root_canonical) {
            return false;
        }

        // Check directory components
        for component in canonical.components() {
            let name = component.as_os_str().to_string_lossy();
            if self.blacklisted_dirs.contains(&name.to_string())
                || self.extra_ignore_dirs.contains(&name.to_string())
            {
                return false;
            }
        }

        // Check filename
        if let Some(file_name) = canonical.file_name() {
            let name = file_name.to_string_lossy();
            for pattern in &self.blacklisted_files {
                if glob_match(pattern, &name) {
                    return false;
                }
            }
        }

        // Check extension
        if let Some(ext) = canonical.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if self.blacklisted_extensions.contains(&ext_str.to_string()) {
                return false;
            }
        }

        true
    }

    /// Check if a command is allowed.
    pub fn is_command_allowed(&self, command: &str, allowed_commands: &[String]) -> bool {
        for allowed in allowed_commands {
            if command.starts_with(allowed.as_str()) {
                return true;
            }
        }
        false
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return text.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return text.starts_with(prefix);
    }
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.pem", "server.pem"));
        assert!(glob_match("credentials.*", "credentials.json"));
        assert!(!glob_match("*.pem", "server.key"));
        assert!(glob_match("id_rsa", "id_rsa"));
    }
}
