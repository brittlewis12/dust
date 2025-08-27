use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn map_reduce_processes_array_in_parallel() {
    let tmp_data = tempfile::tempdir().unwrap();
    let data_dir = tmp_data.path();
    let repo_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Run map/reduce spec that generates array internally
    let output = Command::cargo_bin("dustx")
        .unwrap()
        .env("DUSTX_DATA_DIR", data_dir)
        .current_dir(&repo_dir)
        .args(["run", "examples/map-reduce.dust"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should process array of 4 items
    assert!(stdout.contains("\"count\":4"), "Should process 4 items");

    // Sum should be fib(5) + fib(7) + fib(10) + fib(12) = 5 + 13 + 55 + 144 = 217
    assert!(stdout.contains("\"total_sum\":217"), "Sum should be 217");

    // Should see the map block processing
    assert!(
        stdout.contains("\"block_type\":\"map\",\"name\":\"FIBS\""),
        "Should see map block"
    );

    // Check the final values too
    assert!(
        stdout.contains("\"values\":[{\"n\":5,\"fib\":5}"),
        "Should see computed values"
    );
}
