//! Run the agent container.

use crate::config::{Config, EnvironmentVariableEntry, MountpointEntry};
use crate::utils::docker::DockerBackend;
use crate::utils::git::GitContext;
use std::convert::Infallible;
use std::io::Error as IoError;
use std::path::Path;
use thiserror::Error;

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

/// Assemble the argument list for `docker run`.
///
/// This is a pure function that produces the full argument vector, making it easy to test without
/// actually running Docker.
fn build_docker_run_args(
    config: &Config,
    uid: u32,
    gid: u32,
    current_dir: &str,
    main_worktree: Option<&Path>,
    random_suffix: u32,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Subcommand and fixed flags.
    args.extend(["run", "-t", "-i", "--init", "--rm"].map(String::from));

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

    // Automatic mount: current directory.
    args.push(String::from("-v"));
    args.push(format!("{current_dir}:{current_dir}"));

    // Worktree mount, if present.
    if let Some(worktree) = main_worktree {
        let worktree_str = worktree.display();
        args.push(String::from("-v"));
        args.push(format!("{worktree_str}:{worktree_str}"));
    }

    // Config mountpoints.
    for (container_path, entry) in &config.mountpoints {
        match *entry {
            MountpointEntry::Active(ref host_path) => {
                args.push(String::from("-v"));
                args.push(format!("{host_path}:{container_path}"));
            }
            MountpointEntry::SamePath => {
                args.push(String::from("-v"));
                args.push(format!("{container_path}:{container_path}"));
            }
            MountpointEntry::Remove => {
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

    // Image name (must be last).
    args.push(config.get_image_name());

    args
}

/// Run the agent container.
///
/// Assembles a `docker run` command and replaces the current process via `exec`. On success, the
/// current process is replaced and this function never returns. On failure, returns a `RunError`.
pub(crate) fn run(
    config: &Config,
    docker_backend: &impl DockerBackend,
    git_context: &impl GitContext,
    uid: u32,
    gid: u32,
    current_dir: &str,
    random_suffix: u32,
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
    );

    docker_backend
        .exec_docker_run(&args)
        .map_err(RunError::Exec)
}

#[cfg(test)]
mod tests;
