use super::{Command, default_cli_args, get_config};
use std::env;
use tempfile::tempdir;

#[test]
fn default_dockerfile_is_resolved_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = default_cli_args(Command::Config);

    let cwd_str = cwd
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(
        config.dockerfile,
        format!("{cwd_str}/.agentcontainer/Dockerfile")
    );
}

#[test]
fn default_build_context_is_resolved_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = default_cli_args(Command::Config);

    let cwd_str = cwd
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(config.build_context, cwd_str);
}

#[test]
fn default_project_name_is_derived_from_cwd() {
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

    // The temp directory name will be something like `tmp1a2b3c4d`. We just verify it's
    // non-empty and matches the last component of the CWD.
    let expected = cwd
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("Temporary directory has no valid file name");
    assert_eq!(config.project_name, expected);
}

#[test]
fn default_username_comes_from_whoami() {
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

    assert_eq!(
        config.username,
        whoami::username().unwrap_or_else(|_| String::from("unknown"))
    );
}

#[test]
fn default_target_is_none() {
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

    assert!(config.target.is_none());
}

#[test]
fn default_allow_stale_is_false() {
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

    assert!(!config.allow_stale);
}

#[test]
fn default_force_rebuild_is_false() {
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

    assert!(!config.force_rebuild);
}

#[test]
fn default_no_build_cache_is_false() {
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

    assert!(!config.no_build_cache);
}

#[test]
fn default_no_rebuild_is_false() {
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

    assert!(!config.no_rebuild);
}
