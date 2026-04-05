use crossterm::event::KeyCode;
use serde_json::Value;

use crate::state::AppState;
use crate::transport::{make_event, make_key_event, send_event};

pub fn handle_key(app: &mut AppState, code: KeyCode) {
    let active_page = app.active_page.clone();

    match code {
        KeyCode::Tab => {
            if let Some(page) = app.active_page_mut() {
                if !page.focus_ring.is_empty() {
                    let len = page.focus_ring.len();
                    let cur = page.focus_index.unwrap_or(0);
                    page.focus_index = Some((cur + 1) % len);
                }
            }
        }
        KeyCode::BackTab => {
            if let Some(page) = app.active_page_mut() {
                if !page.focus_ring.is_empty() {
                    let len = page.focus_ring.len();
                    let cur = page.focus_index.unwrap_or(0);
                    page.focus_index = Some((cur + len - 1) % len);
                }
            }
        }
        KeyCode::Up => {
            if let Some(page) = app.active_page_mut() {
                if let Some(focused_id) = page
                    .focus_index
                    .and_then(|i| page.focus_ring.get(i))
                    .cloned()
                {
                    if let Some(w) = page.widgets.get_mut(&focused_id) {
                        if let Some(sel) = w.get("selected").and_then(|v| v.as_u64()) {
                            if sel > 0 {
                                w.as_object_mut()
                                    .unwrap()
                                    .insert("selected".to_string(), (sel - 1).into());
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Down => {
            if let Some(page) = app.active_page_mut() {
                if let Some(focused_id) = page
                    .focus_index
                    .and_then(|i| page.focus_ring.get(i))
                    .cloned()
                {
                    if let Some(w) = page.widgets.get_mut(&focused_id) {
                        let wtype = w.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let max_idx = match wtype {
                            "list" => w
                                .get("items")
                                .and_then(|v| v.as_array())
                                .map(|a| a.len().saturating_sub(1) as u64),
                            "table" => w
                                .get("rows")
                                .and_then(|v| v.as_array())
                                .map(|a| a.len().saturating_sub(1) as u64),
                            _ => None,
                        };
                        let current = w.get("selected").and_then(|v| v.as_u64()).unwrap_or(0);
                        if let Some(max) = max_idx {
                            let next = (current + 1).min(max);
                            w.as_object_mut()
                                .unwrap()
                                .insert("selected".to_string(), next.into());
                        }
                    }
                }
            }
        }
        KeyCode::Left => {
            if let Some(page) = app.active_page_mut() {
                if let Some(focused_id) = page
                    .focus_index
                    .and_then(|i| page.focus_ring.get(i))
                    .cloned()
                {
                    if let Some(w) = page.widgets.get_mut(&focused_id) {
                        let wtype = w.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if wtype == "tabs" {
                            let sel = w.get("selected").and_then(|v| v.as_u64()).unwrap_or(0);
                            if sel > 0 {
                                w.as_object_mut()
                                    .unwrap()
                                    .insert("selected".to_string(), (sel - 1).into());
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Right => {
            if let Some(page) = app.active_page_mut() {
                if let Some(focused_id) = page
                    .focus_index
                    .and_then(|i| page.focus_ring.get(i))
                    .cloned()
                {
                    if let Some(w) = page.widgets.get_mut(&focused_id) {
                        let wtype = w.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if wtype == "tabs" {
                            let count = w
                                .get("titles")
                                .and_then(|v| v.as_array())
                                .map(|a| a.len() as u64)
                                .unwrap_or(0);
                            let sel = w.get("selected").and_then(|v| v.as_u64()).unwrap_or(0);
                            if sel + 1 < count {
                                w.as_object_mut()
                                    .unwrap()
                                    .insert("selected".to_string(), (sel + 1).into());
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Enter => {
            let action_info = app.active_page().and_then(|page| {
                let focused_id = page
                    .focus_index
                    .and_then(|i| page.focus_ring.get(i))?
                    .clone();
                let w = page.widgets.get(&focused_id)?;
                let wtype = w
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let sel = w.get("selected").and_then(|v| v.as_u64()).unwrap_or(0);
                Some((focused_id, wtype, sel))
            });

            if let Some((focused_id, wtype, sel)) = action_info {
                match wtype.as_str() {
                    "tabs" => {
                        if let Some(target) = app.page_order.get(sel as usize).cloned() {
                            app.active_page = target;
                        }
                    }
                    "list" | "table" => {
                        send_event(&make_event(
                            &active_page,
                            Some(&focused_id),
                            "select",
                            Value::from(sel),
                        ));
                    }
                    _ => {
                        send_event(&make_event(
                            &active_page,
                            Some(&focused_id),
                            "submit",
                            Value::Null,
                        ));
                    }
                }
            }
        }
        code => {
            let key_name = match code {
                KeyCode::F(n) => format!("f{}", n),
                KeyCode::Char(c) => format!("{}", c),
                KeyCode::Esc => "esc".to_string(),
                KeyCode::Backspace => "backspace".to_string(),
                KeyCode::Delete => "delete".to_string(),
                KeyCode::Home => "home".to_string(),
                KeyCode::End => "end".to_string(),
                KeyCode::PageUp => "pageup".to_string(),
                KeyCode::PageDown => "pagedown".to_string(),
                KeyCode::Insert => "insert".to_string(),
                _ => return,
            };

            let source = app
                .active_page()
                .and_then(|p| p.focus_index.and_then(|i| p.focus_ring.get(i)))
                .cloned();

            send_event(&make_key_event(&active_page, source.as_deref(), &key_name));
        }
    }
}
