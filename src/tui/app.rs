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
    ScrollPageUp,
    ScrollPageDown,
    ClearChat,
    None,
}

pub enum AppEvent {
    Tick,
    Key(KeyEvent),
    ModelResult(anyhow::Result<String>),
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
    pub session_id: String,
    pub completions: Vec<String>,
    pub completion_index: usize,
    /// Whether the completion popup is active and arrows should select items
    pub completion_active: bool,
}

const ALL_COMMANDS: &[&str] = &[
    "/help", "/clear", "/model", "/status", "/commit",
    "/plan", "/approve", "/cancel", "/exit",
    "/read ", "/exec ",
];

impl App {
    pub fn new(session_id: String) -> Self {
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
            session_id,
            completions: Vec::new(),
            completion_index: 0,
            completion_active: false,
        }
    }

    /// Update completions based on current input prefix.
    pub fn update_completions(&mut self) {
        self.completions.clear();
        self.completion_index = 0;

        if !self.input.starts_with('/') {
            self.completion_active = false;
            return;
        }

        let input_lower = self.input.to_lowercase();
        for &cmd in ALL_COMMANDS {
            if cmd.starts_with(&input_lower) && cmd != input_lower.as_str() {
                self.completions.push(cmd.to_string());
            }
        }

        self.completion_active = !self.completions.is_empty();
        // Clamp index
        if self.completion_index >= self.completions.len() {
            self.completion_index = 0;
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

        // Esc: dismiss completions, or clear input, or quit
        if key.code == KeyCode::Esc {
            if self.completion_active {
                self.completion_active = false;
                self.completions.clear();
                return AppAction::None;
            }
            if !self.input.is_empty() {
                self.input.clear();
                self.input_cursor = 0;
                return AppAction::None;
            }
            return AppAction::Quit;
        }

        // Tab: accept current completion
        if key.code == KeyCode::Tab {
            if self.completion_active && !self.completions.is_empty() {
                let completed = &self.completions[self.completion_index];
                let suffix = if completed.ends_with(' ') { "" } else { " " };
                self.input = format!("{completed}{suffix}");
                self.input_cursor = self.input.len();
                self.update_completions();
            }
            return AppAction::None;
        }

        // Up arrow
        if key.code == KeyCode::Up {
            // If completions active → cycle completion
            if self.completion_active && !self.completions.is_empty() {
                self.completion_index =
                    (self.completion_index + self.completions.len() - 1) % self.completions.len();
                return AppAction::None;
            }
            // Otherwise → history
            return self.history_prev();
        }

        // Down arrow
        if key.code == KeyCode::Down {
            if self.completion_active && !self.completions.is_empty() {
                self.completion_index = (self.completion_index + 1) % self.completions.len();
                return AppAction::None;
            }
            return self.history_next();
        }

        // PgUp/PgDn: scroll chat
        if key.code == KeyCode::PageUp {
            return AppAction::ScrollPageUp;
        }
        if key.code == KeyCode::PageDown {
            return AppAction::ScrollPageDown;
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
            if self.history.len() > 500 {
                self.history.remove(0);
            }
        }
        self.history_index = None;
        self.messages.push(ChatMessage { role: "user".into(), content: text });
        self.auto_scroll();
    }

    pub fn push_assistant_message(&mut self, text: String) {
        self.messages.push(ChatMessage { role: "assistant".into(), content: text });
        self.status = AppStatus::Ready;
        self.spinner_frame = 0;
        self.auto_scroll();
    }

    pub fn push_system_message(&mut self, text: &str) {
        self.messages.push(ChatMessage { role: "system".into(), content: text.to_string() });
        self.auto_scroll();
    }

    pub fn receive_model_response(&mut self, result: anyhow::Result<String>) {
        self.spinner_frame = 0;
        match result {
            Ok(answer) => {
                self.messages.push(ChatMessage { role: "assistant".into(), content: answer });
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
        self.scroll_chat = self.scroll_chat.saturating_sub(5);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_chat = self.scroll_chat.saturating_add(5);
    }

    pub fn tick_spinner(&mut self) {
        if self.status == AppStatus::Thinking {
            self.spinner_frame = (self.spinner_frame + 1) % 4;
        }
    }

    fn auto_scroll(&mut self) {
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
