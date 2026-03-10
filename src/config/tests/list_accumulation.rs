//! Tests that list-typed config fields (`pre_build`, `pre_run`, `post_run`) are concatenated
//! across multiple config sources rather than replaced.

mod pre_build {
    use super::super::*;

    #[test]
    fn two_toml_files_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = ["/usr/local/bin/a.sh"]"#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"pre_build = ["/usr/local/bin/b.sh"]"#,
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

        assert_eq!(
            config.pre_build,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }

    #[test]
    fn toml_and_env_var_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = ["/usr/local/bin/a.sh"]"#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_PRE_BUILD", r#"["/usr/local/bin/b.sh"]"#);
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

        assert_eq!(
            config.pre_build,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }

    #[test]
    fn toml_and_cli_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_build = ["/usr/local/bin/a.sh"]"#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .pre_build(&["/usr/local/bin/b.sh"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert_eq!(
            config.pre_build,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }
}

mod pre_run {
    use super::super::*;

    #[test]
    fn two_toml_files_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = ["/usr/local/bin/a.sh"]"#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"pre_run = ["/usr/local/bin/b.sh"]"#,
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

        assert_eq!(
            config.pre_run,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }

    #[test]
    fn toml_and_env_var_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = ["/usr/local/bin/a.sh"]"#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_PRE_RUN", r#"["/usr/local/bin/b.sh"]"#);
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

        assert_eq!(
            config.pre_run,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }

    #[test]
    fn toml_and_cli_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"pre_run = ["/usr/local/bin/a.sh"]"#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .pre_run(&["/usr/local/bin/b.sh"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert_eq!(
            config.pre_run,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }
}

mod post_run {
    use super::super::*;

    #[test]
    fn two_toml_files_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"post_run = ["/usr/local/bin/a.sh"]"#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"post_run = ["/usr/local/bin/b.sh"]"#,
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

        assert_eq!(
            config.post_run,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }

    #[test]
    fn toml_and_env_var_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"post_run = ["/usr/local/bin/a.sh"]"#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_POST_RUN", r#"["/usr/local/bin/b.sh"]"#);
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

        assert_eq!(
            config.post_run,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }

    #[test]
    fn toml_and_cli_are_concatenated() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"post_run = ["/usr/local/bin/a.sh"]"#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .post_run(&["/usr/local/bin/b.sh"])
            .build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert_eq!(
            config.post_run,
            vec!["/usr/local/bin/a.sh", "/usr/local/bin/b.sh"],
        );
    }
}
