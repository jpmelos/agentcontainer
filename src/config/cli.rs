//! CLI argument definitions and parsing helpers.

use super::entries::{BuildArgumentEntry, EnvironmentVariableEntry, VolumeEntry};
use super::error::ConfigError;
use clap::{Parser, Subcommand};
use std::collections::HashMap;

/// CLI arguments.
#[expect(
    clippy::struct_excessive_bools,
    reason = "These flags directly mirror distinct CLI flags; a state machine would be \
        inappropriate here."
)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "Fields need `pub(super)` visibility so that `get_config` in the parent module \
        can read them."
)]
#[derive(Parser, Debug)]
#[command(about, version)]
pub(crate) struct CliArgs {
    /// Path to the Dockerfile. Defaults to `.agentcontainer/Dockerfile`.
    #[arg(long)]
    pub(super) dockerfile: Option<String>,

    /// Directory used as the Docker build context. Defaults to the current working directory.
    #[arg(long)]
    pub(super) build_context: Option<String>,

    /// Build argument as "KEY=value", "KEY" (inherit from host), or "!KEY" (remove). Repeatable.
    #[arg(long = "build-arg")]
    pub(super) build_arguments: Vec<String>,

    /// Path to an executable to run before `docker build`. Repeatable. Each hook's stdout is
    /// parsed as a TOML list of extra arguments to pass to the `docker build` command (e.g.
    /// `["--label", "foo=bar"]`). Values accumulate across config sources.
    #[arg(long = "pre-build")]
    pub(super) pre_build: Vec<String>,

    /// Name used in Docker image tag. Defaults to the current working directory's name.
    #[arg(long)]
    pub(super) project_name: Option<String>,

    /// Username for image tag. Defaults to the current user's username.
    #[arg(long)]
    pub(super) username: Option<String>,

    /// Docker build `--target`. Also appended to image name. If not provided, no target is used.
    #[arg(long)]
    pub(super) target: Option<String>,

    /// Use stale image if build fails.
    #[arg(long)]
    pub(super) allow_stale: bool,

    /// Force rebuild regardless of staleness.
    #[arg(long)]
    pub(super) force_rebuild: bool,

    /// Pass `--no-cache` to `docker build`.
    #[arg(long)]
    pub(super) no_build_cache: bool,

    /// Skip rebuild; error if no image exists.
    #[arg(long)]
    pub(super) no_rebuild: bool,

    /// Volume as "host:container", "/path" (same path), or "!/path" (remove). Repeatable.
    #[arg(long = "volume", short = 'v')]
    pub(super) volumes: Vec<String>,

    /// Environment variable as "KEY=value", "KEY" (inherit), or "!KEY" (remove). Repeatable.
    #[arg(long = "env", short = 'e')]
    pub(super) environment_variables: Vec<String>,

    /// Path to an executable to run before `docker run`. Repeatable. Each hook's stdout is
    /// parsed as a TOML list of extra arguments to pass to the `docker run` command (e.g.
    /// `["--network", "host"]`). Values accumulate across config sources.
    #[arg(long = "pre-run")]
    pub(super) pre_run: Vec<String>,

    #[command(subcommand)]
    pub(super) command: Command,
}

/// Subcommands.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum Command {
    /// Print the resolved configuration.
    Config,
    /// Build the agent container image.
    Build,
    /// Run the agent container.
    Run {
        /// Arguments to pass through to the container entrypoint. Must come after `--`.
        #[arg(last = true)]
        container_args: Vec<String>,
    },
}

