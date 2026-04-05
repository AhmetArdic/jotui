use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::Tabs,
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

    let titles: Vec<String> = widget
        .get("titles")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let selected = widget
        .get("selected")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let hl_style = widget
        .get("highlight_style")
        .map(|v| parse_style(v))
        .unwrap_or(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let divider = widget
        .get("divider")
        .and_then(|v| v.as_str())
        .unwrap_or(" | ")
        .to_string();

    let mut tabs = Tabs::new(titles)
        .select(selected)
        .style(base_style)
        .highlight_style(hl_style)
        .divider(Span::raw(divider));

    if let Some(block) = make_block(widget, focused) {
        tabs = tabs.block(block);
    }

    f.render_widget(tabs, area);
}
