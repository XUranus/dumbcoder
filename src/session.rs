use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::DUMBCODER_DIR;
use crate::model::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,
    pub created_at: String,
    pub messages: Vec<ChatMessage>,
    pub plan: Option<String>,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: Option<String>,
    pub created_at: String,
    pub message_count: usize,
    pub mode: String,
}

impl Session {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            id: format!("{}", now.as_secs()),
            name: None,
            created_at: chrono_fmt(now.as_secs()),
            messages: Vec::new(),
            plan: None,
            mode: "chat".into(),
        }
    }
}

pub fn save_session(root: &Path, session: &Session) -> Result<()> {
    let dir = root.join(DUMBCODER_DIR).join("sessions");
    std::fs::create_dir_all(&dir).context("Failed to create sessions directory")?;

    let filename = match &session.name {
        Some(name) => format!("{}_{}.json", session.id, sanitize_name(name)),
        None => format!("{}.json", session.id),
    };

    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(session).context("Failed to serialize session")?;
    std::fs::write(&path, json).context("Failed to write session file")?;

    Ok(())
}

pub fn load_session(root: &Path, id_or_name: &str) -> Result<Session> {
    let dir = root.join(DUMBCODER_DIR).join("sessions");
    if !dir.is_dir() {
        anyhow::bail!("No sessions directory found");
    }

    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy();
        if stem == id_or_name || stem.contains(id_or_name) {
            let content = std::fs::read_to_string(&path)?;
            let session: Session =
                serde_json::from_str(&content).context("Failed to parse session file")?;
            return Ok(session);
        }
    }

    anyhow::bail!("Session '{id_or_name}' not found")
}

pub fn list_sessions(root: &Path) -> Result<Vec<SessionInfo>> {
    let dir = root.join(DUMBCODER_DIR).join("sessions");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<Session>(&content) {
                sessions.push(SessionInfo {
                    id: session.id.clone(),
                    name: session.name,
                    created_at: session.created_at,
                    message_count: session.messages.len(),
                    mode: session.mode,
                });
            }
        }
    }

    sessions.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(sessions)
}

pub fn latest_session(root: &Path) -> Option<Session> {
    let dir = root.join(DUMBCODER_DIR).join("sessions");
    if !dir.is_dir() {
        return None;
    }

    let mut latest: Option<(String, Session)> = None;
    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<Session>(&content) {
                if latest.as_ref().map_or(true, |(id, _)| session.id > *id) {
                    latest = Some((session.id.clone(), session));
                }
            }
        }
    }

    latest.map(|(_, s)| s)
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn chrono_fmt(secs: u64) -> String {
    // Simple timestamp formatting without chrono dependency
    format!("{secs}")
}
