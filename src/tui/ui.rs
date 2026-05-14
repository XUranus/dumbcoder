use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppMode, AppStatus};
use super::markdown;

pub fn draw(frame: &mut Frame, app: &App) {
    let completion_height = if app.completion_active && !app.completions.is_empty() {
        std::cmp::min(app.completions.len() as u16 + 1, 8)
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                  // Status line
            Constraint::Min(5),                     // Chat area
            Constraint::Length(completion_height),   // Completions popup
            Constraint::Length(3),                   // Input bar
        ])
        .split(frame.area());

    draw_status_line(frame, chunks[0], app);
    draw_chat(frame, chunks[1], app);

    if app.completion_active && !app.completions.is_empty() {
        draw_completions(frame, chunks[2], app);
    }

    draw_input(frame, chunks[3], app);
}

fn draw_status_line(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = match &app.status {
        AppStatus::Ready => "Ready",
        AppStatus::Thinking => match app.spinner_frame % 4 {
            0 => "⠋ Thinking",
            1 => "⠙ Thinking",
            2 => "⠹ Thinking",
            _ => "⠸ Thinking",
        },
        AppStatus::Error => "Error",
    };
    let status_color = match &app.status {
        AppStatus::Ready => Color::Green,
        AppStatus::Thinking => Color::Yellow,
        AppStatus::Error => Color::Red,
    };

    let mode_str = if app.mode == AppMode::Plan { " [PLAN]" } else { "" };
    let msgs = app.messages.len();

    let line = Line::from(vec![
        Span::styled(" dumbcoder ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(mode_str, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" │ "),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::raw(format!(" │ {msgs} msgs │ ")),
        Span::styled("PgUp/Dn: scroll │ ↑↓: history │ /help", Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn draw_chat(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();

    if app.messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Type a message and press Enter. /help for commands.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    for msg in &app.messages {
        match msg.role.as_str() {
            "user" => {
                lines.push(Line::from(vec![
                    Span::styled("  ▸ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(&msg.content, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]));
                lines.push(Line::raw(""));
            }
            "assistant" => {
                // Markdown rendering for assistant messages
                let md_lines = markdown::render_markdown(&msg.content);
                lines.extend(md_lines);
                lines.push(Line::raw(""));
            }
            "system" => {
                for line in msg.content.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("  ℹ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(line, Style::default().fg(Color::DarkGray)),
                    ]));
                }
                lines.push(Line::raw(""));
            }
            _ => {
                for line in msg.content.lines() {
                    lines.push(Line::raw(format!("    {line}")));
                }
            }
        }
    }

    if let Some(err) = &app.last_error {
        lines.push(Line::from(Span::styled(format!("  ✗ {err}"), Style::default().fg(Color::Red))));
    }

    let total_lines = lines.len();
    let max_scroll = total_lines.saturating_sub(inner_height) as u16;
    let scroll = std::cmp::min(app.scroll_chat, max_scroll);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_completions(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    for (i, cmd) in app.completions.iter().enumerate() {
        let is_selected = i == app.completion_index;
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let marker = if is_selected { " ▸ " } else { "   " };
        lines.push(Line::from(Span::styled(format!("{marker}{cmd}"), style)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let border_color = match &app.status {
        AppStatus::Thinking => Color::Yellow,
        _ => Color::Cyan,
    };

    let (before, cursor_char, after) = if app.input_cursor < app.input.len() {
        (&app.input[..app.input_cursor], &app.input[app.input_cursor..app.input_cursor + 1], &app.input[app.input_cursor + 1..])
    } else {
        (app.input.as_str(), "▌", "")
    };

    let hint = match (&app.status, &app.mode) {
        (AppStatus::Thinking, _) => " thinking...",
        (_, AppMode::Plan) => " /approve | /cancel",
        _ => " Enter: send",
    };

    let input_line = Line::from(vec![
        Span::styled(" ▸ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(before),
        Span::styled(cursor_char, Style::default().fg(Color::White).add_modifier(Modifier::REVERSED)),
        Span::raw(after),
        Span::styled(hint, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    frame.render_widget(Paragraph::new(input_line).block(block), area);
}
