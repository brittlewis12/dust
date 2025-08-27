use assert_cmd::prelude::*;
use std::io::Write;
use std::process::Command;

fn normalize_paths(s: &str) -> String {
    let repo = env!("CARGO_MANIFEST_DIR");
    s.replace(repo, "<REPO>")
}

#[test]
fn spec_parse_error_feedback() {
    // invalid key usage that will trigger a pest parse error location
    let mut f = tempfile::Builder::new().suffix(".dust").tempfile().unwrap();
    write!(
        f,
        "input INPUT {{}}\n\nllm GREET {{\n  provider: \"openai/gpt-5\",\n  prompt: Hello\n}}\n"
    )
    .unwrap();
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = normalize_paths(&String::from_utf8_lossy(&out.stderr));
    // print for manual review
    println!("=== spec_parse_error_feedback ===\n{}", err);
    assert!(!err.is_empty());
}

#[test]
fn spec_unsupported_block_feedback() {
    let mut f = tempfile::Builder::new().suffix(".dust").tempfile().unwrap();
    write!(f, "data_source DS {{\n  query: \"hello\"\n}}\n").unwrap();
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "check", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = normalize_paths(&String::from_utf8_lossy(&out.stderr));
    println!("=== spec_unsupported_block_feedback ===\n{}", err);
    assert!(!err.is_empty());
}

#[test]
fn block_rename_missing_feedback() {
    let mut f = tempfile::Builder::new().suffix(".dust").tempfile().unwrap();
    write!(f, "input INPUT {{}}\n").unwrap();
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args([
            "block",
            "rename",
            f.path().to_str().unwrap(),
            "MISSING",
            "NEW",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = normalize_paths(&String::from_utf8_lossy(&out.stderr));
    println!("=== block_rename_missing_feedback ===\n{}", err);
    assert!(!err.is_empty());
}

#[test]
fn spec_diff_missing_file_feedback() {
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args(["spec", "diff", "<notfound>"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = normalize_paths(&String::from_utf8_lossy(&out.stderr));
    println!("=== spec_diff_missing_file_feedback ===\n{}", err);
    assert!(!err.is_empty());
}
#[test]
fn run_messages_code_shape_hint() {
    // A chat spec whose messages_code returns wrong shape (object), to surface shape hint
    let mut f = tempfile::Builder::new().suffix(".dust").tempfile().unwrap();
    use std::io::Write as _;
    write!(
        f,
        "{}",
        r#"chat TALK {
  instructions: Hello
  temperature: 0.2
  messages_code:
```
_fun = (env) => ({ role: "user", content: "hi" })
```
}
"#
    )
    .unwrap();
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args([
            "run",
            f.path().to_str().unwrap(),
            "--model",
            "TALK=openai/gpt-5",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = normalize_paths(&String::from_utf8_lossy(&out.stderr));
    println!("=== run_messages_code_shape_hint ===\n{}", err);
    assert!(err.contains("hint: messages_code must return an array"));
}
