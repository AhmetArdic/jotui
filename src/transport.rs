use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::Write as IoWrite;
use std::sync::{Mutex, OnceLock};

static EVENT_FILE: OnceLock<Mutex<File>> = OnceLock::new();

pub fn init_event_output() {
    let path = std::env::args()
        .skip_while(|a| a != "--events-file")
        .nth(1)
        .unwrap_or_else(|| "events.jsonl".to_string());

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("Failed to open events output file");

    EVENT_FILE.set(Mutex::new(file)).ok();
}

pub fn send_event(event_json: &Value) {
    if let Ok(s) = serde_json::to_string(event_json) {
        if let Some(file_mutex) = EVENT_FILE.get() {
            if let Ok(mut file) = file_mutex.lock() {
                let _ = writeln!(file, "{}", s);
                let _ = file.flush();
            }
        }
    }
}

pub fn make_event(page: &str, source: Option<&str>, action: &str, value: Value) -> Value {
    let mut ev = serde_json::json!({
        "msg": "event",
        "page": page,
        "source": source,
        "action": action,
    });
    ev.as_object_mut()
        .unwrap()
        .insert("value".to_string(), value);
    ev
}

pub fn make_key_event(page: &str, source: Option<&str>, key: &str) -> Value {
    serde_json::json!({
        "msg": "event",
        "page": page,
        "source": source,
        "action": "key",
        "key": key
    })
}

pub fn load_defaults() -> Value {
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("defaults.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("defaults.json"));

    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(val) = serde_json::from_str(&content) {
            return val;
        }
    }

    serde_json::from_str(include_str!("../config/defaults.json"))
        .unwrap_or(Value::Object(Default::default()))
}
