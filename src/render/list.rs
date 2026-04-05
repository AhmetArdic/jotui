use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{List, ListItem, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use serde_json::Value;

use super::helpers::{make_block, parse_text_spans};
use super::style::parse_style;

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or_default();

    let items: Vec<ListItem<'static>> = widget
        .get("items")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| {
                    let line = parse_text_spans(item, base_style);
                    ListItem::new(line)
                })
                .collect()
        })
        .unwrap_or_default();

    let hl_symbol = widget
        .get("highlight_symbol")
        .and_then(|v| v.as_str())
        .unwrap_or(">> ")
        .to_string();

    let hl_style = widget
        .get("highlight_style")
        .map(|v| parse_style(v))
        .unwrap_or(Style::default().fg(Color::Black).bg(Color::White));

    let mut list = List::new(items)
        .style(base_style)
        .highlight_symbol(&hl_symbol)
        .highlight_style(hl_style);

    if let Some(block) = make_block(widget, focused) {
        list = list.block(block);
    }

    let selected = widget.get("selected").and_then(|v| v.as_u64()).map(|v| v as usize);
    let mut state = ListState::default().with_selected(selected);

    f.render_stateful_widget(list, area, &mut state);

    if widget.get("scrollbar").and_then(|v| v.as_bool()).unwrap_or(false) {
        let item_count = widget
            .get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if item_count > 0 {
            let mut sb_state = ScrollbarState::new(item_count).position(selected.unwrap_or(0));
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut sb_state,
            );
        }
    }
}
