use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::DUMBCODER_DIR;

#[derive(Serialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub command: String,
    pub description: String,
    pub files_read: Vec<String>,
    pub diff_generated: String,
    pub diff_applied: bool,
    pub test_result: Option<String>,
    pub error: Option<String>,
}

impl AuditEntry {
    pub fn new(command: &str, description: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp = format!("{}", now.as_secs());

        Self {
            timestamp,
            command: command.to_string(),
            description: description.to_string(),
            files_read: Vec::new(),
            diff_generated: String::new(),
            diff_applied: false,
            test_result: None,
            error: None,
        }
    }
}

/// Write an audit entry to .dumbcoder/logs/ as a timestamped JSON file.
pub fn log_entry(root: &Path, entry: &AuditEntry) -> Result<()> {
    let log_dir = root.join(DUMBCODER_DIR).join("logs");
    std::fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    let filename = format!(
        "{}_{}.json",
        entry.timestamp,
        entry.command
    );
    let log_path = log_dir.join(filename);

    let json = serde_json::to_string_pretty(entry).context("Failed to serialize audit entry")?;
    std::fs::write(&log_path, json).context("Failed to write audit log")?;

    Ok(())
}
