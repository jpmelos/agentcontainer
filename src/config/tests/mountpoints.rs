use super::{
    CliArgs, Command, ConfigError, MountpointEntry, default_cli_args, get_config, write_file,
};
use std::env;
use tempfile::tempdir;

#[test]
fn default_mountpoints_is_empty() {
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

    assert!(config.mountpoints.is_empty());
}

#[test]
fn single_toml_file_with_mountpoints_is_read_correctly() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [mountpoints]
        "/container" = "/host"
        "/other" = false
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
        config.mountpoints.get("/container"),
        Some(MountpointEntry::Active(host)) if host == "/host"
    ));
    assert!(!config.mountpoints.contains_key("/other"));
}

#[test]
fn two_toml_files_with_different_container_paths_are_unioned() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
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
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
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
        config.mountpoints.get("/container"),
        Some(MountpointEntry::Active(host)) if host == "/host-from-cwd-local"
    ));
}

#[test]
fn cli_mountpoint_host_container_format_is_parsed_correctly() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
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
        config.mountpoints.get("/container"),
        Some(MountpointEntry::Active(host)) if host == "/host"
    ));
}

#[test]
fn cli_mountpoint_removal_format_sets_remove() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [mountpoints]
        "/container" = "/host-from-cwd"
        "#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
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
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect("`get_config` failed.");

    assert!(!config.mountpoints.contains_key("/container"));
}

#[test]
fn cli_mountpoint_overrides_toml_for_same_container_path() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [mountpoints]
        "/container" = "/host-from-toml"
        "#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory.");
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
        config.mountpoints.get("/container"),
        Some(MountpointEntry::Active(host)) if host == "/host-from-cli"
    ));
}

#[test]
fn true_in_toml_triggers_extract_error() {
    // `true` is not a valid mountpoint value; the custom deserializer rejects it, producing a
    // `ConfigError::Extract`.
    let home_dir = tempdir().expect("Failed to create temporary directory.");
    let cwd = tempdir().expect("Failed to create temporary directory.");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
        [mountpoints]
        "/container" = true
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
    .expect_err("Expected `get_config` to fail when `true` is used as a mountpoint value.");

    assert!(
        matches!(error, ConfigError::Extract(_)),
        "Expected `ConfigError::Extract`, got: {error:?}"
    );
}

#[test]
fn malformed_cli_mountpoint_no_colon_triggers_invalid_mountpoint_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
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
        vec![String::from("/no-colon-here")],
        vec![],
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with a malformed mountpoint.");

    assert!(
        matches!(error, ConfigError::InvalidMountpoint { .. }),
        "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
    );
}

#[test]
fn malformed_cli_mountpoint_empty_host_triggers_invalid_mountpoint_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
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
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with an empty host path.");

    assert!(
        matches!(error, ConfigError::InvalidMountpoint { .. }),
        "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
    );
}

#[test]
fn malformed_cli_mountpoint_empty_container_path_triggers_invalid_mountpoint_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
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
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with an empty container path.");

    assert!(
        matches!(error, ConfigError::InvalidMountpoint { .. }),
        "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
    );
}

#[test]
fn cli_mountpoint_removal_with_colon_in_container_path_triggers_invalid_mountpoint_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
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
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with a colon in the removal container path.");

    assert!(
        matches!(error, ConfigError::InvalidMountpoint { .. }),
        "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
    );
}

#[test]
fn cli_mountpoint_with_multiple_colons_triggers_invalid_mountpoint_error() {
    let home_dir = tempdir().expect("Failed to create temporary directory.");
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
    );

    let error = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8."),
        &cli_args,
    )
    .expect_err("Expected `get_config` to fail with multiple colons.");

    assert!(
        matches!(error, ConfigError::InvalidMountpoint { .. }),
        "Expected `ConfigError::InvalidMountpoint`, got: {error:?}"
    );
}
