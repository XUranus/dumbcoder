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
}

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

        // Tab to switch panel
        if key.code == KeyCode::Tab {
            return AppAction::SwitchPanel;
        }

        // Ctrl+L to clear chat
        if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AppAction::ClearChat;
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

        AppAction::None
    }

    pub fn push_user_message(&mut self, text: String) {
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: text,
        });
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
                // Auto-scroll to bottom
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
}
