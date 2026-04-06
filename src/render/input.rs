use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use serde_json::Value;

use super::helpers::make_block;
use super::style::parse_style;

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or_default();

    let value = widget
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let placeholder = widget
        .get("placeholder")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let cursor = widget
        .get("cursor")
        .and_then(|v| v.as_u64())
        .unwrap_or(value.len() as u64) as usize;

    let line = if value.is_empty() && !focused {
        // Show placeholder when empty and not focused
        Line::from(Span::styled(
            placeholder.to_string(),
            Style::default().fg(Color::DarkGray),
        ))
    } else if focused {
        // Show text with cursor
        let cursor = cursor.min(value.len());
        let before = &value[..cursor];
        let cursor_char = value.get(cursor..cursor + 1).unwrap_or(" ");
        let after = if cursor < value.len() {
            &value[cursor + 1..]
        } else {
            ""
        };

        let cursor_style = Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD);

        Line::from(vec![
            Span::styled(before.to_string(), base_style),
            Span::styled(cursor_char.to_string(), cursor_style),
            Span::styled(after.to_string(), base_style),
        ])
    } else {
        Line::from(Span::styled(value.to_string(), base_style))
    };

    let mut p = Paragraph::new(line).style(base_style);

    if let Some(block) = make_block(widget, focused) {
        p = p.block(block);
    }

    f.render_widget(p, area);
}
