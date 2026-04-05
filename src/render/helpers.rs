use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};
use serde_json::Value;

use super::style::parse_style;

pub fn make_block(widget: &Value, focused: bool) -> Option<Block<'static>> {
    let border_str = widget
        .get("border")
        .and_then(|v| v.as_str())
        .unwrap_or("none");

    if border_str == "none" {
        return None;
    }

    let border_type = match border_str {
        "plain" => BorderType::Plain,
        "rounded" => BorderType::Rounded,
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        _ => BorderType::Plain,
    };

    let title = widget
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type);

    if !title.is_empty() {
        block = block.title(title);
    }

    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    }

    Some(block)
}

pub fn parse_text_spans(val: &Value, default_style: Style) -> Line<'static> {
    match val {
        Value::String(s) => Line::from(Span::styled(s.clone(), default_style)),
        Value::Array(arr) => {
            let spans: Vec<Span<'static>> = arr
                .iter()
                .map(|item| match item {
                    Value::String(s) => Span::styled(s.clone(), default_style),
                    Value::Object(_) => {
                        let text = item
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let style = parse_style(item);
                        Span::styled(text, style)
                    }
                    _ => Span::raw(""),
                })
                .collect();
            Line::from(spans)
        }
        _ => Line::from(""),
    }
}
