use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppMode, AppStatus};

pub fn draw(frame: &mut Frame, app: &App) {
    let completion_height = if app.completions.is_empty() {
        0
    } else {
        std::cmp::min(app.completions.len() as u16 + 1, 6)
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

    if !app.completions.is_empty() {
        draw_completions(frame, chunks[2], app);
    }

    draw_input(frame, chunks[3], app);
}

fn draw_status_line(frame: &mut Frame, area: Rect, app: &App) {
    let (status_text, status_color) = match &app.status {
        AppStatus::Ready => ("Ready", Color::Green),
        AppStatus::Thinking => match app.spinner_frame % 4 {
            0 => ("⠋ Thinking", Color::Yellow),
            1 => ("⠙ Thinking", Color::Yellow),
            2 => ("⠹ Thinking", Color::Yellow),
            _ => ("⠸ Thinking", Color::Yellow),
        },
        AppStatus::Error => ("Error", Color::Red),
    };

    let mode_str = if app.mode == AppMode::Plan { " [PLAN]" } else { "" };
    let msgs = app.messages.len();

    let line = Line::from(vec![
        Span::styled(
            " dumbcoder ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(mode_str, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" │ "),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::raw(format!(" │ {msgs} msgs │ ")),
        Span::styled(
            "PgUp/Dn: scroll | ↑↓: history | /help",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn draw_chat(frame: &mut Frame, area: Rect, app: &App) {
    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let mut lines: Vec<Line> = Vec::new();

    if app.messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Type a message and press Enter. /help for commands.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    for msg in &app.messages {
        let (prefix, color) = match msg.role.as_str() {
            "user" => ("  ▸ ", Color::Green),
            "assistant" => ("  ◆ ", Color::Cyan),
            "system" => ("  ℹ ", Color::DarkGray),
            _ => ("    ", Color::White),
        };

        // First line with prefix
        let content_lines: Vec<&str> = msg.content.lines().collect();
        if let Some(first) = content_lines.first() {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::raw(*first),
            ]));
        }
        // Subsequent lines indented
        for line in content_lines.iter().skip(1) {
            lines.push(Line::from(Span::raw(format!("    {line}"))));
        }
        lines.push(Line::raw(""));
    }

    if let Some(err) = &app.last_error {
        lines.push(Line::from(Span::styled(
            format!("  ✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    // Clamp scroll: don't scroll past content
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
        let is_selected = i == app.completion_index % app.completions.len();
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let marker = if is_selected { "▸ " } else { "  " };
        lines.push(Line::from(Span::styled(format!("{marker}{cmd}"), style)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let border_color = if app.status == AppStatus::Thinking {
        Color::Yellow
    } else {
        Color::Cyan
    };

    // Build cursor display
    let (before, cursor_char, after) = if app.input_cursor < app.input.len() {
        (
            &app.input[..app.input_cursor],
            &app.input[app.input_cursor..app.input_cursor + 1],
            &app.input[app.input_cursor + 1..],
        )
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
