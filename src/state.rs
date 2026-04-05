use serde_json::{Map, Value};
use std::collections::HashMap;

/// Three-layer shallow merge (with one-level-deep merge for style objects).
/// Each layer merges on top of the previous one.
pub fn shallow_merge(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(b), Value::Object(o)) => {
            let mut merged = b.clone();
            for (k, v) in o {
                if let (Some(Value::Object(bv)), Value::Object(ov)) = (b.get(k), v) {
                    // One-level-deep merge for nested objects (e.g., style)
                    let mut nested = bv.clone();
                    for (nk, nv) in ov {
                        nested.insert(nk.clone(), nv.clone());
                    }
                    merged.insert(k.clone(), Value::Object(nested));
                } else {
                    merged.insert(k.clone(), v.clone());
                }
            }
            Value::Object(merged)
        }
        _ => overlay.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub layout: Value,
    pub widgets: HashMap<String, Value>,
    pub focus_ring: Vec<String>,
    pub focus_index: Option<usize>,
}

impl Page {
    pub fn from_json(layout: &Value, widgets: &[Value], defaults: &Value, styles: &Value) -> Self {
        let mut widget_map = HashMap::new();
        for w in widgets {
            if let Some(id) = w.get("id").and_then(|v| v.as_str()) {
                let wtype = w.get("type").and_then(|v| v.as_str()).unwrap_or("paragraph");

                // Layer 1: defaults for this widget type
                let base = defaults
                    .get(wtype)
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Map::new()));

                // Layer 2: render props (resolve style references)
                let mut render_props = w.clone();
                resolve_style_ref(&mut render_props, styles);

                let merged = shallow_merge(&base, &render_props);
                widget_map.insert(id.to_string(), merged);
            }
        }

        // Build focus ring from layout tree (depth-first)
        let mut focus_ring = Vec::new();
        build_focus_ring(layout, &widget_map, &mut focus_ring);

        let focus_index = if focus_ring.is_empty() {
            None
        } else {
            Some(0)
        };

        Page {
            layout: layout.clone(),
            widgets: widget_map,
            focus_ring,
            focus_index,
        }
    }

    pub fn apply_patch(&mut self, updates: &[Value], styles: &Value) {
        for update in updates {
            if let Some(id) = update.get("id").and_then(|v| v.as_str()) {
                if let Some(existing) = self.widgets.get(id) {
                    let mut patch = update.clone();
                    resolve_style_ref(&mut patch, styles);
                    let merged = shallow_merge(existing, &patch);
                    self.widgets.insert(id.to_string(), merged);
                } else {
                    eprintln!("[warn] patch targets nonexistent widget: {}", id);
                }
            }
        }
    }
}

fn resolve_style_ref(value: &mut Value, styles: &Value) {
    if let Some(style_val) = value.get("style") {
        if let Some(style_name) = style_val.as_str() {
            if let Some(resolved) = styles.get(style_name) {
                value
                    .as_object_mut()
                    .unwrap()
                    .insert("style".to_string(), resolved.clone());
            }
        }
    }
    // Also resolve highlight_style if it's a string reference
    if let Some(hs_val) = value.get("highlight_style") {
        if let Some(hs_name) = hs_val.as_str() {
            if let Some(resolved) = styles.get(hs_name) {
                value
                    .as_object_mut()
                    .unwrap()
                    .insert("highlight_style".to_string(), resolved.clone());
            }
        }
    }
}

fn build_focus_ring(node: &Value, widgets: &HashMap<String, Value>, ring: &mut Vec<String>) {
    if let Some(ref_id) = node.get("ref").and_then(|v| v.as_str()) {
        if let Some(w) = widgets.get(ref_id) {
            let focusable = w.get("focusable").and_then(|v| v.as_bool()).unwrap_or(false);
            if focusable {
                ring.push(ref_id.to_string());
            }
        }
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            build_focus_ring(child, widgets, ring);
        }
    }
}

#[derive(Debug)]
pub struct AppState {
    pub pages: HashMap<String, Page>,
    pub page_order: Vec<String>,
    pub active_page: String,
    pub styles: Value,
    pub defaults: Value,
}

impl AppState {
    pub fn new(defaults: Value) -> Self {
        AppState {
            pages: HashMap::new(),
            page_order: Vec::new(),
            active_page: String::new(),
            styles: Value::Object(Map::new()),
            defaults,
        }
    }

    pub fn handle_render(&mut self, msg: &Value) {
        self.pages.clear();
        self.page_order.clear();

        if let Some(styles) = msg.get("styles") {
            self.styles = styles.clone();
        }

        if let Some(order) = msg.get("page_order").and_then(|v| v.as_array()) {
            self.page_order = order
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(pages) = msg.get("pages").and_then(|v| v.as_object()) {
            for (page_id, page_def) in pages {
                let layout = page_def.get("layout").cloned().unwrap_or(Value::Object(Map::new()));
                let widgets = page_def
                    .get("widgets")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                let page = Page::from_json(&layout, &widgets, &self.defaults, &self.styles);
                self.pages.insert(page_id.clone(), page);

                if !self.page_order.contains(page_id) {
                    self.page_order.push(page_id.clone());
                }
            }
        }

        if let Some(active) = msg.get("active").and_then(|v| v.as_str()) {
            if self.pages.contains_key(active) {
                self.active_page = active.to_string();
            }
        }

        if self.active_page.is_empty() {
            if let Some(first) = self.page_order.first() {
                self.active_page = first.clone();
            }
        }
    }

    pub fn handle_patch(&mut self, msg: &Value) {
        let page_id = match msg.get("page").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                eprintln!("[warn] patch message missing 'page' field");
                return;
            }
        };

        let updates = match msg.get("updates").and_then(|v| v.as_array()) {
            Some(u) => u.clone(),
            None => return,
        };

        if let Some(page) = self.pages.get_mut(page_id) {
            page.apply_patch(&updates, &self.styles);
        } else {
            eprintln!("[warn] patch targets nonexistent page: {}", page_id);
        }
    }

    pub fn handle_navigate(&mut self, msg: &Value) {
        if let Some(page_id) = msg.get("page").and_then(|v| v.as_str()) {
            if self.pages.contains_key(page_id) {
                self.active_page = page_id.to_string();
            } else {
                eprintln!("[warn] navigate to nonexistent page: {}", page_id);
            }
        }
    }

    pub fn active_page(&self) -> Option<&Page> {
        self.pages.get(&self.active_page)
    }

    pub fn active_page_mut(&mut self) -> Option<&mut Page> {
        self.pages.get_mut(&self.active_page)
    }
}
