//! Pre-run hooks for injecting extra `docker run` arguments.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use std::process::Command;
use toml::de::Error as TomlError;

/// Execute the pre-run hook if configured and return extra `docker run` arguments.
///
/// When `pre_run` is `Some`, the referenced executable is run and its stdout is parsed as a TOML
/// array of strings (e.g. `["--mount", "/host:/container"]`). If `pre_run` is `None`, an empty
/// vector is returned.
pub(crate) fn execute_pre_run_hook(pre_run: Option<&str>) -> Result<Vec<String>> {
    let Some(pre_run_path) = pre_run else {
        return Ok(Vec::new());
    };

    let output = Command::new(pre_run_path)
        .output()
        .with_context(|| format!("Failed to execute pre-run hook {pre_run_path:?}"))?;

    if !output.status.success() {
        let stderr_text = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Pre-run hook {pre_run_path:?} exited with status {status}:\n{stderr_text}",
            status = output.status,
        );
    }

    let stdout_text =
        String::from_utf8(output.stdout).context("Pre-run hook stdout is not valid UTF-8")?;

    parse_pre_run_output(&stdout_text).with_context(|| {
        format!("Failed to parse pre-run hook output as a TOML list of strings: {stdout_text:?}")
    })
}

/// Parse the raw stdout of a pre-run hook into a list of extra arguments.
///
/// A bare TOML array is not a valid TOML document (which requires key-value pairs at the top
/// level). The output is wrapped in a synthetic key so the `toml` crate can parse it.
fn parse_pre_run_output(stdout: &str) -> Result<Vec<String>, TomlError> {
    #[derive(Deserialize)]
    struct PreRunOutput {
        args: Vec<String>,
    }

    let wrapped = format!("args = {}", stdout.trim());
    let parsed: PreRunOutput = toml::from_str(&wrapped)?;
    Ok(parsed.args)
}

#[cfg(test)]
mod tests;
