use super::*;

#[test]
fn empty_when_no_config_files_exist() {
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

    assert!(config.files_read.is_empty());
}

#[test]
fn includes_xdg_config_file() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let xdg_path = home_dir.path().join(".config/agentcontainer/config.toml");
    write_file(&xdg_path, r#"dockerfile = "from-xdg""#);
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
        config.files_read,
        vec![xdg_path.to_string_lossy().into_owned()]
    );
}

#[test]
fn includes_home_dotfile() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let home_path = home_dir.path().join(".agentcontainer/config.toml");
    write_file(&home_path, r#"dockerfile = "from-home""#);
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
        config.files_read,
        vec![home_path.to_string_lossy().into_owned()]
    );
}

#[test]
fn includes_cwd_config_file() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    let cwd_config_path = cwd.path().join(".agentcontainer/config.toml");
    write_file(&cwd_config_path, r#"dockerfile = "from-cwd""#);
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
        config.files_read,
        vec![cwd_config_path.to_string_lossy().into_owned()]
    );
}

#[test]
fn includes_cwd_local_config_file() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    let cwd_local_path = cwd.path().join(".agentcontainer/config.local.toml");
    write_file(&cwd_local_path, r#"dockerfile = "from-local""#);
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
        config.files_read,
        vec![cwd_local_path.to_string_lossy().into_owned()]
    );
}

#[test]
fn includes_ancestor_config_file() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let base = tempdir().expect("Failed to create temporary directory");
    let cwd = base.path().join("child");
    fs::create_dir_all(&cwd).expect("Failed to create nested directory");
    let ancestor_path = base.path().join(".agentcontainer/config.toml");
    write_file(&ancestor_path, r#"dockerfile = "from-ancestor""#);
    env::set_current_dir(&cwd).expect("Failed to set current directory");
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
        config
            .files_read
            .contains(&ancestor_path.to_string_lossy().into_owned()),
        "Expected files_read to contain {ancestor_path:?}, got {:?}",
        config.files_read
    );
}

#[test]
fn lists_multiple_files_in_priority_order() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    let xdg_path = home_dir.path().join(".config/agentcontainer/config.toml");
    let home_path = home_dir.path().join(".agentcontainer/config.toml");
    let cwd_config_path = cwd.path().join(".agentcontainer/config.toml");
    let cwd_local_path = cwd.path().join(".agentcontainer/config.local.toml");
    write_file(&xdg_path, r#"project_name = "from-xdg""#);
    write_file(&home_path, r#"username = "from-home""#);
    write_file(&cwd_config_path, r#"dockerfile = "from-cwd""#);
    write_file(&cwd_local_path, r#"dockerfile = "from-local""#);
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

    let xdg_str = xdg_path.to_string_lossy().into_owned();
    let home_str = home_path.to_string_lossy().into_owned();
    let cwd_str = cwd_config_path.to_string_lossy().into_owned();
    let local_str = cwd_local_path.to_string_lossy().into_owned();

    // Verify all four files are present and in priority order.
    let xdg_pos = config
        .files_read
        .iter()
        .position(|p| p == &xdg_str)
        .expect("XDG config not found");
    let home_pos = config
        .files_read
        .iter()
        .position(|p| p == &home_str)
        .expect("Home config not found");
    let cwd_pos = config
        .files_read
        .iter()
        .position(|p| p == &cwd_str)
        .expect("CWD config not found");
    let local_pos = config
        .files_read
        .iter()
        .position(|p| p == &local_str)
        .expect("CWD local config not found");
    assert!(
        xdg_pos < home_pos && home_pos < cwd_pos && cwd_pos < local_pos,
        "Files should be in priority order (lowest to highest), got: {:?}",
        config.files_read
    );
}

#[test]
fn excludes_nonexistent_files() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    // Only create the CWD config; the XDG and home configs do not exist.
    let cwd_config_path = cwd.path().join(".agentcontainer/config.toml");
    write_file(&cwd_config_path, r#"dockerfile = "from-cwd""#);
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

    // Only the CWD config file should be listed.
    assert_eq!(
        config.files_read,
        vec![cwd_config_path.to_string_lossy().into_owned()]
    );
}