/// Parse the list of `--build-arg` CLI arguments into a `HashMap<String, BuildArgumentEntry>`.
///
/// Accepted formats:
/// - `"KEY=value"` → `("KEY", Value("value"))` (split on the first `=`)
/// - `"KEY"` (no `=`) → `("KEY", Inherit)`
/// - `"!KEY"` → `("KEY", Remove)`
pub(super) fn parse_cli_build_arguments(
    raw_build_args: &[String],
) -> Result<HashMap<String, BuildArgumentEntry>, ConfigError> {
    let mut build_args = HashMap::new();
    for raw in raw_build_args {
        if raw.is_empty() {
            return Err(ConfigError::InvalidBuildArgument { value: raw.clone() });
        }
        if let Some(key) = raw.strip_prefix('!') {
            if !is_valid_env_var_key(key) {
                return Err(ConfigError::InvalidBuildArgumentKey {
                    key: String::from(key),
                });
            }
            build_args.insert(String::from(key), BuildArgumentEntry::Remove);
        } else if let Some((key, value)) = raw.split_once('=') {
            if !is_valid_env_var_key(key) {
                return Err(ConfigError::InvalidBuildArgumentKey {
                    key: String::from(key),
                });
            }
            build_args.insert(
                String::from(key),
                BuildArgumentEntry::Value(String::from(value)),
            );
        } else if !is_valid_env_var_key(raw) {
            return Err(ConfigError::InvalidBuildArgumentKey { key: raw.clone() });
        } else {
            build_args.insert(raw.clone(), BuildArgumentEntry::Inherit);
        }
    }
    Ok(build_args)
}

/// Parse the list of `--volume` CLI arguments into a `HashMap<String, VolumeEntry>`.
///
/// Accepted formats:
/// - `"/host:/container"` → `("/container", Active("/host"))`
/// - `"/path"` (no colon) → `("/path", SamePath)` — mount at the same path in the container.
/// - `"!/container"` → `("/container", Remove)`
pub(super) fn parse_cli_volumes(
    raw_volumes: &[String],
) -> Result<HashMap<String, VolumeEntry>, ConfigError> {
    let mut volumes = HashMap::new();
    for raw in raw_volumes {
        if raw.is_empty() {
            return Err(ConfigError::InvalidVolume { value: raw.clone() });
        }
        if let Some(container_path) = raw.strip_prefix('!') {
            if container_path.is_empty() || container_path.contains(':') {
                return Err(ConfigError::InvalidVolume { value: raw.clone() });
            }
            volumes.insert(String::from(container_path), VolumeEntry::Remove);
        } else if let Some((host_path, container_path)) = raw.rsplit_once(':') {
            if host_path.is_empty() || host_path.contains(':') || container_path.is_empty() {
                return Err(ConfigError::InvalidVolume { value: raw.clone() });
            }
            volumes.insert(
                String::from(container_path),
                VolumeEntry::Active(String::from(host_path)),
            );
        } else {
            volumes.insert(raw.clone(), VolumeEntry::SamePath);
        }
    }
    Ok(volumes)
}

/// Parse the list of `--env` CLI arguments into a
/// `HashMap<String, EnvironmentVariableEntry>`.
///
/// Accepted formats:
/// - `"KEY=value"` → `("KEY", Value("value"))` (split on the first `=`)
/// - `"KEY"` (no `=`) → `("KEY", Inherit)`
/// - `"!KEY"` → `("KEY", Remove)`
pub(super) fn parse_cli_environment_variables(
    raw_env_vars: &[String],
) -> Result<HashMap<String, EnvironmentVariableEntry>, ConfigError> {
    let mut env_vars = HashMap::new();
    for raw in raw_env_vars {
        if raw.is_empty() {
            return Err(ConfigError::InvalidEnvironmentVariable { value: raw.clone() });
        }
        if let Some(key) = raw.strip_prefix('!') {
            if !is_valid_env_var_key(key) {
                return Err(ConfigError::InvalidEnvironmentVariableKey {
                    key: String::from(key),
                });
            }
            env_vars.insert(String::from(key), EnvironmentVariableEntry::Remove);
        } else if let Some((key, value)) = raw.split_once('=') {
            if !is_valid_env_var_key(key) {
                return Err(ConfigError::InvalidEnvironmentVariableKey {
                    key: String::from(key),
                });
            }
            env_vars.insert(
                String::from(key),
                EnvironmentVariableEntry::Value(String::from(value)),
            );
        } else if !is_valid_env_var_key(raw) {
            return Err(ConfigError::InvalidEnvironmentVariableKey { key: raw.clone() });
        } else {
            env_vars.insert(raw.clone(), EnvironmentVariableEntry::Inherit);
        }
    }
    Ok(env_vars)
}

/// Check whether a string is a valid environment variable key.
///
/// Valid keys match the POSIX pattern `[A-Za-z_][A-Za-z0-9_]*`.
pub(super) fn is_valid_env_var_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
