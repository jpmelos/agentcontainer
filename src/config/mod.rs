//! Process application configuration via configuration files, environment variables, and CLI
//! arguments.

use crate::utils::slugify::slugify_or_unknown;
use clap::{Parser, Subcommand};
use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};
use thiserror::Error;

/// Default path to the Dockerfile.
fn default_dockerfile() -> String {
    String::from(".agentcontainer/Dockerfile")
}

/// Default project name, derived from the last component of the current working directory.
fn default_project_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(String::from)
        })
        .unwrap_or_else(|| String::from("unknown"))
}

/// Default username, obtained from the OS.
fn default_username() -> String {
    whoami::username().unwrap_or_else(|_| String::from("unknown"))
}

/// Application configuration.
#[expect(
    clippy::struct_excessive_bools,
    reason = "These flags directly mirror distinct CLI flags; a state machine would be \
        inappropriate here."
)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "Fields need `pub(crate)` visibility so that other code can read them."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
    /// Path to the Dockerfile.
    #[serde(default = "default_dockerfile")]
    pub(crate) dockerfile: String,

    /// Project name used in Docker image tag.
    #[serde(default = "default_project_name")]
    pub(crate) project_name: String,

    /// Username for the image tag and the `USERNAME` build argument.
    #[serde(default = "default_username")]
    pub(crate) username: String,

    /// Docker build `--target`. Also appended to the image name when set.
    #[serde(default)]
    pub(crate) target: Option<String>,

    /// Use a stale image if the build fails.
    #[serde(default)]
    pub(crate) allow_stale: bool,

    /// Force rebuild regardless of staleness.
    #[serde(default)]
    pub(crate) force_rebuild: bool,

    /// Pass `--no-cache` to `docker build`.
    #[serde(default)]
    pub(crate) no_build_cache: bool,

    /// Skip rebuild; error if no image exists.
    #[serde(default)]
    pub(crate) no_rebuild: bool,
}

impl Config {
    /// Return the Docker image name for this configuration.
    ///
    /// The `username`, `project_name`, and `target` fields are slugified before being embedded in
    /// the tag so that the result is always a valid Docker image name. The validity of `target` is
    /// guaranteed by `get_config`, which rejects any value that contains no alphanumeric
    /// characters before returning a `Config`.
    pub(crate) fn image_name(&self) -> String {
        let slugified_username = slugify_or_unknown(&self.username);
        let slugified_project_name = slugify_or_unknown(&self.project_name);

        self.target.as_ref().map_or_else(
            || format!("agentcontainer-{slugified_username}-{slugified_project_name}:latest"),
            |target| {
                let slugified_target = slugify_or_unknown(target);
                format!(
                    "agentcontainer-{slugified_username}-{slugified_project_name}\
                    -{slugified_target}:latest"
                )
            },
        )
    }
}

/// CLI arguments.
#[expect(
    clippy::struct_excessive_bools,
    reason = "These flags directly mirror distinct CLI flags; a state machine would be \
        inappropriate here."
)]
#[derive(Parser, Debug)]
#[command(about, version)]
pub(crate) struct CliArgs {
    /// Path to the Dockerfile.
    #[arg(long, global = true)]
    dockerfile: Option<String>,

    /// Name used in Docker image tag.
    #[arg(long, global = true)]
    project_name: Option<String>,

    /// Username for image tag and `USERNAME` build argument.
    #[arg(long, global = true)]
    username: Option<String>,

    /// Docker build `--target`. Also appended to image name.
    #[arg(long, global = true)]
    target: Option<String>,

    /// Use stale image if build fails.
    #[arg(long, global = true)]
    allow_stale: bool,

    /// Force rebuild regardless of staleness.
    #[arg(long, global = true)]
    force_rebuild: bool,

    /// Pass `--no-cache` to `docker build`.
    #[arg(long, global = true)]
    no_build_cache: bool,

    /// Skip rebuild; error if no image exists.
    #[arg(long, global = true)]
    no_rebuild: bool,

    #[command(subcommand)]
    command: Command,
}

