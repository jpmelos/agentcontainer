//! Run the agent container.

use crate::config::{Config, EnvironmentVariableEntry, VolumeEntry};
use crate::utils::docker::DockerBackend;
use crate::utils::git::GitContext;
use std::convert::Infallible;
use std::io::Error as IoError;
use std::path::Path;
use thiserror::Error;
use tracing::debug;

/// Errors that can be returned from `run`.
#[derive(Debug, Error)]
pub(crate) enum RunError {
    /// Failed to detect Git worktree information.
    #[error("Failed to detect Git worktree: {0}")]
    GitWorktree(#[source] anyhow::Error),
    /// The `exec` system call failed.
    #[error("Failed to exec `docker run`: {0}")]
    Exec(#[source] IoError),
}

/// Run the agent container.
///
/// Assembles a `docker run` command and replaces the current process via `exec`. On success, the
/// current process is replaced and this function never returns. On failure, returns a `RunError`.
#[expect(
    clippy::too_many_arguments,
    reason = "Each parameter represents a distinct concern (config, backends, process identity, \
        filesystem context, container arguments); grouping them into a struct would add \
        indirection without improving clarity."
)]
pub(crate) fn run(
    config: &Config,
    docker_backend: &impl DockerBackend,
    git_context: &impl GitContext,
    uid: u32,
    gid: u32,
    current_dir: &str,
    random_suffix: u32,
    container_args: &[String],
    stdin_is_terminal: bool,
    pre_run_extra_args: &[String],
) -> Result<Infallible, RunError> {
    let main_worktree = git_context
        .main_worktree_root(Path::new(current_dir))
        .map_err(RunError::GitWorktree)?;

    let args = build_docker_run_args(
        config,
        uid,
        gid,
        current_dir,
        main_worktree.as_deref(),
        random_suffix,
        container_args,
        stdin_is_terminal,
        pre_run_extra_args,
    );

    debug!(?args, "Assembled `docker run` arguments");

    let image_name = config.get_image_name();
    debug!(%image_name, "Running container");

    docker_backend
        .exec_docker_run(&args)
        .map_err(RunError::Exec)
}

/// Assemble the argument list for `docker run`.
///
/// This is a pure function that produces the full argument vector, making it easy to test without
/// actually running Docker.
#[expect(
    clippy::too_many_arguments,
    reason = "Each parameter represents a distinct, independently varying input to the command \
        assembly; bundling them would obscure the pure-function contract."
)]
fn build_docker_run_args(
    config: &Config,
    uid: u32,
    gid: u32,
    current_dir: &str,
    main_worktree: Option<&Path>,
    random_suffix: u32,
    container_args: &[String],
    stdin_is_terminal: bool,
    pre_run_extra_args: &[String],
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Subcommand and fixed flags.
    args.extend(["run", "--init", "--rm"].map(String::from));

    // Only allocate a pseudo-TTY and keep stdin open when the host's
    // stdin is actually a TTY. Without this guard, piped or scripted
    // invocations cause Docker to hang or emit spurious warnings.
    if stdin_is_terminal {
        args.push(String::from("-t"));
        args.push(String::from("-i"));
    }

    // User mapping.
    args.push(String::from("--user"));
    args.push(format!("{uid}:{gid}"));
    args.push(String::from("--group-add"));
    args.push(String::from("0"));

    // Container name.
    let name = config.get_container_name(random_suffix);
    args.push(String::from("--name"));
    args.push(name);

    // Working directory.
    args.push(String::from("-w"));
    args.push(String::from(current_dir));

    // Automatic volume: current directory.
    args.push(String::from("-v"));
    args.push(format!("{current_dir}:{current_dir}"));

    // Worktree volume, if present.
    if let Some(worktree) = main_worktree {
        let worktree_str = worktree.display();
        args.push(String::from("-v"));
        args.push(format!("{worktree_str}:{worktree_str}"));
    }

    // Config volumes.
    for (container_path, entry) in &config.volumes {
        match *entry {
            VolumeEntry::Active(ref host_path) => {
                args.push(String::from("-v"));
                args.push(format!("{host_path}:{container_path}"));
            }
            VolumeEntry::SamePath => {
                args.push(String::from("-v"));
                args.push(format!("{container_path}:{container_path}"));
            }
            VolumeEntry::Remove => {
                unreachable!(
                    "`Remove` entries are stripped by `clean_config` before `run` is called."
                )
            }
        }
    }

    // Config environment variables.
    for (key, entry) in &config.environment_variables {
        match *entry {
            EnvironmentVariableEntry::Value(ref value) => {
                args.push(String::from("-e"));
                args.push(format!("{key}={value}"));
            }
            EnvironmentVariableEntry::Inherit => {
                args.push(String::from("-e"));
                args.push(key.clone());
            }
            EnvironmentVariableEntry::Remove => {
                unreachable!(
                    "`Remove` entries are stripped by `clean_config` before `run` is called."
                )
            }
        }
    }

    // Extra arguments from the pre-run hook.
    args.extend_from_slice(pre_run_extra_args);

    // Image name (must come after all Docker flags, before container arguments).
    args.push(config.get_image_name());

    // Passthrough arguments for the container entrypoint.
    args.extend_from_slice(container_args);

    args
}

#[cfg(test)]
mod tests;
