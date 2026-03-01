//! Process application configuration via configuration files, environment variables, and CLI
//! arguments.

use anyhow::Result;
use clap::{Parser, Subcommand};
use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default path to the Dockerfile.
fn default_dockerfile() -> String {
    String::from(".agentcontainer/Dockerfile")
}

/// Application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
    /// Path to the Dockerfile.
    #[serde(default = "default_dockerfile")]
    dockerfile: String,
}

/// CLI arguments.
#[derive(Parser, Debug)]
#[command(about, version)]
pub(crate) struct CliArgs {
    /// Path to the Dockerfile.
    #[arg(long, global = true)]
    dockerfile: Option<String>,

    #[command(subcommand)]
    command: Command,
}

/// Subcommands.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum Command {
    /// Print the resolved configuration.
    Config,
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
) -> Result<(&'cli_args Command, Config)> {
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
    // Must list all fields from `CliArgs` that we want to merge into configuration.
    merge_cli_args!(cli_args, figment, dockerfile);

    // Extract the configuration.
    let config: Config = figment.extract()?;

    Ok((&cli_args.command, config))
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, Command, get_config};
    use std::{env, fs, path::Path};
    use tempfile::tempdir;

    impl CliArgs {
        /// Construct a `CliArgs` for use in tests, without going through CLI parsing.
        fn new(command: Command, dockerfile: Option<String>) -> Self {
            Self {
                dockerfile,
                command,
            }
        }
    }

    // These tests use `std::env::set_current_dir` and `std::env::set_var`, which mutate
    // process-global state. This is safe because `cargo nextest` runs each test in its own
    // process.

    /// Write content to a file, creating parent directories as needed.
    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories for test file.");
        }
        fs::write(path, content).expect("Failed to write test file.");
    }

    #[test]
    fn no_configuration_sources_yields_default_configuration() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, ".agentcontainer/Dockerfile");
    }

    #[test]
    fn xdg_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-xdg");
    }

    #[test]
    fn home_dotfile_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".agentcontainer.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-home-dotfile");
    }

    #[test]
    fn cwd_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cwd");
    }

    #[test]
    fn cwd_local_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cwd-local");
    }

    #[test]
    fn env_var_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-env");
    }

    #[test]
    fn cli_arg_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(Command::Config, Some(String::from("from-cli")));

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cli");
    }

    #[test]
    fn home_dotfile_overrides_xdg_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
        write_file(
            &home_dir.path().join(".agentcontainer.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-home-dotfile");
    }

    #[test]
    fn cwd_config_overrides_home_dotfile() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".agentcontainer.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cwd");
    }

    #[test]
    fn cwd_local_config_overrides_cwd_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cwd-local");
    }

    #[test]
    fn env_var_overrides_cwd_local_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-env");
    }

    #[test]
    fn cli_arg_overrides_env_var() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
        let cli_args = CliArgs::new(Command::Config, Some(String::from("from-cli")));

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cli");
    }

    #[test]
    fn full_priority_chain_cli_arg_wins() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
        write_file(
            &home_dir.path().join(".agentcontainer.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
        let cli_args = CliArgs::new(Command::Config, Some(String::from("from-cli")));

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cli");
    }

    #[test]
    fn cli_none_does_not_override_lower_sources() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = CliArgs::new(Command::Config, None);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.dockerfile, "from-cwd");
    }
}