/// Subcommands.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum Command {
    /// Print the resolved configuration.
    Config,
    /// Build the agent container image.
    Build,
}

/// Errors that can be returned from `get_config`.
#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    /// `force_rebuild` and `no_rebuild` were both set, which is contradictory.
    #[error("`force_rebuild` and `no_rebuild` are mutually exclusive")]
    ConflictingRebuildFlags,
    /// Figment failed to extract the configuration.
    #[error("Failed to load configuration: {0}")]
    Extract(Box<figment::Error>),
    /// The `target` value contains no alphanumeric characters and cannot be slugified.
    #[error("Invalid `target` value {target:?}: contains no alphanumeric characters")]
    InvalidTarget {
        /// The raw target value that failed slugification.
        target: String,
    },
}

impl From<figment::Error> for ConfigError {
    fn from(error: figment::Error) -> Self {
        Self::Extract(Box::new(error))
    }
}

/// Get the configuration from all sources and the command to execute.
///
/// Configuration sources are merged in the following order (lowest to highest priority):
/// - `~/.config/agentcontainer/config.toml`
/// - `~/.agentcontainer.toml`
/// - `.agentcontainer/config.toml`
/// - `.agentcontainer/config.local.toml`
/// - Environment variables prefixed by `AGENTCONTAINER_`.
/// - CLI arguments.
pub(crate) fn get_config<'cli_args>(
    home_dir: &'_ str,
    cli_args: &'cli_args CliArgs,
) -> Result<(&'cli_args Command, Config), ConfigError> {
    // Build the configuration figment by merging sources in order of priority.
    let mut figment = Figment::new()
        .merge(Toml::file(format!(
            "{home_dir}/.config/agentcontainer/config.toml"
        )))
        .merge(Toml::file(format!("{home_dir}/.agentcontainer.toml")))
        .merge(Toml::file(".agentcontainer/config.toml"))
        .merge(Toml::file(".agentcontainer/config.local.toml"))
        .merge(Env::prefixed("AGENTCONTAINER_"));

    // Only merge CLI arguments that were actually provided to avoid overriding config values with
    // `None`.
    macro_rules! merge_cli_args {
        ($cli_args:expr, $figment:expr, $($field:ident),+ $(,)?) => {{
            let mut cli_config = HashMap::new();
            $(
                if let Some(value) = $cli_args.$field.as_ref() {
                    cli_config.insert(stringify!($field), value);
                }
            )+
            if !cli_config.is_empty() {
                $figment = $figment.merge(Serialized::defaults(cli_config));
            }
        }};
    }

    // Only merge `bool` CLI flags when they are `true`; a `false` means the user did not pass the
    // flag, and should not override a `true` from a lower-priority config source.
    macro_rules! merge_bool_cli_args {
        ($cli_args:expr, $figment:expr, $($field:ident),+ $(,)?) => {{
            let mut cli_config: HashMap<&str, bool> = HashMap::new();
            $(
                if $cli_args.$field {
                    cli_config.insert(stringify!($field), true);
                }
            )+
            if !cli_config.is_empty() {
                $figment = $figment.merge(Serialized::defaults(cli_config));
            }
        }};
    }

    // Must list all fields from `CliArgs` that we want to merge into configuration.
    merge_cli_args!(
        cli_args,
        figment,
        dockerfile,
        project_name,
        username,
        target
    );
    merge_bool_cli_args!(
        cli_args,
        figment,
        allow_stale,
        force_rebuild,
        no_build_cache,
        no_rebuild,
    );

    // Extract the configuration.
    let config: Config = figment.extract()?;

    // Validate the resolved configuration.
    if config.force_rebuild && config.no_rebuild {
        return Err(ConfigError::ConflictingRebuildFlags);
    }
    if let Some(ref target) = config.target
        && !target.chars().any(|character| character.is_alphanumeric())
    {
        return Err(ConfigError::InvalidTarget {
            target: target.clone(),
        });
    }

    Ok((&cli_args.command, config))
}

#[cfg(test)]
mod tests;
