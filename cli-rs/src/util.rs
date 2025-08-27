use serde_json::Value;

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
