use crate::context::FileContent;
use crate::index::SymbolInfo;
use crate::model::ChatMessage;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Ready,
    Thinking,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Chat,
    Plan,
}

pub enum AppAction {
    Send,
    Quit,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ClearChat,
    None,
}

pub enum AppEvent {
    Tick,
    Key(KeyEvent),
    ModelResult(anyhow::Result<ModelResponse>),
}

pub struct ModelResponse {
    pub answer: String,
    pub context_files: Vec<FileContent>,
    pub context_symbols: Vec<SymbolInfo>,
}

pub struct App {
    pub running: bool,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub input_cursor: usize,
    pub status: AppStatus,
    pub scroll_chat: u16,
    pub last_error: Option<String>,
    pub spinner_frame: usize,
    pub mode: AppMode,
    pub plan_content: Option<String>,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub session_id: Option<String>,
    pub completions: Vec<String>,
    pub completion_index: usize,
}

const ALL_COMMANDS: &[&str] = &[
    "/help", "/clear", "/model", "/status", "/commit",
    "/plan", "/approve", "/cancel",
    "/session save", "/session load", "/session list",
    "/read ", "/exec ",
];

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            messages: Vec::new(),
            input: String::new(),
            input_cursor: 0,
            status: AppStatus::Ready,
            scroll_chat: 0,
            last_error: None,
            spinner_frame: 0,
            mode: AppMode::Chat,
            plan_content: None,
            history: Vec::new(),
            history_index: None,
            session_id: None,
            completions: Vec::new(),
            completion_index: 0,
        }
    }

    /// Update completions based on current input (prefix matching).
    pub fn update_completions(&mut self) {
        self.completions.clear();
        self.completion_index = 0;

        if !self.input.starts_with('/') {
            return;
        }

        let input_lower = self.input.to_lowercase();
        for &cmd in ALL_COMMANDS {
            if cmd.starts_with(&input_lower) && cmd != input_lower.as_str() {
                self.completions.push(cmd.to_string());
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> AppAction {
        // Global quit
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AppAction::Quit;
        }

        // Ctrl+L clear
        if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AppAction::ClearChat;
        }

        // Esc: clear input, or quit if input empty
        if key.code == KeyCode::Esc {
            if self.input.is_empty() {
                return AppAction::Quit;
            }
            self.input.clear();
            self.input_cursor = 0;
            self.completions.clear();
            return AppAction::None;
        }

        // Tab: accept first completion
        if key.code == KeyCode::Tab {
            if !self.completions.is_empty() {
                let idx = self.completion_index % self.completions.len();
                let completed = &self.completions[idx];
                // Add trailing space if command doesn't end with one
                let suffix = if completed.ends_with(' ') { "" } else { " " };
                self.input = format!("{completed}{suffix}");
                self.input_cursor = self.input.len();
                self.update_completions();
            }
            return AppAction::None;
        }

        // Shift+Tab: cycle completions backwards
        if key.code == KeyCode::BackTab {
            if !self.completions.is_empty() {
                self.completion_index = self.completion_index.wrapping_add(self.completions.len() - 1) % self.completions.len();
            }
            return AppAction::None;
        }

        // PgUp/PgDn: scroll chat
        if key.code == KeyCode::PageUp {
            return AppAction::ScrollPageUp;
        }
        if key.code == KeyCode::PageDown {
            return AppAction::ScrollPageDown;
        }

        // Up/Down: history navigation
        if key.code == KeyCode::Up && self.status != AppStatus::Thinking {
            return self.history_prev();
        }
        if key.code == KeyCode::Down && self.status != AppStatus::Thinking {
            return self.history_next();
        }

        // Enter: send
        if key.code == KeyCode::Enter {
            if !self.input.is_empty() && self.status != AppStatus::Thinking {
                return AppAction::Send;
            }
            return AppAction::None;
        }

        // Input editing
        match key.code {
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input.remove(self.input_cursor);
                }
            }
            KeyCode::Delete => {
                if self.input_cursor < self.input.len() {
                    self.input.remove(self.input_cursor);
                }
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.input_cursor < self.input.len() {
                    self.input_cursor += 1;
                }
            }
            KeyCode::Home => self.input_cursor = 0,
            KeyCode::End => self.input_cursor = self.input.len(),
            KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
                self.input.insert(self.input_cursor, c);
                self.input_cursor += 1;
            }
            _ => {}
        }

        self.update_completions();
        AppAction::None
    }

    pub fn push_user_message(&mut self, text: String) {
        if !text.is_empty() {
            self.history.push(text.clone());
            if self.history.len() > 200 {
                self.history.remove(0);
            }
        }
        self.history_index = None;
        self.messages.push(ChatMessage { role: "user".into(), content: text });
        self.auto_scroll();
    }

    pub fn push_system_message(&mut self, text: &str) {
        self.messages.push(ChatMessage { role: "system".into(), content: text.to_string() });
        self.auto_scroll();
    }

    pub fn receive_model_response(&mut self, result: anyhow::Result<ModelResponse>) {
        self.spinner_frame = 0;
        match result {
            Ok(resp) => {
                self.messages.push(ChatMessage { role: "assistant".into(), content: resp.answer });
                self.status = AppStatus::Ready;
                self.auto_scroll();
            }
            Err(e) => {
                self.status = AppStatus::Error;
                self.last_error = Some(e.to_string());
            }
        }
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.scroll_chat = 0;
        self.last_error = None;
        self.status = AppStatus::Ready;
        self.plan_content = None;
        self.mode = AppMode::Chat;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_chat > 0 {
            self.scroll_chat -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_chat += 1;
    }

    pub fn tick_spinner(&mut self) {
        if self.status == AppStatus::Thinking {
            self.spinner_frame = (self.spinner_frame + 1) % 4;
        }
    }

    /// Auto-scroll to bottom when new messages arrive.
    fn auto_scroll(&mut self) {
        // Set scroll to a very large value; the render function will clamp it.
        self.scroll_chat = u16::MAX;
    }

    fn history_prev(&mut self) -> AppAction {
        if self.history.is_empty() {
            return AppAction::None;
        }
        let idx = match self.history_index {
            Some(i) => i.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        self.input = self.history[idx].clone();
        self.input_cursor = self.input.len();
        self.update_completions();
        AppAction::None
    }

    fn history_next(&mut self) -> AppAction {
        match self.history_index {
            Some(i) if i < self.history.len() - 1 => {
                let idx = i + 1;
                self.history_index = Some(idx);
                self.input = self.history[idx].clone();
                self.input_cursor = self.input.len();
            }
            Some(_) => {
                self.history_index = None;
                self.input.clear();
                self.input_cursor = 0;
            }
            None => {}
        }
        self.update_completions();
        AppAction::None
    }
}
