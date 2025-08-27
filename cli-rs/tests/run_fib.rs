use assert_cmd::prelude::*;
use std::path::Path;
use std::process::Command;

fn bin_with_data_dir(data_dir: &Path) -> assert_cmd::Command {
    let mut cmd: assert_cmd::Command = Command::cargo_bin("dustx").unwrap().into();
    cmd.env("DUSTX_DATA_DIR", data_dir);
    cmd
}

#[test]
fn run_fib_with_dataset_streams_outputs_and_error() {
    // Ephemeral data dir per test run (fast and isolated)
    let tmp_data = tempfile::tempdir().unwrap();
    let data_dir = tmp_data.path();

    // Use repo dir for relative example paths
    let repo_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Create dataset
    let ds_path = repo_dir.join("examples").join("fib-input.json");
    let assert = bin_with_data_dir(data_dir)
        .current_dir(&repo_dir)
        .args(["dataset", "create", "fib", ds_path.to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let hash = stdout.trim();

    // Run the spec with dataset id/hash
    let mut cmd = bin_with_data_dir(data_dir);
    cmd.current_dir(&repo_dir).args([
        "run",
        "examples/fib.dust",
        "--dataset-id",
        "fib",
        "--dataset-hash",
        hash,
    ]);

    // The run will fail because n=-1 causes an error, but that's expected
    let out = cmd.assert().failure().get_output().clone();
    let s = String::from_utf8(out.stdout).unwrap();

    // Expect outputs for n=5,7,10 and an error for n=-1
    assert!(s.contains("\"block_type\":\"code\",\"block_name\":\"FIB\""));
    assert!(s.contains("\"fib\":5"));
    assert!(s.contains("\"fib\":13"));
    assert!(s.contains("\"fib\":55"));
    assert!(s.contains("\"error\":\"Error in"));
}
