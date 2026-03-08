use super::{
    CliArgsBuilder, Command, ConfigError, VolumeEntry, default_cli_args, get_config, write_file,
};
use std::env;
use tempfile::tempdir;

#[test]
fn default_volumes_is_empty() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
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

    assert!(config.volumes.is_empty());
}

mod parsing_toml {
    use super::*;

    #[test]
    fn single_toml_file_with_volumes_is_read_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
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
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == "/host"
        ));
        assert!(!config.volumes.contains_key("/other"));
    }

    #[test]
    fn two_toml_files_with_different_container_paths_are_unioned() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/container1" = "/host1"
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [volumes]
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
            config.volumes.get("/container1"),
            Some(VolumeEntry::Active(host)) if host == "/host1"
        ));
        assert!(matches!(
            config.volumes.get("/container2"),
            Some(VolumeEntry::Active(host)) if host == "/host2"
        ));
    }

    #[test]
    fn two_toml_files_with_same_container_path_later_wins() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/container" = "/host-from-cwd"
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [volumes]
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
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == "/host-from-cwd-local"
        ));
    }

    #[test]
    fn toml_same_path_format_is_resolved_to_active() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
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
                config.volumes.get("/container"),
                Some(VolumeEntry::Active(host)) if host == "/container"
            ),
            "Expected `Active(\"/container\")`, got: {:?}",
            config.volumes.get("/container")
        );
    }

    #[test]
    fn toml_docker_volume_name_is_accepted_and_unchanged() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/container" = "my-volume"
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
        .expect("`get_config` should accept a Docker volume name");

        assert!(matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == "my-volume"
        ));
    }
}

mod parsing_env_var {
    use super::*;

    #[test]
    fn env_var_same_path_volume_is_resolved_to_active() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_VOLUMES", r#"{"/shared" = true}"#);
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
                config.volumes.get("/shared"),
                Some(VolumeEntry::Active(host)) if host == "/shared"
            ),
            "Expected `Active(\"/shared\")`, got: {:?}",
            config.volumes.get("/shared")
        );
    }
}

mod parsing_cli_args {
    use super::*;

    #[test]
    fn cli_volume_host_container_format_is_parsed_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/host:/container"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == "/host"
        ));
    }

    #[test]
    fn cli_volume_same_path_format_is_resolved_to_active() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/same-path"])
            .build();

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
                config.volumes.get("/same-path"),
                Some(VolumeEntry::Active(host)) if host == "/same-path"
            ),
            "Expected `Active(\"/same-path\")`, got: {:?}",
            config.volumes.get("/same-path")
        );
    }

    #[test]
    fn cli_volume_removal_format_sets_remove() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/container" = "/host-from-cwd"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["!/container"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(!config.volumes.contains_key("/container"));
    }

    #[test]
    fn cli_docker_volume_name_is_accepted_and_unchanged() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["my-volume:/container"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` should accept a Docker volume name");

        assert!(matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == "my-volume"
        ));
    }
}

mod priority {
    use super::*;

    #[test]
    fn cli_volume_overrides_toml_for_same_container_path() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/container" = "/host-from-toml"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/host-from-cli:/container"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == "/host-from-cli"
        ));
    }

    #[test]
    fn cli_same_path_overrides_toml_active_for_same_container_path() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/data" = "/host/data"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/data"])
            .build();

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
                config.volumes.get("/data"),
                Some(VolumeEntry::Active(host)) if host == "/data"
            ),
            "Expected `Active(\"/data\")`, got: {:?}",
            config.volumes.get("/data")
        );
    }

    #[test]
    fn toml_same_path_can_be_overridden_by_higher_priority_toml_active() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
            "/data" = true
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [volumes]
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
            config.volumes.get("/data"),
            Some(VolumeEntry::Active(host)) if host == "/other/host/path"
        ));
    }
}

mod validation {
    use super::*;

    #[test]
    fn cli_volume_empty_string_triggers_invalid_volume_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config).volumes(&[""]).build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty volume argument");

        assert!(
            matches!(error, ConfigError::InvalidVolume { .. }),
            "Expected `ConfigError::InvalidVolume`, got: {error:?}"
        );
    }

    #[test]
    fn malformed_cli_volume_empty_host_triggers_invalid_volume_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&[":/container"])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty host path");

        assert!(
            matches!(error, ConfigError::InvalidVolume { .. }),
            "Expected `ConfigError::InvalidVolume`, got: {error:?}"
        );
    }

    #[test]
    fn malformed_cli_volume_empty_container_path_triggers_invalid_volume_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/host:"])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty container path");

        assert!(
            matches!(error, ConfigError::InvalidVolume { .. }),
            "Expected `ConfigError::InvalidVolume`, got: {error:?}"
        );
    }

    #[test]
    fn cli_volume_removal_with_colon_in_container_path_triggers_invalid_volume_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["!/container:extra"])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a colon in the removal container path");

        assert!(
            matches!(error, ConfigError::InvalidVolume { .. }),
            "Expected `ConfigError::InvalidVolume`, got: {error:?}"
        );
    }

    #[test]
    fn cli_volume_with_multiple_colons_triggers_invalid_volume_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/host:/extra:/container"])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with multiple colons");

        assert!(
            matches!(error, ConfigError::InvalidVolume { .. }),
            "Expected `ConfigError::InvalidVolume`, got: {error:?}"
        );
    }

    #[test]
    fn cli_same_path_with_relative_path_triggers_invalid_volume_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["relative-path"])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a relative same-path volume");

        assert!(
            matches!(error, ConfigError::InvalidVolumePath { .. }),
            "Expected `ConfigError::InvalidVolumePath`, got: {error:?}"
        );
    }

    #[test]
    fn cli_host_container_with_relative_container_path_triggers_invalid_volume_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["/host:relative"])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a relative container path");

        assert!(
            matches!(error, ConfigError::InvalidVolumePath { .. }),
            "Expected `ConfigError::InvalidVolumePath`, got: {error:?}"
        );
    }

    #[test]
    fn toml_volume_with_relative_container_path_triggers_invalid_volume_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
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
            matches!(error, ConfigError::InvalidVolumePath { .. }),
            "Expected `ConfigError::InvalidVolumePath`, got: {error:?}"
        );
    }

    #[test]
    fn toml_same_path_with_relative_path_triggers_invalid_volume_path_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [volumes]
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
        .expect_err("Expected `get_config` to fail with a relative same-path volume in TOML");

        assert!(
            matches!(error, ConfigError::InvalidVolumePath { .. }),
            "Expected `ConfigError::InvalidVolumePath`, got: {error:?}"
        );
    }
}
