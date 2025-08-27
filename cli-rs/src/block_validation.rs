use anyhow::{Context, Result};

/// Validate a spec string using Core's App::new parser
/// Returns Ok(()) if valid, or an error describing the validation issue
pub async fn validate_spec(spec: &str) -> Result<()> {
    // Use Core's parser to validate the spec - pass through the actual error
    // The error will be displayed by the caller with full formatting
    dust::app::App::new(spec).await?;
    Ok(())
}

/// Helper to validate and write a spec to disk
/// Only writes if the spec is valid
pub async fn validate_and_write(spec_path: &str, spec: &str) -> Result<()> {
    // First validate with Core
    validate_spec(spec).await?;

    // Only write if validation passed
    std::fs::write(spec_path, spec).with_context(|| format!("Failed to write '{}'", spec_path))?;

    Ok(())
}
