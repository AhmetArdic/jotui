use ratatui::style::{Color, Modifier, Style};
use serde_json::Value;

pub fn parse_color(s: &str) -> Color {
    let s = s.trim();
    match s.to_lowercase().as_str() {
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" => Color::DarkGray,
        "white" => Color::White,
        "black" => Color::Black,
        "reset" => Color::Reset,
        _ => {
            if let Some(hex) = s.strip_prefix('#') {
                if hex.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        return Color::Rgb(r, g, b);
                    }
                }
            }
            if let Some(inner) = s.strip_prefix("color(").and_then(|s| s.strip_suffix(')')) {
                if let Ok(n) = inner.trim().parse::<u8>() {
                    return Color::Indexed(n);
                }
            }
            Color::Reset
        }
    }
}

pub fn parse_style(val: &Value) -> Style {
    let mut style = Style::default();
    if let Some(fg) = val.get("fg").and_then(|v| v.as_str()) {
        style = style.fg(parse_color(fg));
    }
    if let Some(bg) = val.get("bg").and_then(|v| v.as_str()) {
        style = style.bg(parse_color(bg));
    }
    let mut mods = Modifier::empty();
    if val.get("bold").and_then(|v| v.as_bool()).unwrap_or(false) {
        mods |= Modifier::BOLD;
    }
    if val.get("italic").and_then(|v| v.as_bool()).unwrap_or(false) {
        mods |= Modifier::ITALIC;
    }
    if val.get("underline").and_then(|v| v.as_bool()).unwrap_or(false) {
        mods |= Modifier::UNDERLINED;
    }
    style.add_modifier(mods)
}
