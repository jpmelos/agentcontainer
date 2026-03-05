use super::{
    CliArgs, Command, ConfigError, EnvironmentVariableEntry, default_cli_args, get_config,
    write_file,
};
use std::env;
use tempfile::tempdir;

#[test]
fn default_environment_variables_is_empty() {
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

    assert!(config.environment_variables.is_empty());
}

#[test]
fn single_toml_file_with_env_vars_is_read_correctly() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [environment_variables]
        EDITOR = "nvim"
        API_KEY = true
        OLD_VAR = false
        "#,
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

    assert!(matches!(
        config.environment_variables.get("EDITOR"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "nvim"
    ));
    assert!(matches!(
        config.environment_variables.get("API_KEY"),
        Some(EnvironmentVariableEntry::Inherit)
    ));
    assert!(!config.environment_variables.contains_key("OLD_VAR"));
}

#[test]
fn two_toml_files_with_different_variable_names_are_unioned() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [environment_variables]
        VAR1 = "val1"
        "#,
    );
    write_file(
        &cwd.path().join(".agentcontainer/config.local.toml"),
        r#"
        [environment_variables]
        VAR2 = "val2"
        "#,
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

    assert!(matches!(
        config.environment_variables.get("VAR1"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "val1"
    ));
    assert!(matches!(
        config.environment_variables.get("VAR2"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "val2"
    ));
}

#[test]
fn two_toml_files_with_same_variable_name_later_wins() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [environment_variables]
        EDITOR = "vim"
        "#,
    );
    write_file(
        &cwd.path().join(".agentcontainer/config.local.toml"),
        r#"
        [environment_variables]
        EDITOR = "nvim"
        "#,
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

    assert!(matches!(
        config.environment_variables.get("EDITOR"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "nvim"
    ));
}

#[test]
fn cli_env_var_key_equals_value_format_parses_correctly() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::from("KEY=val")],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect("`get_config` failed.");

    assert!(matches!(
        config.environment_variables.get("KEY"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "val"
    ));
}

#[test]
fn cli_env_var_key_equals_value_with_equals_in_value_parses_correctly() {
    // Split is on the first `=` only; anything after is part of the value.
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::from("KEY=value=another")],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect("`get_config` failed.");

    assert!(matches!(
        config.environment_variables.get("KEY"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "value=another"
    ));
}

#[test]
fn cli_env_var_key_only_format_means_inherit() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::from("KEY")],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect("`get_config` failed.");

    assert!(matches!(
        config.environment_variables.get("KEY"),
        Some(EnvironmentVariableEntry::Inherit)
    ));
}

#[test]
fn cli_env_var_removal_format_sets_removed() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [environment_variables]
        EDITOR = "nvim"
        "#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::from("!EDITOR")],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect("`get_config` failed.");

    assert!(!config.environment_variables.contains_key("EDITOR"));
}

#[test]
fn cli_env_var_overrides_toml_for_same_variable_name() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [environment_variables]
        EDITOR = "vim"
        "#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::from("EDITOR=nvim")],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect("`get_config` failed.");

    assert!(matches!(
        config.environment_variables.get("EDITOR"),
        Some(EnvironmentVariableEntry::Value(value)) if value == "nvim"
    ));
}

#[test]
fn malformed_cli_env_var_empty_string_triggers_invalid_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::new()],
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with an empty environment variable argument.");

    assert!(
        matches!(error, ConfigError::InvalidEnvironmentVariable { .. }),
        "Expected `ConfigError::InvalidEnvironmentVariable`, got: {error:?}"
    );
}

#[test]
fn cli_env_var_removal_with_equals_in_key_triggers_invalid_key_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![String::from("!KEY=extra")],
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with an equals sign in the removal key.");

    assert!(
        matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
        "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
    );
}

mod invalid_environment_variable_keys {
    use super::*;

    #[test]
    fn cli_key_with_spaces_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("INVALID KEY=value")],
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn cli_key_starting_with_digit_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("1KEY=value")],
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn cli_key_with_hyphen_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("MY-KEY=value")],
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn cli_inherit_format_with_invalid_key_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("BAD KEY")],
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn cli_removal_format_with_invalid_key_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("!BAD KEY")],
        );

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn toml_key_with_invalid_characters_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [environment_variables]
            "BAD KEY" = "value"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn toml_key_starting_with_digit_is_rejected() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"
            [environment_variables]
            "9VAR" = "value"
            "#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
        let cli_args = default_cli_args(Command::Config);

        let error = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect_err("Expected `get_config` to fail with an invalid environment variable key.");

        assert!(
            matches!(error, ConfigError::InvalidEnvironmentVariableKey { .. }),
            "Expected `ConfigError::InvalidEnvironmentVariableKey`, got: {error:?}"
        );
    }

    #[test]
    fn valid_key_with_underscores_and_digits_is_accepted() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("_MY_VAR_2=value")],
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(matches!(
            config.environment_variables.get("_MY_VAR_2"),
            Some(EnvironmentVariableEntry::Value(value)) if value == "value"
        ));
    }

    #[test]
    fn valid_lowercase_key_is_accepted() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cli_args = CliArgs::new(
            Command::Config,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
            vec![],
            vec![String::from("my_var=value")],
        );

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8."),
            &cli_args,
        )
        .expect("`get_config` failed.");

        assert!(matches!(
            config.environment_variables.get("my_var"),
            Some(EnvironmentVariableEntry::Value(value)) if value == "value"
        ));
    }
}
