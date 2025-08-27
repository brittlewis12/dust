use parking_lot::Mutex;
use serde_json::Value;
use std::sync::OnceLock;

pub fn set_env_default(k: &str, v: &str) {
    if std::env::var(k).is_err() {
        std::env::set_var(k, v);
    }
}

pub fn load_local_credentials() {
    let path = match crate::pg::cli_data_dir() {
        Ok(p) => p.join("credentials.json"),
        Err(_) => return,
    };
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(map) = serde_json::from_slice::<std::collections::HashMap<String, String>>(&bytes)
        {
            for (k, v) in map {
                std::env::set_var(k, v);
            }
        }
    }
}

pub fn emit_event(typ: &str, content: Value) {
    println!("{}", serde_json::json!({ "type": typ, "content": content }));
}

pub fn strip_code_fences(s: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 2 < bytes.len() && bytes[i] == b'`' && bytes[i + 1] == b'`' && bytes[i + 2] == b'`' {
            in_code = !in_code;
            i += 3;
            continue;
        }
        if !in_code {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

pub fn approximate_block_lines(spec: &str) -> std::collections::HashMap<String, usize> {
    let mut map = std::collections::HashMap::new();
    let re = regex::Regex::new(r"^\s*([A-Za-z_]+)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{").unwrap();
    for (idx, line) in spec.lines().enumerate() {
        if let Some(cap) = re.captures(line) {
            if let Some(name) = cap.get(2) {
                map.insert(name.as_str().to_string(), idx + 1);
            }
        }
    }
    map
}

pub fn scan_unsupported_block_types(spec: &str) -> Vec<(String, String, usize)> {
    // Simple line-based scan outside code fences for block headers of unsupported types
    let nc = strip_code_fences(spec);
    let mut out: Vec<(String, String, usize)> = Vec::new();
    let re = regex::Regex::new(
        r"(?m)^\s*(data_source|database_schema|database)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{",
    )
    .unwrap();
    for (i, line) in nc.lines().enumerate() {
        let t = line.trim_start();
        if let Some(cap) = re.captures(t) {
            let bt = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let name = cap
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            out.push((bt, name, i + 1));
        }
    }
    out
}

pub fn likely_provider_misuse(spec: &str) -> bool {
    // Heuristic: outside code fences, presence of `provider:` alongside llm/chat blocks
    let nc = strip_code_fences(spec);
    let has_provider = nc.contains("\n  provider:")
        || nc.contains("\n provider:")
        || nc.trim_start().starts_with("provider:");
    if !has_provider {
        return false;
    }
    let re = regex::Regex::new(r"(?m)^\s*(llm|chat)\s+\w+\s*\{").unwrap();
    re.is_match(&nc)
}

// A tiny global hint sink so we can print hints after the main error in CLI.
static HINT_SINK: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn set_hint<S: Into<String>>(hint: S) {
    let cell = HINT_SINK.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock();
    *guard = Some(hint.into());
}

pub fn take_hint() -> Option<String> {
    let cell = HINT_SINK.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock();
    guard.take()
}
