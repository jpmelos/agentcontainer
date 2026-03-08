//! Hooks for injecting extra arguments into `docker build` and `docker run` commands.
//!
//! Hooks form a pipeline: each hook receives the path to a temporary file as its first argument.
//! The file contains the current arguments as a TOML document (`args = ["--flag", "value", ...]`).
//! The hook returns a (possibly modified) list on `stdout` in the same format. The output of one
//! hook becomes the input to the next. The final `args` from the last hook is what gets passed to
//! the Docker command.

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::Write as _;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;
use toml::de::Error as TomlError;
use tracing::debug;

/// Execute all pre-build hooks and return the final `docker build` arguments.
///
/// `initial_args` contains the hookable arguments computed from the configuration (e.g.
/// `--build-arg`). Each hook in `pre_build` is executed in order, forming a pipeline: the output
/// of one hook becomes the input of the next.
pub(crate) fn execute_pre_build_hooks(
    pre_build: &[String],
    initial_args: Vec<String>,
) -> Result<Vec<String>> {
    execute_hooks(pre_build, "pre-build", initial_args)
}

/// Execute all pre-run hooks and return the final extra `docker run` arguments.
///
/// `initial_args` contains volumes and environment variables from the configuration. It will be
/// empty if none are configured. Base `docker run` flags (like `--init`, `--rm`, `--user`) are
/// not included. Each hook in `pre_run` is executed in order, forming a pipeline.
pub(crate) fn execute_pre_run_hooks(
    pre_run: &[String],
    initial_args: Vec<String>,
) -> Result<Vec<String>> {
    execute_hooks(pre_run, "pre-run", initial_args)
}

/// Execute a list of hook executables as a pipeline and return the final arguments.
///
/// `hook_label` is used in error messages to identify the hook kind (e.g. "pre-build",
/// "pre-run"). Hooks are executed in order; each receives the path to a temporary file containing
/// the current arguments and returns the next set of arguments on `stdout`.
fn execute_hooks(
    hook_paths: &[String],
    hook_label: &str,
    initial_args: Vec<String>,
) -> Result<Vec<String>> {
    let mut current_args = initial_args;
    for path in hook_paths {
        current_args = execute_hook(path, hook_label, &current_args)?;
    }
    Ok(current_args)
}

/// Execute a single hook executable and return the parsed arguments from its stdout.
///
/// The hook receives the path to a temporary file as its first argument. The file contains the
/// current arguments as a TOML document (`args = [...]`). The hook must return a TOML document
/// with the same shape on `stdout`.
fn execute_hook(path: &str, hook_label: &str, current_args: &[String]) -> Result<Vec<String>> {
    let input = serialize_hook_input(current_args);
    debug!(hook_label, path, %input, "Executing hook");

    // Write the input to a temporary file so the hook can read it.
    let mut tmpfile = NamedTempFile::new().with_context(|| {
        format!("Failed to create temporary file for {hook_label} hook {path:?}")
    })?;
    tmpfile
        .write_all(input.as_bytes())
        .with_context(|| format!("Failed to write input file for {hook_label} hook {path:?}"))?;

    let output = Command::new(path)
        .arg(tmpfile.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
        format!("Failed to parse {hook_label} hook output as TOML with `args` key: {stdout_text:?}")
    })?;

    debug!(hook_label, ?args, "Hook produced arguments");

    Ok(args)
}

/// Serialize arguments into the TOML format expected by hooks in their input file.
///
/// Produces a document like `args = ["--flag", "value"]`.
fn serialize_hook_input(args: &[String]) -> String {
    #[derive(Serialize)]
    struct HookInput<'args> {
        args: &'args [String],
    }

    toml::to_string(&HookInput { args }).expect("Serialization of hook input should not fail")
}

/// Parse the raw stdout of a hook into a list of arguments.
///
/// Hooks output a TOML document with a single key `args` containing a list of strings, e.g.
/// `args = ["--network", "host"]`.
fn parse_hook_output(stdout: &str) -> Result<Vec<String>, TomlError> {
    #[derive(Deserialize)]
    struct HookOutput {
        args: Vec<String>,
    }

    let parsed: HookOutput = toml::from_str(stdout.trim())?;
    Ok(parsed.args)
}

#[cfg(test)]
mod tests;
