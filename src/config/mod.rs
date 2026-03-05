//! Process application configuration via configuration files, environment variables, and CLI
//! arguments.

mod merging_provider;

use crate::utils::slugify::slugify_or_unknown;
use clap::{Parser, Subcommand};
use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use merging_provider::MergingProvider;
use serde::de::{Error as DeError, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt::{Formatter, Result as FmtResult};
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

/// A mountpoint entry: either an active host path, or a removal sentinel.
///
/// TOML/CLI user representation: a string = host path; `false` = remove. `true` is rejected by
/// the custom deserializer.
#[derive(Debug, Clone)]
pub(crate) enum MountpointEntry {
    /// The host path to mount at the container path key.
    Active(String),
    /// Removal sentinel.
    Remove,
}

impl Serialize for MountpointEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Active(ref host_path) => serializer.serialize_str(host_path),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for MountpointEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MountpointEntryVisitor;

        impl Visitor<'_> for MountpointEntryVisitor {
            type Value = MountpointEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str("a host path string or `false` as a removal sentinel")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(MountpointEntry::Active(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(MountpointEntry::Active(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Err(E::invalid_value(
                        Unexpected::Bool(true),
                        &"a host path string or `false` as a removal sentinel",
                    ))
                } else {
                    Ok(MountpointEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(MountpointEntryVisitor)
    }
}

/// An environment variable entry.
///
/// TOML/CLI user representation: a string = literal value; `true` = inherit from host; `false` =
/// remove / suppress.
#[derive(Debug, Clone)]
pub(crate) enum EnvironmentVariableEntry {
    /// A literal value to pass into the container.
    Value(String),
    /// Inherit the variable from the host environment.
    Inherit,
    /// Remove / suppress the variable in the container.
    Remove,
}

impl Serialize for EnvironmentVariableEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Value(ref value) => serializer.serialize_str(value),
            Self::Inherit => serializer.serialize_bool(true),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for EnvironmentVariableEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EnvironmentVariableEntryVisitor;

        impl Visitor<'_> for EnvironmentVariableEntryVisitor {
            type Value = EnvironmentVariableEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str("a string value, `true` to inherit, or `false` to remove")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(EnvironmentVariableEntry::Value(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(EnvironmentVariableEntry::Value(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Ok(EnvironmentVariableEntry::Inherit)
                } else {
                    Ok(EnvironmentVariableEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(EnvironmentVariableEntryVisitor)
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,

    /// Use a stale image if the build fails.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) allow_stale: bool,

    /// Force rebuild regardless of staleness.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) force_rebuild: bool,

    /// Pass `--no-cache` to `docker build`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) no_build_cache: bool,

    /// Skip rebuild; error if no image exists.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) no_rebuild: bool,

    /// Mountpoints to set up in the container.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) mountpoints: HashMap<String, MountpointEntry>,

    /// Environment variables to pass to the container.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) environment_variables: HashMap<String, EnvironmentVariableEntry>,
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

    /// Mountpoint in "host:container" format. Repeatable. Prefix with "!" to remove.
    #[arg(long = "mountpoint", global = true)]
    mountpoints: Vec<String>,

    /// Environment variable as "KEY=value", "KEY" (inherit), or "!KEY" (remove). Repeatable.
    #[arg(long = "environment-variable", global = true)]
    environment_variables: Vec<String>,

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
    /// A mountpoint value could not be parsed (bad CLI format, or `true` used as a value in
    /// TOML/env).
    #[error("Invalid mountpoint value {value:?}: expected \"/host:/container\" or \"!/container\"")]
    InvalidMountpoint {
        /// The raw value that failed parsing.
        value: String,
    },
    /// An environment variable CLI argument could not be parsed.
    #[error(
        "Invalid environment variable value {value:?}: expected \"KEY=value\", \"KEY\", or \
         \"!KEY\""
    )]
    InvalidEnvironmentVariable {
        /// The raw value that failed parsing.
        value: String,
    },
    /// An environment variable key is not a valid identifier.
    #[error(
        "Invalid environment variable key {key:?}: must start with a letter or underscore and \
         contain only ASCII letters, digits, and underscores"
    )]
    InvalidEnvironmentVariableKey {
        /// The key that failed validation.
        key: String,
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
    // Only merge scalar CLI arguments that were actually provided to avoid overriding config values
    // with `None`.
    macro_rules! merge_string_cli_args {
        ($cli_args:expr, $providers:expr, $($field:ident),+ $(,)?) => {{
            let mut cli_config: HashMap<&str, String> = HashMap::new();
            $(
                if let Some(value) = $cli_args.$field.as_ref() {
                    cli_config.insert(stringify!($field), value.clone());
                }
            )+
            if !cli_config.is_empty() {
                $providers.push(Box::new(Serialized::defaults(cli_config)));
            }
        }};
    }

    // Only merge `bool` CLI flags when they are `true`; a `false` means the user did not pass the
    // flag, and should not override a `true` from a lower-priority config source.
    macro_rules! merge_bool_cli_args {
        ($cli_args:expr, $providers:expr, $($field:ident),+ $(,)?) => {{
            let mut cli_config: HashMap<&str, bool> = HashMap::new();
            $(
                if $cli_args.$field {
                    cli_config.insert(stringify!($field), true);
                }
            )+
            if !cli_config.is_empty() {
                $providers.push(Box::new(Serialized::defaults(cli_config)));
            }
        }};
    }

    // Parse CLI `--mountpoint` args into a map.
    let cli_mountpoints = parse_cli_mountpoints(&cli_args.mountpoints)?;

    // Parse CLI `--environment-variable` args into a map.
    let cli_env_vars = parse_cli_environment_variables(&cli_args.environment_variables)?;

    // Build the provider list in priority order (lowest to highest).
    let mut providers: Vec<Box<dyn figment::Provider>> = vec![
        Box::new(Toml::file(format!(
            "{home_dir}/.config/agentcontainer/config.toml"
        ))),
        Box::new(Toml::file(format!("{home_dir}/.agentcontainer.toml"))),
        Box::new(Toml::file(".agentcontainer/config.toml")),
        Box::new(Toml::file(".agentcontainer/config.local.toml")),
        Box::new(Env::prefixed("AGENTCONTAINER_")),
    ];

    // CLI dict args (mountpoints and environment_variables) are combined into a single provider so
    // they travel together at the same priority level.
    {
        // We build a combined owned config struct to hold both maps so the provider satisfies the
        // `'static` lifetime bound required by `Box<dyn Provider>`.
        #[derive(Serialize)]
        struct CliDictArgs {
            mountpoints: HashMap<String, MountpointEntry>,
            environment_variables: HashMap<String, EnvironmentVariableEntry>,
        }
        if !cli_mountpoints.is_empty() || !cli_env_vars.is_empty() {
            let cli_dict_args = CliDictArgs {
                mountpoints: cli_mountpoints,
                environment_variables: cli_env_vars,
            };
            providers.push(Box::new(Serialized::defaults(cli_dict_args)));
        }
    }

    // CLI scalar and bool args, via the existing macros (only merged when actually provided/true).
    merge_string_cli_args!(
        cli_args,
        providers,
        dockerfile,
        project_name,
        username,
        target
    );
    merge_bool_cli_args!(
        cli_args,
        providers,
        allow_stale,
        force_rebuild,
        no_build_cache,
        no_rebuild,
    );

    // Extract the configuration using our custom merging provider.
    let mut config: Config = Figment::from(MergingProvider::new(providers)).extract()?;

    validate_config(&config)?;
    clean_config(&mut config);

    Ok((&cli_args.command, config))
}

/// Parse the list of `--mountpoint` CLI arguments into a `HashMap<String, MountpointEntry>`.
///
/// Accepted formats:
/// - `"/host:/container"` → `("/container", Active("/host"))`
/// - `"!/container"` → `("/container", Remove)`
fn parse_cli_mountpoints(
    raw_mountpoints: &[String],
) -> Result<HashMap<String, MountpointEntry>, ConfigError> {
    let mut mountpoints = HashMap::new();
    for raw in raw_mountpoints {
        if let Some(container_path) = raw.strip_prefix('!') {
            if container_path.is_empty() || container_path.contains(':') {
                return Err(ConfigError::InvalidMountpoint { value: raw.clone() });
            }
            mountpoints.insert(String::from(container_path), MountpointEntry::Remove);
        } else {
            let Some((host_path, container_path)) = raw.rsplit_once(':') else {
                return Err(ConfigError::InvalidMountpoint { value: raw.clone() });
            };
            if host_path.is_empty() || host_path.contains(':') || container_path.is_empty() {
                return Err(ConfigError::InvalidMountpoint { value: raw.clone() });
            }
            mountpoints.insert(
                String::from(container_path),
                MountpointEntry::Active(String::from(host_path)),
            );
        }
    }
    Ok(mountpoints)
}

/// Parse the list of `--environment-variable` CLI arguments into a
/// `HashMap<String, EnvironmentVariableEntry>`.
///
/// Accepted formats:
/// - `"KEY=value"` → `("KEY", Value("value"))` (split on the first `=`)
/// - `"KEY"` (no `=`) → `("KEY", Inherit)`
/// - `"!KEY"` → `("KEY", Remove)`
fn parse_cli_environment_variables(
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
fn is_valid_env_var_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Validate a fully-merged `Config`, returning the first error found.
fn validate_config(config: &Config) -> Result<(), ConfigError> {
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

    for key in config.environment_variables.keys() {
        if !is_valid_env_var_key(key) {
            return Err(ConfigError::InvalidEnvironmentVariableKey { key: key.clone() });
        }
    }

    Ok(())
}

/// Strip removal sentinels from a fully-merged `Config`.
///
/// Entries marked `Remove` instruct higher-priority layers to suppress a key inherited from a
/// lower-priority layer. Once merging is complete they carry no further information and are
/// removed so that callers see only the final, actionable set of entries.
fn clean_config(config: &mut Config) {
    config
        .mountpoints
        .retain(|_, entry| !matches!(entry, MountpointEntry::Remove));
    config
        .environment_variables
        .retain(|_, entry| !matches!(entry, EnvironmentVariableEntry::Remove));
}

#[cfg(test)]
mod tests;
