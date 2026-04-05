use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use serde_json::Value;

pub fn parse_constraint(s: &str) -> Constraint {
    let s = s.trim();
    if s == "*" {
        Constraint::Min(0)
    } else if let Some(pct) = s.strip_suffix('%') {
        Constraint::Percentage(pct.parse().unwrap_or(50))
    } else if let Some(min) = s.strip_prefix('>') {
        Constraint::Min(min.parse().unwrap_or(0))
    } else if let Some(max) = s.strip_prefix('<') {
        Constraint::Max(max.parse().unwrap_or(100))
    } else {
        Constraint::Length(s.parse().unwrap_or(1))
    }
}

pub enum LayoutNode {
    Container {
        dir: Direction,
        size: Constraint,
        margin: u16,
        children: Vec<LayoutNode>,
    },
    Leaf {
        ref_id: String,
        size: Constraint,
    },
}

pub fn parse_layout(node: &Value) -> LayoutNode {
    let size = node
        .get("size")
        .and_then(|v| v.as_str())
        .unwrap_or("*");
    let constraint = parse_constraint(size);

    if let Some(ref_id) = node.get("ref").and_then(|v| v.as_str()) {
        return LayoutNode::Leaf {
            ref_id: ref_id.to_string(),
            size: constraint,
        };
    }

    let dir = match node.get("dir").and_then(|v| v.as_str()).unwrap_or("v") {
        "h" => Direction::Horizontal,
        _ => Direction::Vertical,
    };

    let margin = node
        .get("margin")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;

    let children = node
        .get("children")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(parse_layout).collect())
        .unwrap_or_default();

    LayoutNode::Container {
        dir,
        size: constraint,
        margin,
        children,
    }
}

/// Recursively resolve layout tree into a map of ref_id -> Rect
pub fn resolve_layout(
    node: &LayoutNode,
    area: Rect,
    result: &mut Vec<(String, Rect)>,
) {
    match node {
        LayoutNode::Leaf { ref_id, .. } => {
            result.push((ref_id.clone(), area));
        }
        LayoutNode::Container {
            dir,
            margin,
            children,
            ..
        } => {
            let inner = if *margin > 0 {
                area.inner(Margin {
                    horizontal: *margin,
                    vertical: *margin,
                })
            } else {
                area
            };

            if children.is_empty() {
                return;
            }

            let constraints: Vec<Constraint> = children
                .iter()
                .map(|c| match c {
                    LayoutNode::Leaf { size, .. } => *size,
                    LayoutNode::Container { size, .. } => *size,
                })
                .collect();

            let chunks = Layout::default()
                .direction(*dir)
                .constraints(constraints)
                .split(inner);

            for (i, child) in children.iter().enumerate() {
                if i < chunks.len() {
                    resolve_layout(child, chunks[i], result);
                }
            }
        }
    }
}
