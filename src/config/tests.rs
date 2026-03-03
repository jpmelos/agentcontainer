use super::{CliArgs, Command, ConfigError, get_config};
use std::{env, fs, path::Path};
use tempfile::tempdir;

impl CliArgs {
    /// Construct a `CliArgs` for use in tests, without going through CLI parsing.
    #[expect(
        clippy::too_many_arguments,
        reason = "Test constructor needs to accept all fields."
    )]
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "Test constructor mirrors the CLI flags exactly."
    )]
    fn new(
        command: Command,
        dockerfile: Option<String>,
        project_name: Option<String>,
        username: Option<String>,
        target: Option<String>,
        allow_stale: bool,
        force_rebuild: bool,
        no_build_cache: bool,
        no_rebuild: bool,
    ) -> Self {
        Self {
            dockerfile,
            project_name,
            username,
            target,
            allow_stale,
            force_rebuild,
            no_build_cache,
            no_rebuild,
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

/// Construct a default `CliArgs` for tests that don't care about CLI arguments.
fn default_cli_args(command: Command) -> CliArgs {
    CliArgs::new(command, None, None, None, None, false, false, false, false)
}

#[test]
fn no_configuration_sources_yields_default_configuration() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = default_cli_args(Command::Config);

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

mod configuration_sources_are_read {
    use super::*;

    #[test]
    fn xdg_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = CliArgs::new(
            Command::Config,
            Some(String::from("from-cli")),
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );

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
}

mod configuration_sources_priority_order {
    use super::*;

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = default_cli_args(Command::Config);

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
        let cli_args = CliArgs::new(
            Command::Config,
            Some(String::from("from-cli")),
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );

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
        let cli_args = CliArgs::new(
            Command::Config,
            Some(String::from("from-cli")),
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );

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
}

mod merging_cli_args {
    use super::*;

    #[test]
    fn cli_none_does_not_override_lower_sources() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = default_cli_args(Command::Config);

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
    fn bool_cli_false_does_not_override_lower_sources() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            "allow_stale = true",
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        // `allow_stale` is `false` here because the flag was not passed on the CLI; it must not
        // override the `true` set in the config file.
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(config.allow_stale);
    }
}

mod default_values {
    use super::*;

    #[test]
    fn default_dockerfile_is_agentcontainer_slash_dockerfile() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.dockerfile,
            String::from(".agentcontainer/Dockerfile")
        );
    }

    #[test]
    fn default_project_name_is_derived_from_cwd() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        // The temp directory name will be something like `tmp1a2b3c4d`. We just verify it's
        // non-empty and matches the last component of the CWD.
        let expected = cwd
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("Temporary directory has no valid file name.");
        assert_eq!(config.project_name, expected);
    }

    #[test]
    fn default_username_comes_from_whoami() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.username,
            whoami::username().unwrap_or_else(|_| String::from("unknown"))
        );
    }

    #[test]
    fn default_target_is_none() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(config.target.is_none());
    }

    #[test]
    fn default_allow_stale_is_false() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(!config.allow_stale);
    }

    #[test]
    fn default_force_rebuild_is_false() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(!config.force_rebuild);
    }

    #[test]
    fn default_no_build_cache_is_false() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(!config.no_build_cache);
    }

    #[test]
    fn default_no_rebuild_is_false() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(!config.no_rebuild);
    }
}

#[test]
fn force_rebuild_and_no_rebuild_together_is_an_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Build,
        None,
        None,
        None,
        None,
        false,
        true, // force_rebuild
        false,
        true, // no_rebuild
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with conflicting rebuild flags.");

    assert!(
        matches!(error, ConfigError::ConflictingRebuildFlags),
        "Expected `ConfigError::ConflictingRebuildFlags`, got: {error:?}"
    );
}

mod image_name {
    use super::*;

    #[test]
    fn image_name_without_target() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("myproject")),
            Some(String::from("alice")),
            None,
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.image_name(), "agentcontainer-alice-myproject:latest");
    }

    #[test]
    fn image_name_with_target() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("myproject")),
            Some(String::from("alice")),
            Some(String::from("claude")),
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.image_name(),
            "agentcontainer-alice-myproject-claude:latest"
        );
    }

    #[test]
    fn image_name_slugifies_project_name() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("My Project")),
            Some(String::from("alice")),
            None,
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.image_name(),
            "agentcontainer-alice-my-project:latest"
        );
    }

    #[test]
    fn image_name_slugifies_username() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("myproject")),
            Some(String::from("Alice Smith")),
            None,
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.image_name(),
            "agentcontainer-alice-smith-myproject:latest"
        );
    }

    #[test]
    fn image_name_slugifies_target() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("myproject")),
            Some(String::from("alice")),
            Some(String::from("My Target")),
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.image_name(),
            "agentcontainer-alice-myproject-my-target:latest"
        );
    }

    #[test]
    fn image_name_uses_unknown_when_username_slug_is_empty() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("myproject")),
            Some(String::from("@@@")),
            None,
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(
            config.image_name(),
            "agentcontainer-unknown-myproject:latest"
        );
    }

    #[test]
    fn image_name_uses_unknown_when_project_name_slug_is_empty() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            Some(String::from("@@@")),
            Some(String::from("alice")),
            None,
            false,
            false,
            false,
            false,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert_eq!(config.image_name(), "agentcontainer-alice-unknown:latest");
    }

    #[test]
    fn image_name_uses_unknown_when_target_slug_is_empty() {
        // `get_config` rejects targets with no alphanumeric characters, so we construct `Config`
        // directly to test the `image_name` fallback for an empty target slug.
        use super::super::Config;
        let config = Config {
            dockerfile: String::from(".agentcontainer/Dockerfile"),
            project_name: String::from("myproject"),
            username: String::from("alice"),
            target: Some(String::from("@@@")),
            allow_stale: false,
            force_rebuild: false,
            no_build_cache: false,
            no_rebuild: false,
        };

        assert_eq!(
            config.image_name(),
            "agentcontainer-alice-myproject-unknown:latest"
        );
    }
}

#[test]
fn invalid_target_with_no_alphanumeric_chars_is_an_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        Some(String::from("myproject")),
        Some(String::from("alice")),
        Some(String::from("@@@")),
        false,
        false,
        false,
        false,
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with an invalid target.");

    assert!(
        matches!(error, ConfigError::InvalidTarget { .. }),
        "Expected `ConfigError::InvalidTarget`, got: {error:?}"
    );
}
