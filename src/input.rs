use crossterm::event::KeyCode;
use serde_json::Value;

use crate::state::AppState;
use crate::transport::{make_event, make_key_event, send_event};

/// Returns the widget type of the currently focused widget, if any.
fn focused_widget_type(app: &AppState) -> Option<String> {
    let page = app.active_page()?;
    let focused_id = page.focus_index.and_then(|i| page.focus_ring.get(i))?;
    let w = page.widgets.get(focused_id)?;
    w.get("type").and_then(|v| v.as_str()).map(String::from)
}

pub fn handle_key(app: &mut AppState, code: KeyCode) {
    let active_page = app.active_page.clone();

    // Check if focused widget is an input — if so, handle text editing
    if let Some(wtype) = focused_widget_type(app) {
        if wtype == "input" {
            match code {
                KeyCode::Char(c) => {
                    handle_input_char(app, c);
                    return;
                }
                KeyCode::Backspace => {
                    handle_input_backspace(app);
                    return;
                }
                KeyCode::Delete => {
                    handle_input_delete(app);
                    return;
                }
                KeyCode::Left => {
                    handle_input_cursor_left(app);
                    return;
                }
                KeyCode::Right => {
                    handle_input_cursor_right(app);
                    return;
                }
                KeyCode::Home => {
                    handle_input_cursor_home(app);
                    return;
                }
                KeyCode::End => {
                    handle_input_cursor_end(app);
                    return;
                }
                KeyCode::Enter => {
                    // Submit the input value
                    let value = get_focused_input_value(app);
                    let source = get_focused_id(app);
                    if let Some(source) = source {
                        send_event(&make_event(
                            &active_page,
                            Some(&source),
                            "submit",
                            Value::String(value),
                        ));
                    }
                    return;
                }
                // Tab, BackTab, etc. fall through to normal handling
                _ => {}
            }
        }
    }

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

// --- Input widget helpers ---

fn get_focused_id(app: &AppState) -> Option<String> {
    let page = app.active_page()?;
    page.focus_index
        .and_then(|i| page.focus_ring.get(i))
        .cloned()
}

fn get_focused_input_value(app: &AppState) -> String {
    app.active_page()
        .and_then(|page| {
            let focused_id = page.focus_index.and_then(|i| page.focus_ring.get(i))?;
            let w = page.widgets.get(focused_id)?;
            w.get("value").and_then(|v| v.as_str()).map(String::from)
        })
        .unwrap_or_default()
}

fn with_focused_input(app: &mut AppState, f: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    if let Some(page) = app.active_page_mut() {
        if let Some(focused_id) = page
            .focus_index
            .and_then(|i| page.focus_ring.get(i))
            .cloned()
        {
            if let Some(w) = page.widgets.get_mut(&focused_id) {
                if let Some(obj) = w.as_object_mut() {
                    f(obj);
                }
            }
        }
    }
}

fn handle_input_char(app: &mut AppState, c: char) {
    with_focused_input(app, |obj| {
        let value = obj
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cursor = obj
            .get("cursor")
            .and_then(|v| v.as_u64())
            .unwrap_or(value.len() as u64) as usize;
        let cursor = cursor.min(value.len());

        let mut new_value = value;
        new_value.insert(cursor, c);
        let new_cursor = cursor + 1;

        obj.insert("value".to_string(), Value::String(new_value));
        obj.insert("cursor".to_string(), Value::from(new_cursor as u64));
    });
}

fn handle_input_backspace(app: &mut AppState) {
    with_focused_input(app, |obj| {
        let value = obj
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cursor = obj
            .get("cursor")
            .and_then(|v| v.as_u64())
            .unwrap_or(value.len() as u64) as usize;
        let cursor = cursor.min(value.len());

        if cursor > 0 {
            let mut new_value = value;
            new_value.remove(cursor - 1);
            let new_cursor = cursor - 1;
            obj.insert("value".to_string(), Value::String(new_value));
            obj.insert("cursor".to_string(), Value::from(new_cursor as u64));
        }
    });
}

fn handle_input_delete(app: &mut AppState) {
    with_focused_input(app, |obj| {
        let value = obj
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cursor = obj
            .get("cursor")
            .and_then(|v| v.as_u64())
            .unwrap_or(value.len() as u64) as usize;
        let cursor = cursor.min(value.len());

        if cursor < value.len() {
            let mut new_value = value;
            new_value.remove(cursor);
            obj.insert("value".to_string(), Value::String(new_value));
        }
    });
}

fn handle_input_cursor_left(app: &mut AppState) {
    with_focused_input(app, |obj| {
        let cursor = obj
            .get("cursor")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        if cursor > 0 {
            obj.insert("cursor".to_string(), Value::from((cursor - 1) as u64));
        }
    });
}

fn handle_input_cursor_right(app: &mut AppState) {
    with_focused_input(app, |obj| {
        let value = obj
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let cursor = obj
            .get("cursor")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        if cursor < value.len() {
            obj.insert("cursor".to_string(), Value::from((cursor + 1) as u64));
        }
    });
}

fn handle_input_cursor_home(app: &mut AppState) {
    with_focused_input(app, |obj| {
        obj.insert("cursor".to_string(), Value::from(0u64));
    });
}

fn handle_input_cursor_end(app: &mut AppState) {
    with_focused_input(app, |obj| {
        let len = obj
            .get("value")
            .and_then(|v| v.as_str())
            .map(|s| s.len())
            .unwrap_or(0);
        obj.insert("cursor".to_string(), Value::from(len as u64));
    });
}
