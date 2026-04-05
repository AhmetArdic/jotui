use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Gauge, LineGauge},
    Frame,
};
use serde_json::Value;

use super::helpers::make_block;
use super::style::parse_style;

fn gauge_common(widget: &Value) -> (Style, f64, String) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or(Style::default().fg(Color::Green));

    let value = widget.get("value").and_then(|v| v.as_u64()).unwrap_or(0) as f64;
    let max = widget.get("max").and_then(|v| v.as_u64()).unwrap_or(100) as f64;
    let ratio = if max > 0.0 { (value / max).min(1.0) } else { 0.0 };

    let label = widget
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let label_display = if label.is_empty() {
        format!("{:.0}%", ratio * 100.0)
    } else {
        format!("{}: {:.0}%", label, ratio * 100.0)
    };

    (base_style, ratio, label_display)
}

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let (base_style, ratio, label_display) = gauge_common(widget);

    let mut gauge = Gauge::default()
        .gauge_style(base_style)
        .ratio(ratio)
        .label(Span::raw(label_display));

    if let Some(block) = make_block(widget, focused) {
        gauge = gauge.block(block);
    }

    f.render_widget(gauge, area);
}

pub fn render_line(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let (base_style, ratio, label_display) = gauge_common(widget);

    let mut lg = LineGauge::default()
        .filled_style(base_style)
        .ratio(ratio)
        .label(Span::raw(label_display));

    if let Some(block) = make_block(widget, focused) {
        lg = lg.block(block);
    }

    f.render_widget(lg, area);
}
