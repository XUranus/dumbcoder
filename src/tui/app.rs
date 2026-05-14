use crate::context::FileContent;
use crate::index::SymbolInfo;
use crate::model::ChatMessage;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Chat,
    Context,
}

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
    SwitchPanel,
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
    pub context_files: Vec<FileContent>,
    pub context_symbols: Vec<SymbolInfo>,
    pub status: AppStatus,
    pub scroll_chat: u16,
    pub scroll_context: u16,
    pub active_panel: Panel,
    pub last_error: Option<String>,
    pub spinner_frame: usize,
    // New fields
    pub mode: AppMode,
    pub plan_content: Option<String>,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub session_id: Option<String>,
    pub completions: Vec<String>,
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
            context_files: Vec::new(),
            context_symbols: Vec::new(),
            status: AppStatus::Ready,
            scroll_chat: 0,
            scroll_context: 0,
            active_panel: Panel::Chat,
            last_error: None,
            spinner_frame: 0,
            mode: AppMode::Chat,
            plan_content: None,
            history: Vec::new(),
            history_index: None,
            session_id: None,
            completions: Vec::new(),
        }
    }

    pub fn update_completions(&mut self) {
        self.completions.clear();
        if !self.input.starts_with('/') {
            return;
        }
        let input_lower = self.input.to_lowercase();
        for &cmd in ALL_COMMANDS {
            if cmd.starts_with(input_lower.as_str()) || input_lower.starts_with(cmd) {
                continue; // skip exact match
            }
            if cmd.contains(&input_lower) || cmd.starts_with(&input_lower) {
                self.completions.push(cmd.to_string());
            }
        }
        // If input is exactly "/" show all
        if self.input == "/" {
            self.completions = ALL_COMMANDS.iter().map(|s| s.to_string()).collect();
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> AppAction {
        // Global quit
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AppAction::Quit;
        }
        if key.code == KeyCode::Esc {
            if self.input.is_empty() {
                return AppAction::Quit;
            }
            return AppAction::SwitchPanel;
        }

        if key.code == KeyCode::Tab {
            // Tab completion for slash commands
            if self.input.starts_with('/') && !self.completions.is_empty() {
                self.input = self.completions[0].clone();
                self.input_cursor = self.input.len();
                self.update_completions();
                return AppAction::None;
            }
            return AppAction::SwitchPanel;
        }

        if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AppAction::ClearChat;
        }

        // Up/Down for history when input is active and chat panel
        if self.active_panel == Panel::Chat && !self.history.is_empty() {
            if key.code == KeyCode::Up && self.status != AppStatus::Thinking {
                return self.history_prev();
            }
            if key.code == KeyCode::Down && self.status != AppStatus::Thinking {
                return self.history_next();
            }
        }

        // Scroll in context panel
        if self.active_panel == Panel::Context {
            match key.code {
                KeyCode::Up => return AppAction::ScrollUp,
                KeyCode::Down => return AppAction::ScrollDown,
                KeyCode::PageUp => return AppAction::ScrollPageUp,
                KeyCode::PageDown => return AppAction::ScrollPageDown,
                _ => {}
            }
        }

        // Input handling
        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty() && self.status != AppStatus::Thinking {
                    return AppAction::Send;
                }
            }
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
            KeyCode::Home => {
                self.input_cursor = 0;
            }
            KeyCode::End => {
                self.input_cursor = self.input.len();
            }
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
        // Save to history
        if !text.is_empty() {
            self.history.push(text.clone());
            if self.history.len() > 100 {
                self.history.remove(0);
            }
        }
        self.history_index = None;

        self.messages.push(ChatMessage {
            role: "user".into(),
            content: text,
        });
    }

    pub fn push_system_message(&mut self, text: &str) {
        self.messages.push(ChatMessage {
            role: "system".into(),
            content: text.to_string(),
        });
        self.scroll_chat = self.messages.len() as u16 * 3;
    }

    pub fn receive_model_response(&mut self, result: anyhow::Result<ModelResponse>) {
        self.spinner_frame = 0;
        match result {
            Ok(resp) => {
                self.messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: resp.answer,
                });
                self.context_files = resp.context_files;
                self.context_symbols = resp.context_symbols;
                self.status = AppStatus::Ready;
                self.scroll_chat = self.messages.len() as u16 * 3;
            }
            Err(e) => {
                self.status = AppStatus::Error;
                self.last_error = Some(e.to_string());
            }
        }
    }

    pub fn clear_chat(&mut self) {
        self.messages.clear();
        self.context_files.clear();
        self.context_symbols.clear();
        self.scroll_chat = 0;
        self.scroll_context = 0;
        self.last_error = None;
        self.status = AppStatus::Ready;
        self.plan_content = None;
        self.mode = AppMode::Chat;
    }

    pub fn scroll_chat_up(&mut self) {
        if self.scroll_chat > 0 {
            self.scroll_chat -= 1;
        }
    }

    pub fn scroll_chat_down(&mut self) {
        self.scroll_chat += 1;
    }

    pub fn scroll_context_up(&mut self) {
        if self.scroll_context > 0 {
            self.scroll_context -= 1;
        }
    }

    pub fn scroll_context_down(&mut self) {
        self.scroll_context += 1;
    }

    pub fn tick_spinner(&mut self) {
        if self.status == AppStatus::Thinking {
            self.spinner_frame = (self.spinner_frame + 1) % 4;
        }
    }

    fn history_prev(&mut self) -> AppAction {
        if self.history.is_empty() {
            return AppAction::None;
        }
        let idx = match self.history_index {
            Some(i) => {
                if i > 0 { i - 1 } else { 0 }
            }
            None => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        self.input = self.history[idx].clone();
        self.input_cursor = self.input.len();
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
        AppAction::None
    }
}
