use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocEntry {
    pub path: String,
    pub size: u64,
    pub modified_ms: u128,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Manifest {
    pub root: String,
    pub docs: Vec<DocEntry>,
}

fn index_dir() -> PathBuf {
    PathBuf::from(".dust/local_docs")
}
fn manifest_path() -> PathBuf {
    index_dir().join("manifest.json")
}

pub fn stats() -> Result<()> {
    let man = read_manifest().unwrap_or_default();
    println!(
        "index: {} docs={}, root={}",
        index_dir().display(),
        man.docs.len(),
        man.root
    );
    Ok(())
}

pub fn clear() -> Result<()> {
    let dir = index_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("remove {}", dir.display()))?;
    }
    println!("cleared index at {}", dir.display());
    Ok(())
}

pub fn index(path: &str, incremental: bool) -> Result<()> {
    let p = Path::new(path);
    if !p.exists() {
        bail!("path not found: {}", path);
    }
    let mut man = if incremental {
        read_manifest().unwrap_or_default()
    } else {
        Manifest::default()
    };
    man.root = p
        .canonicalize()
        .unwrap_or_else(|_| p.to_path_buf())
        .display()
        .to_string();
    man.docs.clear();
    for e in WalkDir::new(p).into_iter().filter_map(|e| e.ok()) {
        let md = match e.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if md.is_file() {
            let path = e.path().to_path_buf();
            let size = md.len();
            let modified_ms = md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis())
                .unwrap_or(0);
            man.docs.push(DocEntry {
                path: path.display().to_string(),
                size,
                modified_ms,
            });
        }
    }
    write_manifest(&man)?;
    println!("indexed {} documents", man.docs.len());
    Ok(())
}

pub fn search(query: &str, k: usize) -> Result<()> {
    // Naive substring search across files listed in manifest; prints top-K by first match position
    let man = read_manifest().unwrap_or_default();
    let mut hits: Vec<(usize, String)> = Vec::new();
    for d in &man.docs {
        if let Ok(text) = fs::read_to_string(&d.path) {
            if let Some(pos) = text.to_lowercase().find(&query.to_lowercase()) {
                hits.push((pos, d.path.clone()));
            }
        }
    }
    hits.sort_by_key(|(pos, _)| *pos);
    for (_, path) in hits.into_iter().take(k) {
        println!("{}", path);
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchHit {
    pub path: String,
    pub snippet: String,
}

pub fn search_hits(query: &str, k: usize) -> Result<Vec<SearchHit>> {
    let man = read_manifest().unwrap_or_default();
    let mut hits: Vec<(usize, String, usize)> = Vec::new();
    for d in &man.docs {
        if let Ok(text) = fs::read_to_string(&d.path) {
            if let Some(pos) = text.to_lowercase().find(&query.to_lowercase()) {
                hits.push((pos, d.path.clone(), text.len()));
            }
        }
    }
    hits.sort_by_key(|(pos, _, _)| *pos);
    let mut out = Vec::new();
    for (_, path, _len) in hits.into_iter().take(k) {
        if let Ok(text) = fs::read_to_string(&path) {
            // Extract a snippet around the first match (case-insensitive)
            let lower = text.to_lowercase();
            if let Some(pos) = lower.find(&query.to_lowercase()) {
                let start = pos.saturating_sub(160);
                let end = (pos + 160).min(text.len());
                let mut snippet = text[start..end].to_string();
                snippet = snippet.replace('\n', " ");
                out.push(SearchHit { path, snippet });
            }
        }
    }
    Ok(out)
}

fn read_manifest() -> Result<Manifest> {
    let p = manifest_path();
    let b = fs::read(&p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_slice(&b).context("parse manifest")
}

fn write_manifest(m: &Manifest) -> Result<()> {
    let dir = index_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).context("create index dir")?;
    }
    fs::write(manifest_path(), serde_json::to_vec_pretty(m)?).context("write manifest")
}
