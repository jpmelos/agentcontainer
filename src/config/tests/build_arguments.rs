use super::{
    BuildArgumentEntry, CliArgs, Command, ConfigError, default_cli_args, get_config, write_file,
};
use std::env;
use tempfile::tempdir;

#[test]
fn default_build_arguments_is_empty() {
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

    assert!(config.build_arguments.is_empty());
}

mod parsing_toml {
    use super::*;

    #[test]
    fn single_toml_file_with_build_args_is_read_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [build_arguments]
            MY_ARG = "hello"
            REMOVED = false
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
            config.build_arguments.get("MY_ARG"),
            Some(BuildArgumentEntry::Value(value)) if value == "hello"
        ));
        assert!(!config.build_arguments.contains_key("REMOVED"));
    }

    #[test]
    fn two_toml_files_with_different_arg_names_are_unioned() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [build_arguments]
            ARG1 = "val1"
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [build_arguments]
            ARG2 = "val2"
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
            config.build_arguments.get("ARG1"),
            Some(BuildArgumentEntry::Value(value)) if value == "val1"
        ));
        assert!(matches!(
            config.build_arguments.get("ARG2"),
            Some(BuildArgumentEntry::Value(value)) if value == "val2"
        ));
    }

    #[test]
    fn two_toml_files_with_same_arg_name_later_wins() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [build_arguments]
            MY_ARG = "old"
            "#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"
            [build_arguments]
            MY_ARG = "new"
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
            config.build_arguments.get("MY_ARG"),
            Some(BuildArgumentEntry::Value(value)) if value == "new"
        ));
    }

    #[test]
    fn toml_true_means_inherit() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            "
            [build_arguments]
            MY_ARG = true
            ",
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
                config.build_arguments.get("MY_ARG"),
                Some(BuildArgumentEntry::Inherit)
            ),
            "Expected `Inherit`, got: {:?}",
            config.build_arguments.get("MY_ARG")
        );
    }
}

mod parsing_env_var {
    use super::*;

    #[test]
    fn env_var_inherit_format_is_parsed_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(&cwd.path().join(".agentcontainer/config.toml"), "");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
        // process, so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_BUILD_ARGUMENTS", "{MY_ARG = true}");
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
                config.build_arguments.get("MY_ARG"),
                Some(BuildArgumentEntry::Inherit)
            ),
            "Expected `Inherit`, got: {:?}",
            config.build_arguments.get("MY_ARG")
        );
    }
}

mod parsing_cli_args {
    use super::*;

    #[test]
    fn cli_build_arg_key_equals_value_format_parses_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("MY_ARG=hello")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
            config.build_arguments.get("MY_ARG"),
            Some(BuildArgumentEntry::Value(value)) if value == "hello"
        ));
    }

    #[test]
    fn cli_build_arg_key_equals_value_with_equals_in_value_parses_correctly() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("MY_ARG=val=ue")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
            config.build_arguments.get("MY_ARG"),
            Some(BuildArgumentEntry::Value(value)) if value == "val=ue"
        ));
    }

    #[test]
    fn cli_build_arg_key_only_means_inherit() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("MY_ARG")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
                config.build_arguments.get("MY_ARG"),
                Some(BuildArgumentEntry::Inherit)
            ),
            "Expected `Inherit`, got: {:?}",
            config.build_arguments.get("MY_ARG")
        );
    }

    #[test]
    fn cli_build_arg_removal_format_sets_removed() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [build_arguments]
            MY_ARG = "hello"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("!MY_ARG")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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

        assert!(!config.build_arguments.contains_key("MY_ARG"));
    }

    #[test]
    fn cli_build_arg_empty_string_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::new()],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
        .expect_err("Expected `get_config` to fail with an empty build argument");

        assert!(
            matches!(error, ConfigError::InvalidBuildArgument { .. }),
            "Expected `ConfigError::InvalidBuildArgument`, got: {error:?}"
        );
    }
}

mod priority {
    use super::*;

    #[test]
    fn cli_build_arg_overrides_toml_for_same_key() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [build_arguments]
            MY_ARG = "from-toml"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("MY_ARG=from-cli")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
            config.build_arguments.get("MY_ARG"),
            Some(BuildArgumentEntry::Value(value)) if value == "from-cli"
        ));
    }
}

mod invalid_build_argument_keys {
    use super::*;

    #[test]
    fn cli_key_only_with_invalid_key_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("1INVALID")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
        .expect_err("Expected `get_config` to fail with an invalid build argument key");

        assert!(
            matches!(error, ConfigError::InvalidBuildArgumentKey { .. }),
            "Expected `ConfigError::InvalidBuildArgumentKey`, got: {error:?}"
        );
    }

    #[test]
    fn cli_key_with_spaces_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("BAD KEY=value")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
        .expect_err("Expected `get_config` to fail with an invalid build argument key");

        assert!(
            matches!(error, ConfigError::InvalidBuildArgumentKey { .. }),
            "Expected `ConfigError::InvalidBuildArgumentKey`, got: {error:?}"
        );
    }

    #[test]
    fn cli_key_starting_with_digit_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            vec![String::from("1KEY=value")],
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
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
        .expect_err("Expected `get_config` to fail with an invalid build argument key");

        assert!(
            matches!(error, ConfigError::InvalidBuildArgumentKey { .. }),
            "Expected `ConfigError::InvalidBuildArgumentKey`, got: {error:?}"
        );
    }

    #[test]
    fn toml_key_with_invalid_characters_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [build_arguments]
            "BAD KEY" = "value"
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
        .expect_err("Expected `get_config` to fail with an invalid build argument key");

        assert!(
            matches!(error, ConfigError::InvalidBuildArgumentKey { .. }),
            "Expected `ConfigError::InvalidBuildArgumentKey`, got: {error:?}"
        );
    }
}
