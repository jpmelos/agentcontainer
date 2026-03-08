//! Process application configuration via configuration files, environment variables, and CLI
//! arguments.

mod merging_provider;

use crate::utils::paths::{
    expand_and_resolve_path, has_tilde_user_prefix, is_relative_filesystem_path,
};
use crate::utils::slugify::slugify;
use clap::{Parser, Subcommand};
use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use merging_provider::MergingProvider;
use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt::{Formatter, Result as FmtResult};
use std::io::Error as IoError;
use std::{collections::HashMap, env};
use thiserror::Error as ThisError;
use tracing::{debug, trace, warn};

/// Default path to the Dockerfile.
fn default_dockerfile() -> String {
    String::from(".agentcontainer/Dockerfile")
}

/// Default Docker build context directory.
fn default_build_context() -> String {
    String::from(".")
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

/// A build argument entry: a literal value, a host-inherited value, or a removal sentinel.
///
/// In TOML and environment variables: a string = literal value; `true` = inherit from host
/// environment; `false` = remove.
///
/// On the CLI: `"KEY=value"` = literal value; `"KEY"` (no `=`) = inherit from host environment;
/// `"!KEY"` = remove.
#[derive(Debug, Clone)]
pub(crate) enum BuildArgumentEntry {
    /// A literal value to pass as a `--build-arg` to `docker build`.
    Value(String),
    /// Inherit the build argument value from the host environment.
    Inherit,
    /// Removal sentinel.
    Remove,
}

impl Serialize for BuildArgumentEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Value(ref value) => serializer.serialize_str(value),
            Self::Inherit => serializer.serialize_bool(true),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for BuildArgumentEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BuildArgumentEntryVisitor;

        impl Visitor<'_> for BuildArgumentEntryVisitor {
            type Value = BuildArgumentEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter
                    .write_str("a string value, `true` to inherit from host, or `false` to remove")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(BuildArgumentEntry::Value(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(BuildArgumentEntry::Value(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Ok(BuildArgumentEntry::Inherit)
                } else {
                    Ok(BuildArgumentEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(BuildArgumentEntryVisitor)
    }
}

/// A volume entry: an explicit host path, a same-path shorthand, or a removal sentinel.
///
/// In TOML and environment variables: a string = host path; `true` = mount at the same path as
/// the container path key; `false` = remove.
///
/// On the CLI: `"/host:/container"` = explicit mount; `"/path"` (no colon) = same-path shorthand;
/// `"!/container"` = remove.
#[derive(Debug, Clone)]
pub(crate) enum VolumeEntry {
    /// The host path to mount at the container path key.
    Active(String),
    /// Mount the container path key at the same path on the host.
    SamePath,
    /// Removal sentinel.
    Remove,
}

impl Serialize for VolumeEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Active(ref host_path) => serializer.serialize_str(host_path),
            Self::SamePath => serializer.serialize_bool(true),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for VolumeEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct VolumeEntryVisitor;

        impl Visitor<'_> for VolumeEntryVisitor {
            type Value = VolumeEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str(
                    "a host path string, `true` for same-path mount, or `false` to remove",
                )
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(VolumeEntry::Active(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(VolumeEntry::Active(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Ok(VolumeEntry::SamePath)
                } else {
                    Ok(VolumeEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(VolumeEntryVisitor)
    }
}

/// An environment variable entry.
///
/// In TOML and environment variables: a string = literal value; `true` = inherit from host;
/// `false` = remove / suppress.
///
/// On the CLI: `"KEY=value"` = literal value; `"KEY"` (no `=`) = inherit from host; `"!KEY"` =
/// remove.
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

    /// Directory used as the Docker build context.
    #[serde(default = "default_build_context")]
    pub(crate) build_context: String,

    /// Extra build arguments to pass to `docker build`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) build_arguments: HashMap<String, BuildArgumentEntry>,

    /// Path to an executable to run before `docker build`. Its stdout is parsed as a TOML list of
    /// extra arguments to pass to the `docker build` command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pre_build: Option<String>,

    /// Project name used in Docker image tag.
    #[serde(default = "default_project_name")]
    pub(crate) project_name: String,

    /// Username for the image tag.
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

    /// Volumes to set up in the container.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) volumes: HashMap<String, VolumeEntry>,

    /// Environment variables to pass to the container.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) environment_variables: HashMap<String, EnvironmentVariableEntry>,

    /// Path to an executable to run before `docker run`. Its stdout is parsed as a TOML list of
    /// extra arguments to pass to the `docker run` command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pre_run: Option<String>,
}

impl Config {
    /// Return the Docker image name for this configuration.
    ///
    /// The `username`, `project_name`, and `target` fields are slugified before being embedded in
    /// the tag so that the result is always a valid Docker image name. The validity of `username`,
    /// `project_name`, and `target` is guaranteed by `get_config`, which rejects any value whose
    /// slug is empty before returning a `Config`.
    pub(crate) fn get_image_name(&self) -> String {
        let slugified_username = slugify(&self.username);
        let slugified_project_name = slugify(&self.project_name);

        self.target.as_ref().map_or_else(
            || format!("agentcontainer_{slugified_username}_{slugified_project_name}:latest"),
            |target| {
                let slugified_target = slugify(target);
                format!(
                    "agentcontainer_{slugified_username}_{slugified_project_name}\
                    _{slugified_target}:latest"
                )
            },
        )
    }

    /// Generate a container name from the project name and a random suffix.
    ///
    /// The project name is slugified then truncated to 41 characters (with any trailing underscore
    /// removed after truncation). The final format is `agentcontainer_{name}_{suffix}`, which is
    /// at most 63 characters long, the limit for a container name.
    pub(crate) fn get_container_name(&self, random_suffix: u32) -> String {
        let slugified = slugify(&self.project_name);
        // `slugify` only produces ASCII characters (lowercase letters, digits, underscores), so
        // slicing at a byte offset is always on a character boundary.
        #[expect(
            clippy::string_slice,
            reason = "The slugified output is ASCII-only, so byte slicing is safe."
        )]
        let mut truncated: &str = if slugified.len() > 41 {
            &slugified[..41]
        } else {
            &slugified
        };
        truncated = truncated.trim_end_matches('_');
        format!("agentcontainer_{truncated}_{random_suffix}")
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
    /// Path to the Dockerfile. Defaults to `.agentcontainer/Dockerfile`.
    #[arg(long)]
    dockerfile: Option<String>,

    /// Directory used as the Docker build context. Defaults to the current working directory.
    #[arg(long)]
    build_context: Option<String>,

    /// Build argument as "KEY=value", "KEY" (inherit from host), or "!KEY" (remove). Repeatable.
    #[arg(long = "build-arg")]
    build_arguments: Vec<String>,

    /// Path to an executable to run before `docker build`. Its stdout is parsed as a TOML list of
    /// extra arguments to pass to the `docker build` command (e.g. `["--label", "foo=bar"]`).
    #[arg(long)]
    pre_build: Option<String>,

    /// Name used in Docker image tag. Defaults to the current working directory's name.
    #[arg(long)]
    project_name: Option<String>,

    /// Username for image tag. Defaults to the current user's username.
    #[arg(long)]
    username: Option<String>,

    /// Docker build `--target`. Also appended to image name. If not provided, no target is used.
    #[arg(long)]
    target: Option<String>,

    /// Use stale image if build fails.
    #[arg(long)]
    allow_stale: bool,

    /// Force rebuild regardless of staleness.
    #[arg(long)]
    force_rebuild: bool,

    /// Pass `--no-cache` to `docker build`.
    #[arg(long)]
    no_build_cache: bool,

    /// Skip rebuild; error if no image exists.
    #[arg(long)]
    no_rebuild: bool,

    /// Volume as "host:container", "/path" (same path), or "!/path" (remove). Repeatable.
    #[arg(long = "volume", short = 'v')]
    volumes: Vec<String>,

    /// Environment variable as "KEY=value", "KEY" (inherit), or "!KEY" (remove). Repeatable.
    #[arg(long = "env", short = 'e')]
    environment_variables: Vec<String>,

    /// Path to an executable to run before `docker run`. Its stdout is parsed as a TOML list of
    /// extra arguments to pass to the `docker run` command (e.g. `["--network", "host"]`).
    #[arg(long)]
    pre_run: Option<String>,

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
    /// Run the agent container.
    Run {
        /// Arguments to pass through to the container entrypoint. Must come after `--`.
        #[arg(last = true)]
        container_args: Vec<String>,
    },
}

/// Errors that can be returned from `get_config`.
#[derive(Debug, ThisError)]
pub(crate) enum ConfigError {
    /// The current working directory could not be determined.
    #[error("Failed to determine the current working directory: {0}")]
    CurrentWorkingDirectoryUnavailable(IoError),
    /// The current working directory is not valid UTF-8.
    #[error("The current working directory is not valid UTF-8")]
    CurrentWorkingDirectoryNotUtf8,
    /// The `dockerfile` path is empty.
    #[error("`dockerfile` must not be empty")]
    EmptyDockerfile,
    /// The `build_context` path is empty.
    #[error("`build_context` must not be empty")]
    EmptyBuildContext,
    /// A build argument CLI argument could not be parsed.
    #[error("Invalid build argument value {value:?}: expected \"KEY=value\" or \"!KEY\"")]
    InvalidBuildArgument {
        /// The raw value that failed parsing.
        value: String,
    },
    /// A build argument key is not a valid identifier.
    #[error(
        "Invalid build argument key {key:?}: must start with a letter or underscore and \
         contain only ASCII letters, digits, and underscores"
    )]
    InvalidBuildArgumentKey {
        /// The key that failed validation.
        key: String,
    },
    /// The `pre_build` path is empty.
    #[error("`pre_build` must not be empty")]
    EmptyPreBuild,
    /// The project name contains no alphanumeric characters and cannot produce a valid slug.
    #[error("Invalid `project_name` value {project_name:?}: contains no alphanumeric characters")]
    InvalidProjectName {
        /// The raw project name that failed slugification.
        project_name: String,
    },
    /// The username contains no alphanumeric characters and cannot produce a valid slug.
    #[error("Invalid `username` value {username:?}: contains no alphanumeric characters")]
    InvalidUsername {
        /// The raw username that failed slugification.
        username: String,
    },
    /// The `target` value is empty.
    #[error("`target` must not be empty; use \"!\" to suppress an inherited value")]
    EmptyTarget,
    /// The `target` value contains no alphanumeric characters and cannot be slugified.
    #[error("Invalid `target` value {target:?}: contains no alphanumeric characters")]
    InvalidTarget {
        /// The raw target value that failed slugification.
        target: String,
    },
    /// `force_rebuild` and `no_rebuild` were both set, which is contradictory.
    #[error("`force_rebuild` and `no_rebuild` are mutually exclusive")]
    ConflictingRebuildFlags,
    /// A volume value could not be parsed (bad CLI format).
    #[error(
        "Invalid volume value {value:?}: expected \"/host:/container\", \"/path\", or \
         \"!/container\""
    )]
    InvalidVolume {
        /// The raw value that failed parsing.
        value: String,
    },
    /// A volume container path is not absolute.
    #[error("Invalid volume path {path:?}: container paths must be absolute (start with \"/\")")]
    InvalidVolumePath {
        /// The container path that is not absolute.
        path: String,
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
    /// The `pre_run` path is empty.
    #[error("`pre_run` must not be empty")]
    EmptyPreRun,
    /// Figment failed to extract the configuration.
    #[error("Failed to load configuration: {0}")]
    Extract(Box<figment::Error>),
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
/// - `~/.agentcontainer/config.toml`
/// - `{ancestor}/.agentcontainer/config.toml` for each ancestor directory from `/` towards the
///   current working directory (excluding the CWD itself). Closer to `/` has lower priority.
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

    // Parse CLI `--build-arg` args into a map.
    let cli_build_args = parse_cli_build_arguments(&cli_args.build_arguments)?;

    // Parse CLI `--volume` args into a map.
    let cli_volumes = parse_cli_volumes(&cli_args.volumes)?;

    // Parse CLI `--env` args into a map.
    let cli_env_vars = parse_cli_environment_variables(&cli_args.environment_variables)?;

    debug!("Loading configuration from all sources");

    // Build the provider list in priority order (lowest to highest).
    let mut providers: Vec<Box<dyn figment::Provider>> = vec![
        Box::new(Toml::file(format!(
            "{home_dir}/.config/agentcontainer/config.toml"
        ))),
        Box::new(Toml::file(format!(
            "{home_dir}/.agentcontainer/config.toml"
        ))),
    ];

    // Get the current working directory once for ancestor config loading and path expansion.
    let cwd = env::current_dir().map_err(ConfigError::CurrentWorkingDirectoryUnavailable)?;

    // Ancestor directory configs: traverse from `/` towards the current working directory
    // (exclusive), each providing `.agentcontainer/config.toml`. Closer to `/` has lower priority.
    // When the home directory is an ancestor of current working directory it appears in this
    // traversal at higher priority than the explicit home entry above, which is the desired
    // behavior. Skip the current working directory itself (already covered by the relative
    // `.agentcontainer/config.toml` entry below) and reverse so that `/` comes first (lowest
    // priority).
    let mut ancestor_configs: Vec<_> = cwd
        .ancestors()
        .skip(1)
        .map(|ancestor| ancestor.join(".agentcontainer/config.toml"))
        .collect();
    ancestor_configs.reverse();
    for config_path in ancestor_configs {
        providers.push(Box::new(Toml::file(config_path)));
    }

    providers.push(Box::new(Toml::file(".agentcontainer/config.toml")));
    providers.push(Box::new(Toml::file(".agentcontainer/config.local.toml")));
    providers.push(Box::new(Env::prefixed("AGENTCONTAINER_")));

    // CLI dict args (build_arguments, volumes, and environment_variables) are combined into a
    // single provider so they travel together at the same priority level.
    {
        // We build a combined owned config struct to hold all maps so the provider satisfies the
        // `'static` lifetime bound required by `Box<dyn Provider>`.
        #[derive(Serialize)]
        struct CliDictArgs {
            build_arguments: HashMap<String, BuildArgumentEntry>,
            volumes: HashMap<String, VolumeEntry>,
            environment_variables: HashMap<String, EnvironmentVariableEntry>,
        }
        if !cli_build_args.is_empty() || !cli_volumes.is_empty() || !cli_env_vars.is_empty() {
            let cli_dict_args = CliDictArgs {
                build_arguments: cli_build_args,
                volumes: cli_volumes,
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
        build_context,
        pre_build,
        project_name,
        username,
        target,
        pre_run,
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
    debug!("Extracting merged configuration");
    let mut config: Config =
        Figment::from(MergingProvider::new(providers, String::from(home_dir))).extract()?;

    let cwd_str = cwd
        .to_str()
        .ok_or(ConfigError::CurrentWorkingDirectoryNotUtf8)?;
    clean_config(&mut config);
    validate_config(&config)?;
    expand_config_paths(&mut config, home_dir, cwd_str);

    trace!(?config, "Final resolved configuration");

    Ok((&cli_args.command, config))
}

/// Parse the list of `--build-arg` CLI arguments into a `HashMap<String, BuildArgumentEntry>`.
///
/// Accepted formats:
/// - `"KEY=value"` → `("KEY", Value("value"))` (split on the first `=`)
/// - `"KEY"` (no `=`) → `("KEY", Inherit)`
/// - `"!KEY"` → `("KEY", Remove)`
fn parse_cli_build_arguments(
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
fn parse_cli_volumes(raw_volumes: &[String]) -> Result<HashMap<String, VolumeEntry>, ConfigError> {
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

/// Strip removal sentinels from a fully-merged `Config`.
///
/// Entries marked `Remove` instruct higher-priority layers to suppress a key inherited from a
/// lower-priority layer. Once merging is complete they carry no further information and are
/// removed so that callers see only the final, actionable set of entries.
///
/// For `target`, `"!"` acts as a removal sentinel: a higher-priority layer can set it to `"!"`
/// to suppress a value inherited from a lower-priority layer.
fn clean_config(config: &mut Config) {
    config
        .build_arguments
        .retain(|_, entry| !matches!(entry, BuildArgumentEntry::Remove));

    if config.target.as_deref() == Some("!") {
        config.target = None;
    }

    config
        .volumes
        .retain(|_, entry| !matches!(entry, VolumeEntry::Remove));

    config
        .environment_variables
        .retain(|_, entry| !matches!(entry, EnvironmentVariableEntry::Remove));
}

/// Validate a fully-merged `Config`, returning the first error found.
fn validate_config(config: &Config) -> Result<(), ConfigError> {
    if config.dockerfile.is_empty() {
        return Err(ConfigError::EmptyDockerfile);
    }

    if config.build_context.is_empty() {
        return Err(ConfigError::EmptyBuildContext);
    }

    for key in config.build_arguments.keys() {
        if !is_valid_env_var_key(key) {
            return Err(ConfigError::InvalidBuildArgumentKey { key: key.clone() });
        }
    }

    if config.pre_build.as_deref() == Some("") {
        return Err(ConfigError::EmptyPreBuild);
    }

    if slugify(&config.project_name).is_empty() {
        return Err(ConfigError::InvalidProjectName {
            project_name: config.project_name.clone(),
        });
    }

    if slugify(&config.username).is_empty() {
        return Err(ConfigError::InvalidUsername {
            username: config.username.clone(),
        });
    }

    if config.target.as_deref() == Some("") {
        return Err(ConfigError::EmptyTarget);
    }

    if let Some(ref target) = config.target
        && slugify(target).is_empty()
    {
        return Err(ConfigError::InvalidTarget {
            target: target.clone(),
        });
    }

    if config.force_rebuild && config.no_rebuild {
        return Err(ConfigError::ConflictingRebuildFlags);
    }

    for container_path in config.volumes.keys() {
        if !container_path.starts_with('/') {
            return Err(ConfigError::InvalidVolumePath {
                path: container_path.clone(),
            });
        }
    }

    for key in config.environment_variables.keys() {
        if !is_valid_env_var_key(key) {
            return Err(ConfigError::InvalidEnvironmentVariableKey { key: key.clone() });
        }
    }

    if config.pre_run.as_deref() == Some("") {
        return Err(ConfigError::EmptyPreRun);
    }

    Ok(())
}

/// Expand leading `~` to `home_dir` and resolve relative paths to absolute using the current
/// working directory.
///
/// For `dockerfile`, `build_context`, `pre_build`, and `pre_run`: tildes are expanded, and relative
/// paths are resolved relative to the current working directory. These fields must not be empty;
/// `validate_config` is expected to run before this function.
///
/// For volume host paths: relative paths that look like filesystem paths (start with `.` or
/// contain `/`) are resolved relative to the current working directory. Paths that do not start
/// with `.` and do not contain `/` are treated as Docker volume names and left unchanged.
/// `SamePath` entries are converted to `Active` with the container path as the host path.
fn expand_config_paths(config: &mut Config, home_dir: &str, cwd: &str) {
    // Resolve `dockerfile` to an absolute path.
    if has_tilde_user_prefix(&config.dockerfile) {
        warn!(
            path = %config.dockerfile,
            "The `~user` syntax is not supported in `dockerfile`; \
             treating as a relative path."
        );
    }
    config.dockerfile = expand_and_resolve_path(&config.dockerfile, home_dir, cwd);

    // Resolve `build_context` to an absolute path.
    if has_tilde_user_prefix(&config.build_context) {
        warn!(
            path = %config.build_context,
            "The `~user` syntax is not supported in `build_context`; \
             treating as a relative path."
        );
    }
    config.build_context = expand_and_resolve_path(&config.build_context, home_dir, cwd);

    if let Some(ref mut pre_build_path) = config.pre_build {
        if has_tilde_user_prefix(pre_build_path) {
            warn!(
                path = %pre_build_path,
                "The `~user` syntax is not supported in `pre_build`; \
                 treating as a relative path."
            );
        }
        *pre_build_path = expand_and_resolve_path(pre_build_path, home_dir, cwd);
    }

    for (container_path, entry) in &mut config.volumes {
        match *entry {
            VolumeEntry::Active(ref mut host_path)
                if !host_path.starts_with('/') && is_relative_filesystem_path(host_path) =>
            {
                if has_tilde_user_prefix(host_path) {
                    warn!(
                        volume = %container_path,
                        path = %host_path,
                        "The `~user` syntax is not supported in volume host paths; \
                         treating as a relative path."
                    );
                }
                *host_path = expand_and_resolve_path(host_path, home_dir, cwd);
            }
            VolumeEntry::SamePath => {
                *entry = VolumeEntry::Active(container_path.clone());
            }
            _ => {}
        }
    }

    if let Some(ref mut pre_run_path) = config.pre_run {
        if has_tilde_user_prefix(pre_run_path) {
            warn!(
                path = %pre_run_path,
                "The `~user` syntax is not supported in `pre_run`; \
                 treating as a relative path."
            );
        }
        *pre_run_path = expand_and_resolve_path(pre_run_path, home_dir, cwd);
    }
}

#[cfg(test)]
mod tests;
