use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Sparkline,
    Frame,
};
use serde_json::Value;

use super::helpers::make_block;
use super::style::parse_style;

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or(Style::default().fg(Color::Green));

    let data: Vec<u64> = widget
        .get("data")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();

    let mut spark = Sparkline::default().data(&data).style(base_style);

    if let Some(max) = widget.get("max").and_then(|v| v.as_u64()) {
        spark = spark.max(max);
    }

    if let Some(block) = make_block(widget, focused) {
        spark = spark.block(block);
    }

    f.render_widget(spark, area);
}
