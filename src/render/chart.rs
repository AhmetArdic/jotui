use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType},
    Frame,
};
use serde_json::Value;

use super::style::parse_style;

pub fn render(f: &mut Frame, area: Rect, widget: &Value, focused: bool) {
    let base_style = widget
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or_default();

    // Parse datasets
    let datasets_val = widget
        .get("datasets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // We need to store the data outside the closure so references stay valid
    let mut dataset_data: Vec<(String, Vec<(f64, f64)>, Style, symbols::Marker, GraphType)> =
        Vec::new();

    for ds in &datasets_val {
        let name = ds
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let data: Vec<(f64, f64)> = ds
            .get("data")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|point| {
                        let pair = point.as_array()?;
                        Some((pair.first()?.as_f64()?, pair.get(1)?.as_f64()?))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let style = ds
            .get("style")
            .map(|v| parse_style(v))
            .unwrap_or(Style::default().fg(Color::Cyan));

        let marker = match ds.get("marker").and_then(|v| v.as_str()).unwrap_or("braille") {
            "dot" => symbols::Marker::Dot,
            "block" => symbols::Marker::Block,
            "bar" => symbols::Marker::Bar,
            "halfblock" => symbols::Marker::HalfBlock,
            _ => symbols::Marker::Braille,
        };

        let graph_type =
            match ds.get("graph_type").and_then(|v| v.as_str()).unwrap_or("line") {
                "scatter" => GraphType::Scatter,
                _ => GraphType::Line,
            };

        dataset_data.push((name, data, style, marker, graph_type));
    }

    let datasets: Vec<Dataset> = dataset_data
        .iter()
        .map(|(name, data, style, marker, graph_type)| {
            Dataset::default()
                .name(name.clone())
                .data(data)
                .style(*style)
                .marker(*marker)
                .graph_type(*graph_type)
        })
        .collect();

    // Parse axes
    let x_axis_val = widget
        .get("x_axis")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let y_axis_val = widget
        .get("y_axis")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let x_title = x_axis_val
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let x_bounds = parse_bounds(&x_axis_val);
    let x_labels = parse_axis_labels(&x_axis_val, x_bounds);

    let y_title = y_axis_val
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let y_bounds = parse_bounds(&y_axis_val);
    let y_labels = parse_axis_labels(&y_axis_val, y_bounds);

    let x_axis_style = x_axis_val
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or(base_style);
    let y_axis_style = y_axis_val
        .get("style")
        .map(|v| parse_style(v))
        .unwrap_or(base_style);

    let x_axis = Axis::default()
        .title(x_title)
        .style(x_axis_style)
        .bounds(x_bounds)
        .labels(x_labels);

    let y_axis = Axis::default()
        .title(y_title)
        .style(y_axis_style)
        .bounds(y_bounds)
        .labels(y_labels);

    // Build chart
    let mut chart = Chart::new(datasets)
        .x_axis(x_axis)
        .y_axis(y_axis)
        .style(base_style);

    // Block / border
    let border_str = widget
        .get("border")
        .and_then(|v| v.as_str())
        .unwrap_or("rounded");

    if border_str != "none" {
        let border_type = match border_str {
            "plain" => BorderType::Plain,
            "double" => BorderType::Double,
            "thick" => BorderType::Thick,
            _ => BorderType::Rounded,
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
            block = block.border_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );
        }

        chart = chart.block(block);
    }

    f.render_widget(chart, area);
}

fn parse_bounds(axis: &Value) -> [f64; 2] {
    axis.get("bounds")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            Some([arr.first()?.as_f64()?, arr.get(1)?.as_f64()?])
        })
        .unwrap_or([0.0, 100.0])
}

fn parse_axis_labels(axis: &Value, bounds: [f64; 2]) -> Vec<String> {
    if let Some(labels) = axis.get("labels").and_then(|v| v.as_array()) {
        labels
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    } else {
        // Auto-generate min/mid/max labels from bounds
        let min = bounds[0];
        let max = bounds[1];
        let mid = (min + max) / 2.0;
        vec![
            format_number(min),
            format_number(mid),
            format_number(max),
        ]
    }
}

fn format_number(n: f64) -> String {
    if n == n.floor() {
        format!("{}", n as i64)
    } else {
        format!("{:.1}", n)
    }
}
