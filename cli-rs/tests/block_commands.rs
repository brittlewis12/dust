use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn dustx() -> assert_cmd::Command {
    Command::cargo_bin("dustx").unwrap().into()
}

#[test]
fn block_add_appends_new_block() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "input INPUT {{}}\n\ncode EXISTING {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}"
    )
    .unwrap();

    dustx()
        .args([
            "block",
            "add",
            file.path().to_str().unwrap(),
            "code",
            "NEW_BLOCK",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(file.path()).unwrap();
    assert!(content.contains("code NEW_BLOCK"));
    assert!(content.contains("code EXISTING")); // Original still there
}

#[test]
fn block_rm_removes_block() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "input INPUT {{}}\n\ncode TO_REMOVE {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}\n\ncode KEEP {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}").unwrap();

    dustx()
        .args(["block", "rm", file.path().to_str().unwrap(), "TO_REMOVE"])
        .assert()
        .success();

    let content = std::fs::read_to_string(file.path()).unwrap();
    assert!(!content.contains("code TO_REMOVE"));
    assert!(content.contains("code KEEP")); // Other block still there
}

#[test]
fn block_rename_changes_block_name() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "code OLD_NAME {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}"
    )
    .unwrap();

    dustx()
        .args([
            "block",
            "rename",
            file.path().to_str().unwrap(),
            "OLD_NAME",
            "NEW_NAME",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(file.path()).unwrap();
    assert!(!content.contains("OLD_NAME"));
    assert!(content.contains("code NEW_NAME"));
}

#[test]
fn block_move_reorders_blocks() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "code FIRST {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}\n\ncode SECOND {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}\n\ncode THIRD {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}").unwrap();

    dustx()
        .args([
            "block",
            "move",
            file.path().to_str().unwrap(),
            "THIRD",
            "--before",
            "FIRST",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(file.path()).unwrap();
    let third_pos = content.find("code THIRD").unwrap();
    let first_pos = content.find("code FIRST").unwrap();
    assert!(
        third_pos < first_pos,
        "THIRD should come before FIRST after move"
    );
}

#[test]
fn block_set_config_updates_config() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "llm TEST {{\n  prompt: Hello world\n  max_tokens: 100\n}}"
    )
    .unwrap();

    dustx()
        .args([
            "block",
            "set-config",
            file.path().to_str().unwrap(),
            "TEST",
            "temperature=0.5",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(file.path()).unwrap();
    assert!(content.contains("temperature"));
}

#[test]
fn block_rm_nonexistent_fails() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "code EXISTING {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}"
    )
    .unwrap();

    dustx()
        .args(["block", "rm", file.path().to_str().unwrap(), "NONEXISTENT"])
        .assert()
        .failure();
}

#[test]
fn block_rename_duplicate_name_fails() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "code BLOCK1 {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}\n\ncode BLOCK2 {{\n  code:\n```\n_fun = (env) => ({{}})\n```\n}}").unwrap();

    dustx()
        .args([
            "block",
            "rename",
            file.path().to_str().unwrap(),
            "BLOCK1",
            "BLOCK2",
        ])
        .assert()
        .failure();
}

#[test]
fn block_add_dry_run_does_not_modify_file() {
    // Create a temp spec
    let mut f = tempfile::Builder::new()
        .prefix("spec")
        .suffix(".dust")
        .tempfile()
        .unwrap();
    write!(f, "input INPUT {{\n\n}}\n").unwrap();
    let path = f.path().to_path_buf();
    let before = std::fs::read_to_string(&path).unwrap();

    // Dry-run rename the INPUT block to something else (no write)
    Command::cargo_bin("dustx")
        .unwrap()
        .args([
            "block",
            "rename",
            path.to_str().unwrap(),
            "INPUT",
            "RENAMED",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run] would rename"));

    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(before, after, "spec file should be unchanged in dry-run");
}
