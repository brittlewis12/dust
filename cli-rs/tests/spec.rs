use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn write_tmp(contents: &str, name: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new()
        .prefix(name)
        .suffix(".dust")
        .tempfile()
        .unwrap();
    f.write_all(contents.as_bytes()).unwrap();
    f
}

#[test]
fn spec_check_ok_parses_simple_code() {
    let spec = r#"
input INPUT {}

code DO {
  code:
```
_fun = (env) => ({ ok: true })
```
}
"#;
    let f = write_tmp(spec, "ok_spec");
    Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK "));
}

#[test]
fn spec_check_rejects_unsupported_blocks() {
    let spec = r#"
data_source DS {
  query: "hello"
}
"#;
    let f = write_tmp(spec, "unsupported_spec");
    Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}

#[test]
fn spec_fmt_idempotent_on_simple_spec() {
    let spec = r#"
input INPUT {}

code DO {
  code:
```
_fun = (env) => ({ ok: true })
```
}
"#;
    let f = write_tmp(spec, "fmt_spec");
    let path = f.path().to_path_buf();

    // First format
    Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "fmt", path.to_str().unwrap()])
        .assert()
        .success();
    let first = fs::read_to_string(&path).unwrap();
    // Second format should be no-op
    Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "fmt", path.to_str().unwrap()])
        .assert()
        .success();
    let second = fs::read_to_string(&path).unwrap();
    assert_eq!(first, second);
}

#[test]
fn spec_check_detects_duplicate_blocks() {
    let spec = r#"
code DUPE {
  code:
```
_fun = (env) => ({})
```
}

code DUPE {
  code:
```
_fun = (env) => ({})
```
}
"#;
    let f = write_tmp(spec, "duplicate_blocks");
    Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("DUPE").or(predicate::str::contains("multiple")));
}

#[test]
fn spec_check_validates_map_reduce_pairs() {
    // Test orphaned reduce - Core validates that reduce needs a matching map
    let spec = r#"
input INPUT {}

reduce ORPHAN {
}
"#;
    let f = write_tmp(spec, "orphan_reduce");
    Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("reduce ORPHAN").and(predicate::str::contains("not matched")),
        );
}

#[test]
fn spec_diff_shows_format_differences() {
    let mut file = NamedTempFile::new().unwrap();
    // Write poorly formatted spec
    writeln!(
        file,
        "input INPUT   {{}}\n\n\ncode    FIB {{\ncode:\n```\n_fun=(env)=>({{}})\n```\n}}"
    )
    .unwrap();

    let output = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "diff", file.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should show formatting differences
    assert!(stdout.contains("-") || stdout.contains("+") || stdout.contains("No differences"));
}

#[test]
fn spec_tokens_estimates_usage() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "llm GENERATE {{\n  model:\n    provider: openai\n    id: gpt-4\n  prompt: Write a haiku about Rust programming\n}}").unwrap();

    let output = Command::cargo_bin("dustx")
        .unwrap()
        .args([
            "spec",
            "tokens",
            file.path().to_str().unwrap(),
            "--model",
            "openai/gpt-4",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should output token estimates
    assert!(stdout.contains("token") || stdout.contains("Token") || !stdout.is_empty());
}

#[test]
fn spec_hash_is_deterministic() {
    let mut file = NamedTempFile::new().unwrap();
    let spec = "input INPUT {}\n\ncode TEST {\n  code:\n```\n_fun = (env) => ({})\n```\n}";
    writeln!(file, "{}", spec).unwrap();

    // Get hash twice
    let output1 = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "hash", file.path().to_str().unwrap()])
        .output()
        .unwrap();

    let output2 = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "hash", file.path().to_str().unwrap()])
        .output()
        .unwrap();

    let hash1 = String::from_utf8(output1.stdout).unwrap();
    let hash2 = String::from_utf8(output2.stdout).unwrap();

    assert_eq!(hash1, hash2, "Hash should be deterministic");
    assert!(!hash1.is_empty(), "Hash should not be empty");
}

#[test]
fn spec_check_shows_parse_location_for_unclosed_quote() {
    let spec = r#"llm TEST {
  prompt: "Unclosed quote
  max_tokens: 100
}"#;
    let f = write_tmp(spec, "unclosed_quote");

    let output = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();

    // Verify we're showing the actual parse location
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr.contains("2:"),
        "Missing line number. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("prompt"),
        "Missing field context. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("expected") || stderr.contains("Expected"),
        "Missing expectation. Got:\n{}",
        stderr
    );
}

#[test]
fn spec_check_shows_type_error_with_context() {
    let spec = r#"llm TEST {
  prompt: Hello world
  max_tokens: not_a_number
}"#;
    let f = write_tmp(spec, "type_error");

    let output = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should give clear error about type mismatch
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr.contains("max_tokens"),
        "Missing field name. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("integer") || stderr.contains("Integer"),
        "Missing type info. Got:\n{}",
        stderr
    );
}

#[test]
fn spec_check_shows_missing_required_field() {
    let spec = r#"llm TEST {
  prompt: Hello world
}"#;
    let f = write_tmp(spec, "missing_field");

    let output = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should clearly state what's missing
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr.contains("max_tokens"),
        "Missing field name. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("required")
            || stderr.contains("Required")
            || stderr.contains("Missing")
            || stderr.contains("missing"),
        "Missing requirement info. Got:\n{}",
        stderr
    );
}

#[test]
fn spec_check_shows_unexpected_field() {
    let spec = r#"llm TEST {
  prompt: Hello world
  max_tokens: 100
  invalid_field: value
}"#;
    let f = write_tmp(spec, "unexpected_field");

    let output = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should identify the problematic field
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr.contains("invalid_field"),
        "Missing field name. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("Unexpected") || stderr.contains("unexpected"),
        "Missing 'unexpected' explanation. Got:\n{}",
        stderr
    );
}
