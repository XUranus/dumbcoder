use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Render markdown text into styled ratatui Lines.
/// Supports: headers (#), bold (**), code blocks (```), inline code (`), bullet lists (-).
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in text.lines() {
        // Code block fence
        if line.trim_start().starts_with("```") {
            if in_code_block {
                in_code_block = false;
                lines.push(Line::from(Span::styled(
                    "  ─────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                in_code_block = true;
                lines.push(Line::from(Span::styled(
                    "  ┌────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("  │ {line}"),
                Style::default().fg(Color::LightGreen),
            )));
            continue;
        }

        // Header
        if line.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                format!("  {}", &line[2..]),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if line.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                format!("  {}", &line[3..]),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if line.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                format!("  {}", &line[4..]),
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        // Bullet list
        if line.starts_with("- ") || line.starts_with("* ") {
            let mut spans = vec![
                Span::styled("  • ", Style::default().fg(Color::Yellow)),
            ];
            spans.extend(render_inline(line[2..].trim()));
            lines.push(Line::from(spans));
            continue;
        }

        // Numbered list
        if line.len() > 2
            && line.as_bytes()[0].is_ascii_digit()
            && line.as_bytes()[1] == b'.'
            && line.as_bytes().get(2) == Some(&b' ')
        {
            let num = &line[..2];
            let mut spans = vec![
                Span::styled(format!("  {num} "), Style::default().fg(Color::Yellow)),
            ];
            spans.extend(render_inline(line[3..].trim()));
            lines.push(Line::from(spans));
            continue;
        }

        // Separator
        if line.trim() == "---" || line.trim() == "***" {
            lines.push(Line::from(Span::styled(
                "  ──────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        // Empty line
        if line.trim().is_empty() {
            lines.push(Line::raw(""));
            continue;
        }

        // Normal text with inline formatting
        let mut spans = vec![Span::raw("  ")];
        spans.extend(render_inline(line.trim()));
        lines.push(Line::from(spans));
    }

    lines
}

/// Render inline markdown: **bold**, `code`, normal text.
fn render_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut buf = String::new();

    while i < len {
        // Inline code
        if bytes[i] == b'`' && !buf.is_empty() {
            // flush buf as normal
            spans.push(Span::raw(std::mem::take(&mut buf)));
        }
        if bytes[i] == b'`' {
            // find closing `
            if let Some(end) = text[i + 1..].find('`') {
                let code = &text[i + 1..i + 1 + end];
                spans.push(Span::styled(
                    format!(" {code} "),
                    Style::default().fg(Color::LightGreen).bg(Color::DarkGray),
                ));
                i += end + 2;
                continue;
            }
        }

        // Bold **text**
        if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if !buf.is_empty() {
                spans.push(Span::raw(std::mem::take(&mut buf)));
            }
            if let Some(end) = text[i + 2..].find("**") {
                let bold_text = &text[i + 2..i + 2 + end];
                spans.push(Span::styled(
                    bold_text.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                i += end + 4;
                continue;
            }
        }

        buf.push(bytes[i] as char);
        i += 1;
    }

    if !buf.is_empty() {
        spans.push(Span::raw(buf));
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }

    spans
}
