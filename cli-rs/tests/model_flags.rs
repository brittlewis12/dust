use assert_cmd::prelude::*;
use std::io::Write;
use std::process::Command;

fn write_llm_spec() -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new()
        .prefix("llm_spec")
        .suffix(".dust")
        .tempfile()
        .unwrap();
    // An LLM block with prompt and required fields; runtime model provided via --model
    write!(
        f,
        "llm GREET {{\n  prompt: Hello\n  max_tokens: 8\n  temperature: 0.2\n}}\n"
    )
    .unwrap();
    f
}

#[test]
fn run_llm_without_model_errors_with_missing_config() {
    let spec = write_llm_spec();
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args(["run", spec.path().to_str().unwrap()])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("Missing configuration for llm block"),
        "Expected missing config error in events; got: {}",
        stdout
    );
}

#[test]
fn run_llm_with_model_reaches_provider_key_error() {
    let spec = write_llm_spec();
    // Provide model via runtime flag and a sample extra config; should now fail for provider key
    let out = Command::cargo_bin("dustx")
        .unwrap()
        .args([
            "run",
            spec.path().to_str().unwrap(),
            "--model",
            "GREET=openai/gpt-5",
            "--set",
            "GREET:temperature=0.2",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("OPENAI_API_KEY") || stdout.contains("To use OpenAI\'s models"),
        "Expected provider key error; got: {}",
        stdout
    );
}
