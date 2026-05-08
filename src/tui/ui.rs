use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, AppStatus, Panel};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Title bar
            Constraint::Min(8),     // Main content
            Constraint::Length(3),  // Input bar
        ])
        .split(frame.area());

    draw_title_bar(frame, chunks[0], app);

    // Split main area into chat + context
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    draw_chat_panel(frame, main_chunks[0], app);
    draw_context_panel(frame, main_chunks[1], app);
    draw_input_bar(frame, chunks[2], app);
}

fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = match &app.status {
        AppStatus::Ready => "Ready".to_string(),
        AppStatus::Thinking => {
            let spinner = ["⠋", "⠙", "⠹", "⠸"][app.spinner_frame % 4];
            format!("{spinner} Thinking...")
        }
        AppStatus::Error => "Error".to_string(),
    };

    let msg_count = app.messages.len();
    let status_style = match app.status {
        AppStatus::Ready => Style::default().fg(Color::Green),
        AppStatus::Thinking => Style::default().fg(Color::Yellow),
        AppStatus::Error => Style::default().fg(Color::Red),
    };

    let title = Line::from(vec![
        Span::styled(
            " dumbcoder ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled(status_text, status_style),
        Span::raw(format!(" │ {msg_count} messages │ ")),
        Span::styled(
            "Ctrl+C: quit | Tab: switch | Enter: send | Ctrl+L: clear",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(title).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_chat_panel(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.active_panel == Panel::Chat {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut lines: Vec<Line> = Vec::new();

    if app.messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Type a question below and press Enter...",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for msg in &app.messages {
            let (prefix, style) = match msg.role.as_str() {
                "user" => ("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                "assistant" => ("< ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                _ => ("  ", Style::default()),
            };

            lines.push(Line::from(Span::styled(prefix, style)));

            for line in msg.content.lines() {
                lines.push(Line::from(Span::raw(format!("  {line}"))));
            }
            lines.push(Line::raw(""));
        }
    }

    if let Some(err) = &app.last_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Chat ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_chat, 0));

    frame.render_widget(paragraph, area);
}

fn draw_context_panel(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.active_panel == Panel::Context {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut lines: Vec<Line> = Vec::new();

    if !app.context_symbols.is_empty() {
        lines.push(Line::from(Span::styled(
            "── Symbols ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for sym in &app.context_symbols {
            let kind_str = sym.kind.as_str();
            lines.push(Line::from(vec![
                Span::styled(format!("  {kind_str} "), Style::default().fg(Color::Magenta)),
                Span::styled(&sym.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {}:{}", sym.path, sym.start_line), Style::default().fg(Color::DarkGray)),
            ]));
        }
        lines.push(Line::raw(""));
    }

    if !app.context_files.is_empty() {
        lines.push(Line::from(Span::styled(
            "── Referenced Files ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for fc in &app.context_files {
            lines.push(Line::from(Span::styled(
                format!("  ── {} ──", fc.path),
                Style::default().fg(Color::Blue),
            )));
            for line in fc.content.lines().take(30) {
                lines.push(Line::from(Span::raw(format!("  {line}"))));
            }
            lines.push(Line::raw(""));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Context will appear here after a query.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Context ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_context, 0));

    frame.render_widget(paragraph, area);
}

fn draw_input_bar(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.active_panel == Panel::Chat {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let cursor_char = if app.input_cursor < app.input.len() {
        &app.input[app.input_cursor..app.input_cursor + 1]
    } else {
        " "
    };

    let display_input = if app.input_cursor < app.input.len() {
        format!(
            "{}{}{}",
            &app.input[..app.input_cursor],
            cursor_char,
            &app.input[app.input_cursor + 1..]
        )
    } else {
        format!("{}▌", &app.input)
    };

    let hint = if app.status == AppStatus::Thinking {
        " (thinking...)"
    } else {
        " [Enter] Send"
    };

    let input_line = Line::from(vec![
        Span::styled(" > ", Style::default().fg(Color::Green)),
        Span::raw(&display_input),
        Span::styled(hint, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(input_line).block(block);
    frame.render_widget(paragraph, area);
}
