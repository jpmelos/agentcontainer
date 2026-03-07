use super::*;

#[test]
fn image_name_without_target() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        Some(String::from("myproject")),
        Some(String::from("alice")),
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(
        config.get_image_name(),
        "agentcontainer_alice_myproject:latest"
    );
}

#[test]
fn image_name_with_target() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        Some(String::from("myproject")),
        Some(String::from("alice")),
        Some(String::from("claude")),
        false,
        false,
        false,
        false,
        vec![],
        vec![],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(
        config.get_image_name(),
        "agentcontainer_alice_myproject_claude:latest"
    );
}

#[test]
fn image_name_slugifies_project_name() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        Some(String::from("My Project")),
        Some(String::from("alice")),
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(
        config.get_image_name(),
        "agentcontainer_alice_my_project:latest"
    );
}

#[test]
fn image_name_slugifies_username() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        Some(String::from("myproject")),
        Some(String::from("Alice Smith")),
        None,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(
        config.get_image_name(),
        "agentcontainer_alice_smith_myproject:latest"
    );
}

#[test]
fn image_name_slugifies_target() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgs::new(
        Command::Config,
        None,
        None,
        Some(String::from("myproject")),
        Some(String::from("alice")),
        Some(String::from("My Target")),
        false,
        false,
        false,
        false,
        vec![],
        vec![],
    );

    let (_, config) = get_config(
        home_dir
            .path()
            .to_str()
            .expect("Temporary directory path is not valid UTF-8"),
        &cli_args,
    )
    .expect("`get_config` failed");

    assert_eq!(
        config.get_image_name(),
        "agentcontainer_alice_myproject_my_target:latest"
    );
}
