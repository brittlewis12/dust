pub struct Bounds {
    pub start: usize,
    pub header_end: usize,
    pub end: usize,
}

pub fn find_and_extract(spec: &str, name: &str) -> Option<(Bounds, String)> {
    let (b, _) = find_block_bounds(spec, name)?;
    let snippet = spec[b.start..b.end].to_string();
    Some((b, snippet))
}

pub fn remove_block_by_name(spec: &str, name: &str) -> Option<(String, String)> {
    let (b, _) = find_block_bounds(spec, name)?;
    let mut out = String::new();
    out.push_str(&spec[..b.start]);
    out.push_str(&spec[b.end..]);
    Some((out, spec[b.start..b.end].to_string()))
}

pub fn find_block_bounds(spec: &str, name: &str) -> Option<(Bounds, usize)> {
    let header_pat = regex::Regex::new(&format!(
        r"(?m)^\n?([a-zA-Z_]+)\s+{}\s*\{{\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let m = header_pat.find(spec)?;
    let header_end = spec[m.end()..]
        .find('\n')
        .map(|o| m.end() + o + 1)
        .unwrap_or(m.end());
    let mut depth: i32 = 0;
    let mut in_code = false;
    let mut end_idx = header_end;
    let bytes = spec.as_bytes();
    let mut i = m.start();
    while i < bytes.len() {
        if i + 2 < bytes.len() && bytes[i] == b'`' && bytes[i + 1] == b'`' && bytes[i + 2] == b'`' {
            in_code = !in_code;
            i += 3;
            continue;
        }
        let c = bytes[i] as char;
        if !in_code {
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth -= 1;
                if depth == 0 {
                    end_idx = i + 1;
                    break;
                }
            }
        }
        i += 1;
    }
    if end_idx <= header_end {
        return None;
    }
    Some((
        Bounds {
            start: m.start(),
            header_end,
            end: end_idx,
        },
        m.start(),
    ))
}

pub fn set_config_line(body: &str, key: &str, val: &str) -> String {
    let mut out = String::new();
    let mut replaced = false;
    let re = regex::Regex::new(&format!(
        r"(?m)^(\n?\n?\s*){}\s*:\s*.*$",
        regex::escape(key)
    ))
    .unwrap();
    if let Some(m) = re.find(body) {
        out.push_str(&body[..m.start()]);
        out.push_str(&format!("{}: {}\n", key, val));
        out.push_str(&body[m.end()..]);
        replaced = true;
    }
    if !replaced {
        out.push_str(&format!("{}: {}\n", key, val));
        out.push_str(body);
    }
    out
}
