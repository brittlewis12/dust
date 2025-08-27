use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn run_fib_without_events_prints_nothing() {
    let tmp_data = tempfile::tempdir().unwrap();
    let data_dir = tmp_data.path();
    let repo_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Create dataset
    let ds_path = repo_dir.join("examples").join("fib-input.json");
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .env("DUSTX_DATA_DIR", data_dir)
        .current_dir(&repo_dir)
        .args(["dataset", "create", "fib", ds_path.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .clone();
    let hash = String::from_utf8(out.stdout).unwrap();
    let hash = hash.trim();

    // Run the spec with --no-events
    let output = Command::cargo_bin("dustx")
        .unwrap()
        .env("DUSTX_DATA_DIR", data_dir)
        .current_dir(&repo_dir)
        .args([
            "run",
            "examples/fib.dust",
            "--dataset-id",
            "fib",
            "--dataset-hash",
            hash,
            "--no-events",
        ])
        .assert()
        .failure() // fib example deliberately errors on one item
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("\"block_type\":"),
        "Expected no streamed events in stdout"
    );
}
