use assert_cmd::prelude::*;
use std::path::Path;
use std::process::Command;

fn bin_with_data_dir(data_dir: &Path) -> assert_cmd::Command {
    let mut cmd: assert_cmd::Command = Command::cargo_bin("dustx").unwrap().into();
    cmd.env("DUSTX_DATA_DIR", data_dir);
    cmd
}

#[test]
fn data_block_loads_dataset() {
    // Use a single temp dir for this test to avoid PG conflicts
    let tmp_data = tempfile::tempdir().unwrap();
    let data_dir = tmp_data.path();

    // Create a dataset first
    let dataset_content = r#"[
        {"name": "Alice", "age": 30},
        {"name": "Bob", "age": 25}
    ]"#;
    let dataset_file = data_dir.join("people.json");
    std::fs::write(&dataset_file, dataset_content).unwrap();

    // Register the dataset
    let assert = bin_with_data_dir(data_dir)
        .args([
            "dataset",
            "create",
            "people",
            dataset_file.to_str().unwrap(),
        ])
        .assert()
        .success();
    let hash = String::from_utf8(assert.get_output().stdout.clone())
        .unwrap()
        .trim()
        .to_string();

    // Create a spec that uses a Data block to load the dataset
    let spec_content = format!(
        r#"
data PEOPLE {{
  dataset_id: people
  hash: {}
}}

code ANALYZE {{
  code:
```
_fun = (env) => {{
  const people = env.state.PEOPLE;
  const totalAge = people.reduce((sum, p) => sum + p.age, 0);
  const avgAge = totalAge / people.length;
  return {{
    count: people.length,
    average_age: avgAge,
    names: people.map(p => p.name)
  }};
}}
```
}}
"#,
        hash
    );

    let spec_file = data_dir.join("analyze.dust");
    std::fs::write(&spec_file, spec_content).unwrap();

    // Run the spec (no dataset flags needed since Data block loads it)
    let assert = bin_with_data_dir(data_dir)
        .args(["run", spec_file.to_str().unwrap()])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Verify the Data block loaded the dataset and the code processed it
    assert!(stdout.contains("\"count\":2"), "Should count 2 people");
    assert!(
        stdout.contains("\"average_age\":27.5"),
        "Should calculate correct average"
    );
    assert!(stdout.contains("\"Alice\""), "Should contain Alice");
    assert!(stdout.contains("\"Bob\""), "Should contain Bob");
}
