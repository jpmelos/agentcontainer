//! Hooks for injecting extra arguments into `docker build` and `docker run` commands.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use std::process::Command;
use toml::de::Error as TomlError;
use tracing::debug;

/// Execute the pre-build hook if configured and return extra `docker build` arguments.
///
/// When `pre_build` is `Some`, the referenced executable is run and its stdout is parsed as a TOML
/// array of strings (e.g. `["--label", "foo=bar"]`). If `pre_build` is `None`, an empty vector is
/// returned.
pub(crate) fn execute_pre_build_hook(pre_build: Option<&str>) -> Result<Vec<String>> {
    execute_hook(pre_build, "Pre-build")
}

/// Execute the pre-run hook if configured and return extra `docker run` arguments.
///
/// When `pre_run` is `Some`, the referenced executable is run and its stdout is parsed as a TOML
/// array of strings (e.g. `["--volume", "/host:/container"]`). If `pre_run` is `None`, an empty
/// vector is returned.
pub(crate) fn execute_pre_run_hook(pre_run: Option<&str>) -> Result<Vec<String>> {
    execute_hook(pre_run, "Pre-run")
}

/// Execute a hook executable and return the parsed extra arguments.
///
/// `hook_label` is used in error messages to identify the hook (e.g. "Pre-build", "Pre-run").
fn execute_hook(hook_path: Option<&str>, hook_label: &str) -> Result<Vec<String>> {
    let Some(path) = hook_path else {
        return Ok(Vec::new());
    };

    debug!(hook_label, path, "Executing hook");

    let output = Command::new(path)
        .output()
        .with_context(|| format!("Failed to execute {hook_label} hook {path:?}"))?;

    if !output.status.success() {
        let stderr_text = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{hook_label} hook {path:?} exited with status {status}:\n{stderr_text}",
            status = output.status,
        );
    }

    let stdout_text = String::from_utf8(output.stdout)
        .with_context(|| format!("{hook_label} hook stdout is not valid UTF-8"))?;

    let args = parse_hook_output(&stdout_text).with_context(|| {
        format!(
            "Failed to parse {hook_label} hook output as a TOML list of strings: {stdout_text:?}"
        )
    })?;

    debug!(hook_label, ?args, "Hook produced extra arguments");

    Ok(args)
}

/// Parse the raw stdout of a hook into a list of extra arguments.
///
/// A bare TOML array is not a valid TOML document (which requires key-value pairs at the top
/// level). The output is wrapped in a synthetic key so the `toml` crate can parse it.
fn parse_hook_output(stdout: &str) -> Result<Vec<String>, TomlError> {
    #[derive(Deserialize)]
    struct HookOutput {
        args: Vec<String>,
    }

    let wrapped = format!("args = {}", stdout.trim());
    let parsed: HookOutput = toml::from_str(&wrapped)?;
    Ok(parsed.args)
}

#[cfg(test)]
mod tests;
