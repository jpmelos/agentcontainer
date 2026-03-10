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
fn tilde_in_toml_same_path_key_is_expanded_to_home_dir_and_resolved_to_active() {
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
            Some(VolumeEntry::Active(host)) if host == &expected_path
        ),
        "Expected `Active({expected_path:?})`, got: {:?}",
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
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
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
fn tilde_user_syntax_is_not_tilde_expanded_but_resolved_to_cwd() {
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

    let expected_host = format!("{cwd_str}/~alice/data");
    assert!(
        matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == &expected_host
        ),
        "Expected host path {expected_host:?}, got: {:?}",
        config.volumes.get("/container")
    );
}

#[test]
fn relative_host_path_starting_with_dot_is_expanded_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
            [volumes]
            "/container" = "./data"
            "#,
    );
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

    let expected_host = format!("{cwd_str}/data");
    assert!(
        matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == &expected_host
        ),
        "Expected host path {expected_host:?}, got: {:?}",
        config.volumes.get("/container")
    );
}

#[test]
fn relative_host_path_containing_slash_is_expanded_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
            [volumes]
            "/container" = "data/subdir"
            "#,
    );
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

    let expected_host = format!("{cwd_str}/data/subdir");
    assert!(
        matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == &expected_host
        ),
        "Expected host path {expected_host:?}, got: {:?}",
        config.volumes.get("/container")
    );
}

#[test]
fn docker_volume_name_is_not_expanded() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"
            [volumes]
            "/container" = "my_volume"
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
            Some(VolumeEntry::Active(host)) if host == "my_volume"
        ),
        "Expected Docker volume name to be unchanged, got: {:?}",
        config.volumes.get("/container")
    );
}

#[test]
fn relative_host_path_from_cli_starting_with_dot_is_expanded_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .volumes(&["./data:/container"])
        .build();

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

    let expected_host = format!("{cwd_str}/data");
    assert!(
        matches!(
            config.volumes.get("/container"),
            Some(VolumeEntry::Active(host)) if host == &expected_host
        ),
        "Expected host path {expected_host:?}, got: {:?}",
        config.volumes.get("/container")
    );
}
