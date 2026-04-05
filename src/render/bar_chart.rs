use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Bar, BarChart, BarGroup},
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

    let bar_width = widget
        .get("bar_width")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as u16;

    let max_val = widget.get("max").and_then(|v| v.as_u64());

    let bars_data: Vec<(String, u64)> = widget
        .get("bars")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let tuple = item.as_array()?;
                    let label = tuple.first()?.as_str()?.to_string();
                    let value = tuple.get(1)?.as_u64()?;
                    Some((label, value))
                })
                .collect()
        })
        .unwrap_or_default();

    let bars: Vec<Bar> = bars_data
        .iter()
        .map(|(label, value)| {
            Bar::default()
                .label(Line::from(label.clone()))
                .value(*value)
        })
        .collect();

    let group = BarGroup::default().bars(&bars);

    let mut chart = BarChart::default()
        .data(group)
        .bar_width(bar_width)
        .style(base_style);

    if let Some(m) = max_val {
        chart = chart.max(m);
    }

    if let Some(block) = make_block(widget, focused) {
        chart = chart.block(block);
    }

    f.render_widget(chart, area);
}
