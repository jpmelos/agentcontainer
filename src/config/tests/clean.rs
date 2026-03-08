use super::*;

mod target {
    use super::*;

    #[test]
    fn bang_target_in_config_file_removes_inherited_target() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &home_dir.path().join(".agentcontainer/config.toml"),
            r#"target = "builder""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"target = "!""#,
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
            config.target.is_none(),
            "Expected `target` to be `None`, got: {:?}",
            config.target
        );
    }

    #[test]
    fn bang_target_via_env_var_removes_inherited_target() {
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
            env::set_var("AGENTCONTAINER_TARGET", "!");
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
            config.target.is_none(),
            "Expected `target` to be `None`, got: {:?}",
            config.target
        );
    }

    #[test]
    fn bang_target_via_cli_removes_inherited_target() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"target = "builder""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config).target("!").build();

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(
            config.target.is_none(),
            "Expected `target` to be `None`, got: {:?}",
            config.target
        );
    }
}
