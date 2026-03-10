//! Process application configuration via configuration files, environment variables, and CLI
//! arguments.

mod cli;
mod entries;
mod error;
mod merging_provider;

use crate::utils::paths::{
    expand_and_resolve_path, has_tilde_user_prefix, is_relative_filesystem_path,
};
use crate::utils::slugify::slugify;
use cli::{
    is_valid_env_var_key, parse_cli_build_arguments, parse_cli_environment_variables,
    parse_cli_volumes,
};
use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use merging_provider::MergingProvider;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};
use tracing::{debug, trace, warn};

pub(crate) use cli::{CliArgs, Command};
pub(crate) use entries::{BuildArgumentEntry, EnvironmentVariableEntry, VolumeEntry};
pub(crate) use error::ConfigError;

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

    /// Paths to executables to run before `docker build`. Their stdout is parsed as a TOML list of
    /// extra arguments to pass to the `docker build` command. Lists from multiple config sources
    /// are concatenated (lower-priority first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pre_build: Vec<String>,

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

    /// Paths to executables to run before `docker run`. Their stdout is parsed as a TOML list of
    /// extra arguments to pass to the `docker run` command. Lists from multiple config sources
    /// are concatenated (lower-priority first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pre_run: Vec<String>,

    /// Paths to executables to run after `docker run`. Each hook receives the path to a file
    /// containing the stdout from the previous stage (or from `docker run` itself for the first
    /// hook) and prints the transformed output to its own stdout. When non-empty, `docker run`
    /// stdout is captured instead of inherited. Lists from multiple config sources are
    /// concatenated (lower-priority first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) post_run: Vec<String>,
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

    // CLI dict args (`build_arguments`, `volumes`, and `environment_variables`) are combined into
    // a single provider so they travel together at the same priority level.
    {
        #[derive(Serialize)]
        struct CliDictArgs {
            #[serde(skip_serializing_if = "HashMap::is_empty")]
            build_arguments: HashMap<String, BuildArgumentEntry>,
            #[serde(skip_serializing_if = "HashMap::is_empty")]
            volumes: HashMap<String, VolumeEntry>,
            #[serde(skip_serializing_if = "HashMap::is_empty")]
            environment_variables: HashMap<String, EnvironmentVariableEntry>,
        }
        let cli_dict_args = CliDictArgs {
            build_arguments: cli_build_args,
            volumes: cli_volumes,
            environment_variables: cli_env_vars,
        };
        providers.push(Box::new(Serialized::defaults(cli_dict_args)));
    }

    // CLI list args (`pre_build`, `pre_run`, `post_run`): empty fields are skipped during
    // serialization so that an absent flag does not override lists from lower-priority config
    // sources.
    {
        #[derive(Serialize)]
        struct CliListArgs {
            #[serde(skip_serializing_if = "Vec::is_empty")]
            pre_build: Vec<String>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            pre_run: Vec<String>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            post_run: Vec<String>,
        }
        let cli_list_args = CliListArgs {
            pre_build: cli_args.pre_build.clone(),
            pre_run: cli_args.pre_run.clone(),
            post_run: cli_args.post_run.clone(),
        };
        providers.push(Box::new(Serialized::defaults(cli_list_args)));
    }

    // CLI scalar and bool args, via the existing macros (only merged when actually provided/true).
    merge_string_cli_args!(
        cli_args,
        providers,
        dockerfile,
        build_context,
        project_name,
        username,
        target,
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

    if config.pre_build.iter().any(|path| path.is_empty()) {
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

    if config.pre_run.iter().any(|path| path.is_empty()) {
        return Err(ConfigError::EmptyPreRun);
    }

    if config.post_run.iter().any(|path| path.is_empty()) {
        return Err(ConfigError::EmptyPostRun);
    }

    Ok(())
}

/// Expand leading `~` to `home_dir` and resolve relative paths to absolute using the current
/// working directory.
///
/// For `dockerfile`, `build_context`, and each entry in `pre_build`, `pre_run`, and `post_run`:
/// tildes are expanded, and relative paths are resolved relative to the current working
/// directory. These fields must not be empty; `validate_config` is expected to run before this
/// function.
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

    for pre_build_path in &mut config.pre_build {
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

    for pre_run_path in &mut config.pre_run {
        if has_tilde_user_prefix(pre_run_path) {
            warn!(
                path = %pre_run_path,
                "The `~user` syntax is not supported in `pre_run`; \
                 treating as a relative path."
            );
        }
        *pre_run_path = expand_and_resolve_path(pre_run_path, home_dir, cwd);
    }

    for post_run_path in &mut config.post_run {
        if has_tilde_user_prefix(post_run_path) {
            warn!(
                path = %post_run_path,
                "The `~user` syntax is not supported in `post_run`; \
                 treating as a relative path."
            );
        }
        *post_run_path = expand_and_resolve_path(post_run_path, home_dir, cwd);
    }
}

#[cfg(test)]
mod tests;
