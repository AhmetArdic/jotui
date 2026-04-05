use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Row, Table, TableState},
    Frame,
};
use serde_json::Value;

use super::helpers::make_block;
use super::style::parse_style;
use crate::layout::parse_constraint;

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or_default();

    let header_style = widget
        .get("header_style")
        .map(|v| parse_style(v))
        .unwrap_or(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let headers: Vec<String> = widget
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let header_row = Row::new(headers.clone()).style(header_style);

    let rows: Vec<Row<'static>> = widget
        .get("rows")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|row| {
                    let cells: Vec<String> = row
                        .as_array()
                        .map(|r| {
                            r.iter()
                                .filter_map(|c| c.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    Row::new(cells)
                })
                .collect()
        })
        .unwrap_or_default();

    let widths: Vec<Constraint> = widget
        .get("widths")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(parse_constraint))
                .collect()
        })
        .unwrap_or_else(|| {
            if headers.is_empty() {
                vec![]
            } else {
                let pct = 100 / headers.len().max(1) as u16;
                headers.iter().map(|_| Constraint::Percentage(pct)).collect()
            }
        });

    let hl_style = widget
        .get("highlight_style")
        .map(|v| parse_style(v))
        .unwrap_or(Style::default().fg(Color::Black).bg(Color::White));

    let mut table = Table::new(rows, &widths)
        .header(header_row)
        .style(base_style)
        .row_highlight_style(hl_style);

    if let Some(block) = make_block(widget, focused) {
        table = table.block(block);
    }

    let selected = widget.get("selected").and_then(|v| v.as_u64()).map(|v| v as usize);
    let mut state = TableState::default().with_selected(selected);

    f.render_stateful_widget(table, area, &mut state);
}
