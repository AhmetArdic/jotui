use ratatui::{layout::Rect, text::{Line, Span}, widgets::{Paragraph, Wrap}, Frame};
use serde_json::Value;

use super::helpers::{make_block, parse_text_spans};
use super::style::parse_style;

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or_default();

    let text_val = widget.get("text").cloned().unwrap_or(Value::String(String::new()));

    let lines: Vec<Line<'static>> = match &text_val {
        Value::String(s) => s
            .split('\n')
            .map(|line| Line::from(Span::styled(line.to_string(), base_style)))
            .collect(),
        Value::Array(_) => vec![parse_text_spans(&text_val, base_style)],
        _ => vec![Line::from("")],
    };

    let align = match widget.get("align").and_then(|v| v.as_str()).unwrap_or("left") {
        "center" => ratatui::layout::Alignment::Center,
        "right" => ratatui::layout::Alignment::Right,
        _ => ratatui::layout::Alignment::Left,
    };

    let wrap = widget
        .get("wrap")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut p = Paragraph::new(lines).alignment(align).style(base_style);
    if wrap {
        p = p.wrap(Wrap { trim: true });
    }

    if let Some(block) = make_block(widget, focused) {
        p = p.block(block);
    }

    f.render_widget(p, area);
}
