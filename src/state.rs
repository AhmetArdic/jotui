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

/// Resolve `$name` references in top-level string properties against defs.
fn resolve_refs(value: &mut Value, defs: &Value) {
    if let Some(obj) = value.as_object_mut() {
        let keys: Vec<String> = obj.keys().cloned().collect();
        for key in keys {
            if let Some(s) = obj.get(&key).and_then(|v| v.as_str()).map(String::from) {
                if let Some(name) = s.strip_prefix('$') {
                    if let Some(resolved) = defs.get(name) {
                        obj.insert(key, resolved.clone());
                    }
                }
            }
        }
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
    /// Build a page from a unified tree node (layout + widgets merged).
    pub fn from_json(tree: &Value, defaults: &Value, defs: &Value) -> Self {
        let mut widget_map = HashMap::new();
        extract_widgets(tree, defaults, defs, &mut widget_map);

        let mut focus_ring = Vec::new();
        build_focus_ring(tree, &widget_map, &mut focus_ring);

        let focus_index = if focus_ring.is_empty() {
            None
        } else {
            Some(0)
        };

        Page {
            layout: tree.clone(),
            widgets: widget_map,
            focus_ring,
            focus_index,
        }
    }

    pub fn apply_patch(&mut self, updates: &[Value], defs: &Value) {
        for update in updates {
            if let Some(id) = update.get("id").and_then(|v| v.as_str()) {
                if let Some(existing) = self.widgets.get(id) {
                    let mut patch = update.clone();
                    resolve_refs(&mut patch, defs);
                    let merged = shallow_merge(existing, &patch);
                    self.widgets.insert(id.to_string(), merged);
                } else {
                    eprintln!("[warn] patch targets nonexistent widget: {}", id);
                }
            }
        }
    }
}

/// Walk the unified tree and extract widgets (nodes with `id` + `type`) into the map.
fn extract_widgets(
    node: &Value,
    defaults: &Value,
    defs: &Value,
    widgets: &mut HashMap<String, Value>,
) {
    if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
        if node.get("type").is_some() {
            let wtype = node.get("type").and_then(|v| v.as_str()).unwrap_or("paragraph");

            // Layer 1: defaults for this widget type
            let base = defaults
                .get(wtype)
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));

            // Layer 2: render props (resolve $ref references)
            let mut render_props = node.clone();
            resolve_refs(&mut render_props, defs);

            let merged = shallow_merge(&base, &render_props);
            widgets.insert(id.to_string(), merged);
        }
    }

    // Recurse into children
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            extract_widgets(child, defaults, defs, widgets);
        }
    }
}

fn build_focus_ring(node: &Value, widgets: &HashMap<String, Value>, ring: &mut Vec<String>) {
    if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
        if let Some(w) = widgets.get(id) {
            let focusable = w.get("focusable").and_then(|v| v.as_bool()).unwrap_or(false);
            if focusable {
                ring.push(id.to_string());
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
    pub defs: Value,
    pub defaults: Value,
}

impl AppState {
    pub fn new(defaults: Value) -> Self {
        AppState {
            pages: HashMap::new(),
            page_order: Vec::new(),
            active_page: String::new(),
            defs: Value::Object(Map::new()),
            defaults,
        }
    }

    pub fn handle_render(&mut self, msg: &Value) {
        self.pages.clear();
        self.page_order.clear();

        if let Some(defs) = msg.get("defs") {
            self.defs = defs.clone();
        }

        if let Some(pages) = msg.get("pages").and_then(|v| v.as_array()) {
            for page_def in pages {
                if let Some(page_id) = page_def.get("id").and_then(|v| v.as_str()) {
                    let page = Page::from_json(page_def, &self.defaults, &self.defs);
                    self.page_order.push(page_id.to_string());
                    self.pages.insert(page_id.to_string(), page);
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
            page.apply_patch(&updates, &self.defs);
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
