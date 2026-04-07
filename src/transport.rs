use flate2::read::ZlibDecoder;
use serde_json::Value;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock};

static CONN: OnceLock<Mutex<BufWriter<TcpStream>>> = OnceLock::new();

/// Connect to the backend's TCP listener and spawn a reader thread.
/// Returns the mpsc receiver for incoming JSON-RPC messages.
pub fn connect(port: u16, tx: std::sync::mpsc::Sender<Value>) {
    let stream = TcpStream::connect(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("Failed to connect to backend on port {}: {}", port, e));

    let reader_stream = stream.try_clone().expect("Failed to clone TCP stream");
    CONN.set(Mutex::new(BufWriter::new(stream))).ok();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(reader_stream);
        loop {
            match read_message(&mut reader) {
                Ok(Some(val)) => {
                    if tx.send(val).is_err() {
                        break;
                    }
                }
                Ok(None) => continue,
                Err(_) => break,
            }
        }
    });
}

/// Read a JSON-RPC message with Content-Length framing from a buffered reader.
/// Supports optional `Content-Encoding: deflate` (zlib) compression.
pub fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut content_length: Option<usize> = None;
    let mut compressed = false;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        } else if trimmed.eq_ignore_ascii_case("Content-Encoding: deflate") {
            compressed = true;
        }
    }

    let len = match content_length {
        Some(n) => n,
        None => {
            eprintln!("[warn] missing Content-Length header");
            return Ok(None);
        }
    };

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    let json_bytes: Vec<u8> = if compressed {
        let mut out = Vec::new();
        ZlibDecoder::new(&buf[..]).read_to_end(&mut out).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("deflate decode: {}", e))
        })?;
        out
    } else {
        buf
    };

    match serde_json::from_slice(&json_bytes) {
        Ok(val) => Ok(Some(val)),
        Err(e) => {
            eprintln!("[warn] invalid JSON in message body: {}", e);
            Ok(None)
        }
    }
}

/// Write a JSON-RPC message with Content-Length framing to a writer.
pub fn write_message(writer: &mut impl Write, json: &Value) -> io::Result<()> {
    let body = serde_json::to_string(json).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(body.as_bytes())?;
    writer.flush()
}

/// Send an event notification to the backend via TCP.
pub fn send_event(params: &Value) {
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event",
        "params": params
    });

    if let Some(conn) = CONN.get() {
        if let Ok(mut writer) = conn.lock() {
            let _ = write_message(&mut *writer, &envelope);
        }
    }
}

/// Construct event params for a widget action (select, submit, etc.).
pub fn make_event(page: &str, source: Option<&str>, action: &str, value: Value) -> Value {
    serde_json::json!({
        "page": page,
        "source": source,
        "action": action,
        "value": value
    })
}

/// Construct event params for a key press.
pub fn make_key_event(page: &str, source: Option<&str>, key: &str) -> Value {
    serde_json::json!({
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
