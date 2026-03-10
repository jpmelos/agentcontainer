use super::*;

#[test]
fn tilde_in_toml_pre_build_is_expanded_to_home_dir() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["~/hooks/build.sh"]"#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = default_cli_args(Command::Config);

    let home_dir_str = home_dir
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

    assert_eq!(
        config.pre_build,
        vec![format!("{home_dir_str}/hooks/build.sh")],
    );
}

#[test]
fn bare_tilde_in_toml_pre_build_is_expanded_to_home_dir() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["~"]"#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = default_cli_args(Command::Config);

    let home_dir_str = home_dir
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

    assert_eq!(config.pre_build, vec![home_dir_str]);
}

#[test]
fn tilde_in_cli_pre_build_is_expanded_to_home_dir() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .pre_build(&["~/hooks/build.sh"])
        .build();

    let home_dir_str = home_dir
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

    assert_eq!(
        config.pre_build,
        vec![format!("{home_dir_str}/hooks/build.sh")],
    );
}

#[test]
fn absolute_path_in_pre_build_is_unchanged() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["/usr/local/bin/hook.sh"]"#,
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

    assert_eq!(config.pre_build, vec!["/usr/local/bin/hook.sh"]);
}

#[test]
fn tilde_user_syntax_in_pre_build_is_not_tilde_expanded_but_resolved_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["~alice/hooks/build.sh"]"#,
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

    assert_eq!(
        config.pre_build,
        vec![format!("{cwd_str}/~alice/hooks/build.sh")],
    );
}

#[test]
fn relative_path_in_toml_pre_build_is_resolved_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["scripts/build.sh"]"#,
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

    assert_eq!(
        config.pre_build,
        vec![format!("{cwd_str}/scripts/build.sh")],
    );
}

#[test]
fn dot_slash_relative_path_in_toml_pre_build_is_resolved_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["./scripts/build.sh"]"#,
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

    assert_eq!(
        config.pre_build,
        vec![format!("{cwd_str}/scripts/build.sh")],
    );
}

#[test]
fn relative_path_in_cli_pre_build_is_resolved_to_cwd() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .pre_build(&["scripts/build.sh"])
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

    assert_eq!(
        config.pre_build,
        vec![format!("{cwd_str}/scripts/build.sh")],
    );
}

#[test]
fn tilde_in_env_var_pre_build_is_expanded_to_home_dir() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    // SAFETY: `set_var` is safe here because `cargo nextest` runs each test in its own
    // process, so there are no other threads to race with.
    unsafe {
        env::set_var("AGENTCONTAINER_PRE_BUILD", r#"["~/hooks/build.sh"]"#);
    };
    let cli_args = default_cli_args(Command::Config);

    let home_dir_str = home_dir
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

    assert_eq!(
        config.pre_build,
        vec![format!("{home_dir_str}/hooks/build.sh")],
    );
}

#[test]
fn toml_list_pre_build_entries_are_all_expanded() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cwd = tempdir().expect("Failed to create temporary directory");
    write_file(
        &cwd.path().join(".agentcontainer/config.toml"),
        r#"pre_build = ["~/hooks/a.sh", "scripts/b.sh"]"#,
    );
    env::set_current_dir(cwd.path()).expect("Failed to set current directory");
    let cli_args = default_cli_args(Command::Config);

    let home_dir_str = home_dir
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let cwd_str = cwd
        .path()
        .to_str()
        .expect("Temporary directory path is not valid UTF-8");
    let (_, config) = get_config(home_dir_str, &cli_args).expect("`get_config` failed");

    assert_eq!(
        config.pre_build,
        vec![
            format!("{home_dir_str}/hooks/a.sh"),
            format!("{cwd_str}/scripts/b.sh"),
        ],
    );
}
