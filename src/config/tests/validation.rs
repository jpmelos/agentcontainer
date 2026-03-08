use super::*;

mod dockerfile {
    use super::*;

    #[test]
    fn empty_dockerfile_in_config_file_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = """#,
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
        .expect_err("Expected `get_config` to fail with an empty dockerfile");

        assert!(
            matches!(error, ConfigError::EmptyDockerfile),
            "Expected `ConfigError::EmptyDockerfile`, got: {error:?}"
        );
    }

    #[test]
    fn empty_dockerfile_via_env_var_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "");
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty dockerfile");

        assert!(
            matches!(error, ConfigError::EmptyDockerfile),
            "Expected `ConfigError::EmptyDockerfile`, got: {error:?}"
        );
    }

    #[test]
    fn empty_dockerfile_via_cli_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config).dockerfile("").build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty dockerfile");

        assert!(
            matches!(error, ConfigError::EmptyDockerfile),
            "Expected `ConfigError::EmptyDockerfile`, got: {error:?}"
        );
    }
}

mod build_context {
    use super::*;

    #[test]
    fn empty_build_context_in_config_file_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"build_context = """#,
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
        .expect_err("Expected `get_config` to fail with an empty build context");

        assert!(
            matches!(error, ConfigError::EmptyBuildContext),
            "Expected `ConfigError::EmptyBuildContext`, got: {error:?}"
        );
    }

    #[test]
    fn empty_build_context_via_env_var_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_BUILD_CONTEXT", "");
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty build context");

        assert!(
            matches!(error, ConfigError::EmptyBuildContext),
            "Expected `ConfigError::EmptyBuildContext`, got: {error:?}"
        );
    }

    #[test]
    fn empty_build_context_via_cli_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .build_context("")
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty build context");

        assert!(
            matches!(error, ConfigError::EmptyBuildContext),
            "Expected `ConfigError::EmptyBuildContext`, got: {error:?}"
        );
    }
}

mod pre_build {
    use super::*;

    #[test]
    fn empty_pre_build_entry_in_config_file_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = [""]"#,
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
        .expect_err("Expected `get_config` to fail with an empty pre_build entry");

        assert!(
            matches!(error, ConfigError::EmptyPreBuild),
            "Expected `ConfigError::EmptyPreBuild`, got: {error:?}"
        );
    }

    #[test]
    fn string_pre_build_in_config_file_is_a_config_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = "not-a-list""#,
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
        .expect_err("Expected `get_config` to fail with a non-list pre_build");

        assert!(
            matches!(error, ConfigError::Extract(_)),
            "Expected `ConfigError::Extract`, got: {error:?}"
        );
    }

    #[test]
    fn empty_pre_build_entry_via_env_var_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_PRE_BUILD", r#"[""]"#);
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty pre_build entry");

        assert!(
            matches!(error, ConfigError::EmptyPreBuild),
            "Expected `ConfigError::EmptyPreBuild`, got: {error:?}"
        );
    }

    #[test]
    fn string_pre_build_via_env_var_is_a_config_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_PRE_BUILD", "not-a-list");
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a non-list pre_build");

        assert!(
            matches!(error, ConfigError::Extract(_)),
            "Expected `ConfigError::Extract`, got: {error:?}"
        );
    }

    #[test]
    fn empty_pre_build_via_cli_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .pre_build(&[""])
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty pre_build");

        assert!(
            matches!(error, ConfigError::EmptyPreBuild),
            "Expected `ConfigError::EmptyPreBuild`, got: {error:?}"
        );
    }
}

mod project_name {
    use super::*;

    #[test]
    fn project_name_not_slugifiable_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .project_name("@@@")
            .username("alice")
            .build();

        let result = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        );

        assert!(
            matches!(result, Err(ConfigError::InvalidProjectName { .. })),
            "Expected `InvalidProjectName` error, got: {result:?}"
        );
    }
}

mod username {
    use super::*;

    #[test]
    fn username_not_slugifiable_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .project_name("myproject")
            .username("@@@")
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid username");

        assert!(
            matches!(error, ConfigError::InvalidUsername { .. }),
            "Expected `ConfigError::InvalidUsername`, got: {error:?}"
        );
    }
}

mod target {
    use super::*;

    #[test]
    fn target_not_slugifiable_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .project_name("myproject")
            .username("alice")
            .target("@@@")
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid target");

        assert!(
            matches!(error, ConfigError::InvalidTarget { .. }),
            "Expected `ConfigError::InvalidTarget`, got: {error:?}"
        );
    }

    #[test]
    fn empty_target_in_config_file_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &home_dir.path().join(".agentcontainer/config.toml"),
            r#"target = "builder""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"target = """#,
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
        .expect_err("Expected `get_config` to fail with an empty target");

        assert!(
            matches!(error, ConfigError::EmptyTarget),
            "Expected `ConfigError::EmptyTarget`, got: {error:?}"
        );
    }

    #[test]
    fn empty_target_via_env_var_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"target = "builder""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_TARGET", "");
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty target");

        assert!(
            matches!(error, ConfigError::EmptyTarget),
            "Expected `ConfigError::EmptyTarget`, got: {error:?}"
        );
    }

    #[test]
    fn empty_target_via_cli_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"target = "builder""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config).target("").build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty target");

        assert!(
            matches!(error, ConfigError::EmptyTarget),
            "Expected `ConfigError::EmptyTarget`, got: {error:?}"
        );
    }
}

mod build_flags {
    use super::*;

    #[test]
    fn force_rebuild_and_no_rebuild_together_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Build)
            .force_rebuild()
            .no_rebuild()
            .build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with conflicting rebuild flags");

        assert!(
            matches!(error, ConfigError::ConflictingRebuildFlags),
            "Expected `ConfigError::ConflictingRebuildFlags`, got: {error:?}"
        );
    }
}

mod pre_run {
    use super::*;

    #[test]
    fn empty_pre_run_entry_in_config_file_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = [""]"#,
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
        .expect_err("Expected `get_config` to fail with an empty pre_run entry");

        assert!(
            matches!(error, ConfigError::EmptyPreRun),
            "Expected `ConfigError::EmptyPreRun`, got: {error:?}"
        );
    }

    #[test]
    fn string_pre_run_in_config_file_is_a_config_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = "not-a-list""#,
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
        .expect_err("Expected `get_config` to fail with a non-list pre_run");

        assert!(
            matches!(error, ConfigError::Extract(_)),
            "Expected `ConfigError::Extract`, got: {error:?}"
        );
    }

    #[test]
    fn empty_pre_run_entry_via_env_var_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_PRE_RUN", r#"[""]"#);
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty pre_run entry");

        assert!(
            matches!(error, ConfigError::EmptyPreRun),
            "Expected `ConfigError::EmptyPreRun`, got: {error:?}"
        );
    }

    #[test]
    fn string_pre_run_via_env_var_is_a_config_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_PRE_RUN", "not-a-list");
        };
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with a non-list pre_run");

        assert!(
            matches!(error, ConfigError::Extract(_)),
            "Expected `ConfigError::Extract`, got: {error:?}"
        );
    }

    #[test]
    fn empty_pre_run_via_cli_is_an_error() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config).pre_run(&[""]).build();

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an empty pre_run");

        assert!(
            matches!(error, ConfigError::EmptyPreRun),
            "Expected `ConfigError::EmptyPreRun`, got: {error:?}"
        );
    }
}
