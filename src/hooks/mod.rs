//! Hooks for injecting extra arguments into `docker build` and `docker run` commands.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use std::process::Command;
use toml::de::Error as TomlError;
use tracing::debug;

/// Execute all pre-build hooks and return their concatenated extra `docker build` arguments.
///
/// Each hook in `pre_build` is executed in order. Their outputs are parsed as TOML arrays of
/// strings (e.g. `["--label", "foo=bar"]`) and concatenated in the same order as the hooks.
pub(crate) fn execute_pre_build_hooks(pre_build: &[String]) -> Result<Vec<String>> {
    execute_hooks(pre_build, "Pre-build")
}

/// Execute all pre-run hooks and return their concatenated extra `docker run` arguments.
///
/// Each hook in `pre_run` is executed in order. Their outputs are parsed as TOML arrays of
/// strings (e.g. `["--volume", "/host:/container"]`) and concatenated in the same order as the
/// hooks.
pub(crate) fn execute_pre_run_hooks(pre_run: &[String]) -> Result<Vec<String>> {
    execute_hooks(pre_run, "Pre-run")
}

/// Execute a list of hook executables and return the concatenated parsed extra arguments.
///
/// `hook_label` is used in error messages to identify the hook kind (e.g. "Pre-build",
/// "Pre-run"). Hooks are executed in order and their results are concatenated.
fn execute_hooks(hook_paths: &[String], hook_label: &str) -> Result<Vec<String>> {
    let mut all_args = Vec::new();
    for path in hook_paths {
        let args = execute_hook(path, hook_label)?;
        all_args.extend(args);
    }
    Ok(all_args)
}

/// Execute a single hook executable and return the parsed extra arguments.
///
/// `hook_label` is used in error messages to identify the hook kind (e.g. "Pre-build",
/// "Pre-run").
fn execute_hook(path: &str, hook_label: &str) -> Result<Vec<String>> {
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
