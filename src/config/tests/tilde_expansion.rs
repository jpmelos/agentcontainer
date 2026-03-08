use super::{CliArgsBuilder, Command, VolumeEntry, default_cli_args, get_config, write_file};
use std::env;
use tempfile::tempdir;

mod pre_build {
    use super::*;

    #[test]
    fn tilde_in_toml_pre_build_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = "~/hooks/build.sh""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert_eq!(
            config.pre_build.as_deref(),
            Some(format!("{home_dir_str}/hooks/build.sh").as_str()),
        );
    }

    #[test]
    fn bare_tilde_in_toml_pre_build_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = "~""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert_eq!(config.pre_build.as_deref(), Some(home_dir_str));
    }

    #[test]
    fn tilde_in_cli_pre_build_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .pre_build("~/hooks/build.sh")
            .build();

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert_eq!(
            config.pre_build.as_deref(),
            Some(format!("{home_dir_str}/hooks/build.sh").as_str()),
        );
    }

    #[test]
    fn absolute_path_in_pre_build_is_unchanged() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = "/usr/local/bin/hook.sh""#,
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

        assert_eq!(config.pre_build.as_deref(), Some("/usr/local/bin/hook.sh"),);
    }

    #[test]
    fn tilde_user_syntax_in_pre_build_is_not_expanded() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = "~alice/hooks/build.sh""#,
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

        assert_eq!(config.pre_build.as_deref(), Some("~alice/hooks/build.sh"),);
    }
}

mod volumes {
    use super::*;

    #[test]
    fn tilde_in_toml_host_path_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "/container" = "~/host-data"
        "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert!(matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == &format!("{home_dir_str}/host-data")
        ));
    }

    #[test]
    fn tilde_in_toml_container_path_key_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "~/.ssh" = "/host/.ssh"
        "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        let expected_container_path = format!("{home_dir_str}/.ssh");
        assert!(matches!(
            config.volumes.get(expected_container_path.as_str()),
            Some(VolumeEntry::Active(host)) if host == "/host/.ssh"
        ));
    }

    #[test]
    fn tilde_in_toml_same_path_key_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "~/.ssh" = true
        "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        let expected_path = format!("{home_dir_str}/.ssh");
        assert!(
            matches!(
                config.volumes.get(expected_path.as_str()),
                Some(VolumeEntry::SamePath)
            ),
            "Expected `SamePath` at {expected_path:?}, got: {:?}",
            config.volumes
        );
    }

    #[test]
    fn bare_tilde_in_host_path_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "/home" = "~"
        "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert!(matches!(
            config.volumes.get("/home"),
            Some(VolumeEntry::Active(host)) if host == home_dir_str
        ));
    }

    #[test]
    fn tilde_in_cli_host_path_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .volumes(&["~/projects:/container"])
            .build();

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert!(matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == &format!("{home_dir_str}/projects")
        ));
    }

    #[test]
    fn tilde_in_both_paths_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "~/.config" = "~/.config"
        "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        let expected_path = format!("{home_dir_str}/.config");
        assert!(matches!(
            config.volumes.get(expected_path.as_str()),
            Some(VolumeEntry::Active(host)) if host == &expected_path
        ));
    }

    #[test]
    fn embedded_tilde_is_not_expanded() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "/container" = "/host/~data"
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
            Some(VolumeEntry::Active(host)) if host == "/host/~data"
        ));
    }

    #[test]
    fn higher_priority_literal_path_overrides_lower_priority_tilde_path() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");

        // Lower-priority source uses `~/.ssh`.
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "~/.ssh" = "/low-priority-host"
        "#,
        );
        // Higher-priority source uses the literal expanded path.
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            &format!(
                r#"
            [volumes]
            "{home_dir_str}/.ssh" = "/high-priority-host"
            "#
            ),
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        let expected_key = format!("{home_dir_str}/.ssh");
        assert!(
            matches!(
                config.volumes.get(expected_key.as_str()),
                Some(VolumeEntry::Active(host)) if host == "/high-priority-host"
            ),
            "Expected higher-priority literal path to win, got: {:?}",
            config.volumes
        );
    }

    #[test]
    fn higher_priority_tilde_path_overrides_lower_priority_literal_path() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");

        // Lower-priority source uses the literal expanded path.
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            &format!(
                r#"
            [volumes]
            "{home_dir_str}/.ssh" = "/low-priority-host"
            "#
            ),
        );
        // Higher-priority source uses `~/.ssh`.
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
        [volumes]
        "~/.ssh" = "/high-priority-host"
        "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        let expected_key = format!("{home_dir_str}/.ssh");
        assert!(
            matches!(
                config.volumes.get(expected_key.as_str()),
                Some(VolumeEntry::Active(host)) if host == "/high-priority-host"
            ),
            "Expected higher-priority tilde path to win, got: {:?}",
            config.volumes
        );
    }

    #[test]
    fn tilde_user_syntax_is_not_expanded() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
        [volumes]
        "/container" = "~alice/data"
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
            Some(VolumeEntry::Active(host)) if host == "~alice/data"
        ));
    }
}

mod pre_run {
    use super::*;

    #[test]
    fn tilde_in_toml_pre_run_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = "~/hooks/run.sh""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert_eq!(
            config.pre_run.as_deref(),
            Some(format!("{home_dir_str}/hooks/run.sh").as_str()),
        );
    }

    #[test]
    fn bare_tilde_in_toml_pre_run_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = "~""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert_eq!(config.pre_run.as_deref(), Some(home_dir_str));
    }

    #[test]
    fn tilde_in_cli_pre_run_is_expanded_to_home_dir() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .pre_run("~/hooks/run.sh")
            .build();

        let home_dir_str = home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

        assert_eq!(
            config.pre_run.as_deref(),
            Some(format!("{home_dir_str}/hooks/run.sh").as_str()),
        );
    }

    #[test]
    fn absolute_path_in_pre_run_is_unchanged() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = "/usr/local/bin/hook.sh""#,
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

        assert_eq!(config.pre_run.as_deref(), Some("/usr/local/bin/hook.sh"));
    }

    #[test]
    fn tilde_user_syntax_in_pre_run_is_not_expanded() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = "~alice/hooks/run.sh""#,
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

        assert_eq!(config.pre_run.as_deref(), Some("~alice/hooks/run.sh"),);
    }
}
