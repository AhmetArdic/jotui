pub mod style;
mod helpers;
mod paragraph;
mod list;
mod table;
mod tabs;
mod gauge;
mod sparkline;
mod bar_chart;

use ratatui::{layout::Rect, Frame};
use serde_json::Value;

pub fn render_widget(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let visible = widget
        .get("visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !visible {
        return;
    }

    let wtype = widget
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("paragraph");

    match wtype {
        "paragraph" => paragraph::render(f, area, widget, focused),
        "list" => list::render(f, area, widget, focused),
        "table" => table::render(f, area, widget, focused),
        "tabs" => tabs::render(f, area, widget, focused),
        "gauge" => gauge::render(f, area, widget, focused),
        "line_gauge" => gauge::render_line(f, area, widget, focused),
        "sparkline" => sparkline::render(f, area, widget, focused),
        "bar_chart" => bar_chart::render(f, area, widget, focused),
        _ => {
            eprintln!("[warn] unknown widget type: {}", wtype);
        }
    }
}
