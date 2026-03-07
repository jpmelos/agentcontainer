use super::{
    CliArgs, Command, ConfigError, MountpointEntry, default_cli_args, get_config, write_file,
};
use std::env;
use tempfile::tempdir;

#[test]
fn default_mountpoints_is_empty() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = default_cli_args(Command::Config);

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert!(config.mountpoints.is_empty());
}

mod parsing_toml {
    use super::*;

    #[test]
    fn single_toml_file_with_mountpoints_is_read_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container" = "/host"
            "/other" = false
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.mountpoints.get("/container"),
            Some(MountpointEntry::Active(host)) if host == "/host"
        ));
        assert!(!config.mountpoints.contains_key("/other"));
    }

    #[test]
    fn two_toml_files_with_different_container_paths_are_unioned() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container1" = "/host1"
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [mountpoints]
            "/container2" = "/host2"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.mountpoints.get("/container1"),
            Some(MountpointEntry::Active(host)) if host == "/host1"
        ));
        assert!(matches!(
            config.mountpoints.get("/container2"),
            Some(MountpointEntry::Active(host)) if host == "/host2"
        ));
    }

    #[test]
    fn two_toml_files_with_same_container_path_later_wins() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container" = "/host-from-cwd"
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [mountpoints]
            "/container" = "/host-from-cwd-local"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.mountpoints.get("/container"),
            Some(MountpointEntry::Active(host)) if host == "/host-from-cwd-local"
        ));
    }

    #[test]
    fn toml_same_path_format_is_parsed_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container" = true
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(
            matches!(
                config.mountpoints.get("/container"),
                Some(MountpointEntry::SamePath)
            ),
            "Expected `SamePath`, got: {:?}",
            config.mountpoints.get("/container")
        );
    }

    #[test]
    fn toml_host_container_with_relative_host_path_is_accepted() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container" = "relative-host"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` should accept a relative host path");

        assert!(matches!(
            config.mountpoints.get("/container"),
            Some(MountpointEntry::Active(host)) if host == "relative-host"
        ));
    }
}

mod parsing_env_var {
    use super::*;

    #[test]
    fn env_var_same_path_mountpoint_is_parsed_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_MOUNTPOINTS", r#"{"/shared" = true}"#);
        };
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(
            matches!(
                config.mountpoints.get("/shared"),
                Some(MountpointEntry::SamePath)
            ),
            "Expected `SamePath`, got: {:?}",
            config.mountpoints.get("/shared")
        );
    }
}

mod parsing_cli_args {
    use super::*;

    #[test]
    fn cli_mountpoint_host_container_format_is_parsed_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/host:/container")],
            vec![],
            None,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.mountpoints.get("/container"),
            Some(MountpointEntry::Active(host)) if host == "/host"
        ));
    }

    #[test]
    fn cli_mountpoint_same_path_format_is_parsed_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/same-path")],
            vec![],
            None,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(
            matches!(
                config.mountpoints.get("/same-path"),
                Some(MountpointEntry::SamePath)
            ),
            "Expected `SamePath`, got: {:?}",
            config.mountpoints.get("/same-path")
        );
    }

    #[test]
    fn cli_mountpoint_removal_format_sets_remove() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container" = "/host-from-cwd"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("!/container")],
            vec![],
            None,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(!config.mountpoints.contains_key("/container"));
    }

    #[test]
    fn cli_host_container_with_relative_host_path_is_accepted() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("relative-host:/container")],
            vec![],
            None,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` should accept a relative host path");

        assert!(matches!(
            config.mountpoints.get("/container"),
            Some(MountpointEntry::Active(host)) if host == "relative-host"
        ));
    }
}

mod priority {
    use super::*;

    #[test]
    fn cli_mountpoint_overrides_toml_for_same_container_path() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/container" = "/host-from-toml"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/host-from-cli:/container")],
            vec![],
            None,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.mountpoints.get("/container"),
            Some(MountpointEntry::Active(host)) if host == "/host-from-cli"
        ));
    }

    #[test]
    fn cli_same_path_overrides_toml_active_for_same_container_path() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/data" = "/host/data"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/data")],
            vec![],
            None,
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(
            matches!(
                config.mountpoints.get("/data"),
                Some(MountpointEntry::SamePath)
            ),
            "Expected `SamePath`, got: {:?}",
            config.mountpoints.get("/data")
        );
    }

    #[test]
    fn toml_same_path_can_be_overridden_by_higher_priority_toml_active() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "/data" = true
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [mountpoints]
            "/data" = "/other/host/path"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.mountpoints.get("/data"),
            Some(MountpointEntry::Active(host)) if host == "/other/host/path"
        ));
    }
}

mod validation {
    use super::*;

    #[test]
    fn cli_mountpoint_empty_string_triggers_invalid_mountpoint_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::new()],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty mountpoint argument");

        assert!(
            matches!(error, ConfigError::InvalidMountpoint { .. }),
            "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
        );
    }

    #[test]
    fn malformed_cli_mountpoint_empty_host_triggers_invalid_mountpoint_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from(":/container")],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty host path");

        assert!(
            matches!(error, ConfigError::InvalidMountpoint { .. }),
            "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
        );
    }

    #[test]
    fn malformed_cli_mountpoint_empty_container_path_triggers_invalid_mountpoint_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/host:")],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty container path");

        assert!(
            matches!(error, ConfigError::InvalidMountpoint { .. }),
            "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
        );
    }

    #[test]
    fn cli_mountpoint_removal_with_colon_in_container_path_triggers_invalid_mountpoint_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("!/container:extra")],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a colon in the removal container path");

        assert!(
            matches!(error, ConfigError::InvalidMountpoint { .. }),
            "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
        );
    }

    #[test]
    fn cli_mountpoint_with_multiple_colons_triggers_invalid_mountpoint_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/host:/extra:/container")],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with multiple colons");

        assert!(
            matches!(error, ConfigError::InvalidMountpoint { .. }),
            "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
        );
    }

    #[test]
    fn cli_same_path_with_relative_path_triggers_invalid_mountpoint_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("relative-path")],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a relative same-path mountpoint");

        assert!(
            matches!(error, ConfigError::InvalidMountpointPath { .. }),
            "Expected `ConfigError::InvalidMountpointPath`, got: {error:?}"
        );
    }

    #[test]
    fn cli_host_container_with_relative_container_path_triggers_invalid_mountpoint_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![String::from("/host:relative")],
            vec![],
            None,
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a relative container path");

        assert!(
            matches!(error, ConfigError::InvalidMountpointPath { .. }),
            "Expected `ConfigError::InvalidMountpointPath`, got: {error:?}"
        );
    }

    #[test]
    fn toml_mountpoint_with_relative_container_path_triggers_invalid_mountpoint_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "relative" = "/host"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a relative container path in TOML");

        assert!(
            matches!(error, ConfigError::InvalidMountpointPath { .. }),
            "Expected `ConfigError::InvalidMountpointPath`, got: {error:?}"
        );
    }

    #[test]
    fn toml_same_path_with_relative_path_triggers_invalid_mountpoint_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [mountpoints]
            "relative" = true
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a relative same-path mountpoint in TOML");

        assert!(
            matches!(error, ConfigError::InvalidMountpointPath { .. }),
            "Expected `ConfigError::InvalidMountpointPath`, got: {error:?}"
        );
    }
}
