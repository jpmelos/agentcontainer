use super::*;

#[test]
fn image_name_without_target() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .project_name("myproject")
        .username("alice")
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
        config.get_image_name(),
        "agentcontainer_alice_myproject:latest"
    );
}

#[test]
fn image_name_with_target() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .project_name("myproject")
        .username("alice")
        .target("claude")
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
        config.get_image_name(),
        "agentcontainer_alice_myproject_claude:latest"
    );
}

#[test]
fn image_name_slugifies_project_name() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .project_name("My Project")
        .username("alice")
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
        config.get_image_name(),
        "agentcontainer_alice_my_project:latest"
    );
}

#[test]
fn image_name_slugifies_username() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .project_name("myproject")
        .username("Alice Smith")
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
        config.get_image_name(),
        "agentcontainer_alice_smith_myproject:latest"
    );
}

#[test]
fn image_name_slugifies_target() {
    let home_dir = tempdir().expect("Failed to create temporary directory");
    let cli_args = CliArgsBuilder::new(Command::Config)
        .project_name("myproject")
        .username("alice")
        .target("My Target")
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
        config.get_image_name(),
        "agentcontainer_alice_myproject_my_target:latest"
    );
}
