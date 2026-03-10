mod build_arguments;
mod clean;
mod default_values;
mod environment_variables;
mod get_container_name;
mod get_image_name;
mod list_accumulation;
mod path_expansion;
mod validation;
mod volumes;

use super::{
    BuildArgumentEntry, CliArgs, Command, Config, ConfigError, EnvironmentVariableEntry,
    VolumeEntry, get_config,
};
use std::{collections::HashMap, env, fs, path::Path};
use tempfile::tempdir;

/// Builder for constructing `CliArgs` in tests without specifying all fields.
struct CliArgsBuilder {
    command: Command,
    dockerfile: Option<String>,
    build_context: Option<String>,
    build_arguments: Vec<String>,
    pre_build: Vec<String>,
    project_name: Option<String>,
    username: Option<String>,
    target: Option<String>,
    force_rebuild: bool,
    no_rebuild: bool,
    volumes: Vec<String>,
    environment_variables: Vec<String>,
    pre_run: Vec<String>,
    post_run: Vec<String>,
}

impl CliArgsBuilder {
    /// Start building a `CliArgs` for the given subcommand, with all other fields set to their
    /// defaults.
    fn new(command: Command) -> Self {
        Self {
            command,
            dockerfile: None,
            build_context: None,
            build_arguments: vec![],
            pre_build: vec![],
            project_name: None,
            username: None,
            target: None,
            force_rebuild: false,
            no_rebuild: false,
            volumes: vec![],
            environment_variables: vec![],
            pre_run: vec![],
            post_run: vec![],
        }
    }

    fn dockerfile(mut self, value: &str) -> Self {
        self.dockerfile = Some(String::from(value));
        self
    }

    fn build_context(mut self, value: &str) -> Self {
        self.build_context = Some(String::from(value));
        self
    }

    fn build_arguments(mut self, values: &[&str]) -> Self {
        self.build_arguments = values.iter().map(|s| String::from(*s)).collect();
        self
    }

    fn pre_build(mut self, values: &[&str]) -> Self {
        self.pre_build = values.iter().map(|s| String::from(*s)).collect();
        self
    }

    fn project_name(mut self, value: &str) -> Self {
        self.project_name = Some(String::from(value));
        self
    }

    fn username(mut self, value: &str) -> Self {
        self.username = Some(String::from(value));
        self
    }

    fn target(mut self, value: &str) -> Self {
        self.target = Some(String::from(value));
        self
    }

    fn force_rebuild(mut self) -> Self {
        self.force_rebuild = true;
        self
    }

    fn no_rebuild(mut self) -> Self {
        self.no_rebuild = true;
        self
    }

    fn volumes(mut self, values: &[&str]) -> Self {
        self.volumes = values.iter().map(|s| String::from(*s)).collect();
        self
    }

    fn environment_variables(mut self, values: &[&str]) -> Self {
        self.environment_variables = values.iter().map(|s| String::from(*s)).collect();
        self
    }

    fn pre_run(mut self, values: &[&str]) -> Self {
        self.pre_run = values.iter().map(|s| String::from(*s)).collect();
        self
    }

    fn post_run(mut self, values: &[&str]) -> Self {
        self.post_run = values.iter().map(|s| String::from(*s)).collect();
        self
    }

    /// Consume the builder and produce the `CliArgs`.
    fn build(self) -> CliArgs {
        CliArgs {
            command: self.command,
            dockerfile: self.dockerfile,
            build_context: self.build_context,
            build_arguments: self.build_arguments,
            pre_build: self.pre_build,
            project_name: self.project_name,
            username: self.username,
            target: self.target,
            allow_stale: false,
            force_rebuild: self.force_rebuild,
            no_build_cache: false,
            no_rebuild: self.no_rebuild,
            volumes: self.volumes,
            environment_variables: self.environment_variables,
            pre_run: self.pre_run,
            post_run: self.post_run,
        }
    }
}

// These tests use `std::env::set_current_dir` and `std::env::set_var`, which mutate
// process-global state. This is safe because `cargo nextest` runs each test in its own
// process.

/// Write content to a file, creating parent directories as needed.
fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent directories for test file");
    }
    fs::write(path, content).expect("Failed to write test file");
}

/// Construct a default `CliArgs` for tests that don't care about CLI arguments.
fn default_cli_args(command: Command) -> CliArgs {
    CliArgsBuilder::new(command).build()
}

/// Construct a `Config` for use in tests, without going through CLI parsing or `figment`.
fn make_config() -> Config {
    Config {
        dockerfile: String::from(".agentcontainer/Dockerfile"),
        build_context: String::from("."),
        build_arguments: HashMap::new(),
        pre_build: vec![],
        project_name: String::from("myproject"),
        username: String::from("alice"),
        target: None,
        allow_stale: false,
        force_rebuild: false,
        no_build_cache: false,
        no_rebuild: false,
        volumes: HashMap::new(),
        environment_variables: HashMap::new(),
        pre_run: vec![],
        post_run: vec![],
    }
}

#[test]
fn no_configuration_sources_yields_default_configuration() {
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

mod configuration_sources_are_read {
    use super::*;

    #[test]
    fn xdg_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-xdg"));
    }

    #[test]
    fn home_dotfile_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        write_file(
            &home_dir.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-home-dotfile"));
    }

    #[test]
    fn ancestor_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let base = tempdir().expect("Failed to create temporary directory");
        let cwd = base.path().join("child");
        fs::create_dir_all(&cwd).expect("Failed to create nested directory");
        // Write a config in the parent of CWD (an ancestor directory).
        write_file(
            &base.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-ancestor""#,
        );
        env::set_current_dir(&cwd).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let cwd_str = cwd
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-ancestor"));
    }

    #[test]
    fn cwd_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cwd"));
    }

    #[test]
    fn cwd_local_config_file_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cwd-local"));
    }

    #[test]
    fn env_var_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-env"));
    }

    #[test]
    fn cli_arg_is_read() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        let cli_args = CliArgsBuilder::new(Command::Config)
            .dockerfile("from-cli")
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cli"));
    }
}

mod configuration_sources_priority_order {
    use super::*;

    #[test]
    fn home_dotfile_overrides_xdg_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
        write_file(
            &home_dir.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-home-dotfile"));
    }

    #[test]
    fn ancestor_config_overrides_home_dotfile() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let base = tempdir().expect("Failed to create temporary directory");
        let cwd = base.path().join("child");
        fs::create_dir_all(&cwd).expect("Failed to create nested directory");
        write_file(
            &home_dir.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
        // Write a config in the parent of CWD (an ancestor directory).
        write_file(
            &base.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-ancestor""#,
        );
        env::set_current_dir(&cwd).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let cwd_str = cwd
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-ancestor"));
    }

    #[test]
    fn closer_ancestor_overrides_farther_ancestor() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let base = tempdir().expect("Failed to create temporary directory");
        // Create a nested directory structure: base/child/grandchild (CWD).
        let child = base.path().join("child");
        let grandchild = child.join("grandchild");
        fs::create_dir_all(&grandchild).expect("Failed to create nested directories");
        // Write config in base (farther ancestor).
        write_file(
            &base.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-farther-ancestor""#,
        );
        // Write config in child (closer ancestor).
        write_file(
            &child.join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-closer-ancestor""#,
        );
        env::set_current_dir(&grandchild).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let cwd_str = grandchild
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-closer-ancestor"));
    }

    #[test]
    fn home_config_overrides_parent_of_home_when_home_is_ancestor_of_cwd() {
        let base = tempdir().expect("Failed to create temporary directory");
        let home_dir = base.path().join("home");
        let cwd = home_dir.join("project");
        fs::create_dir_all(&cwd).expect("Failed to create nested directories");
        // Write config in parent of home (farther ancestor).
        write_file(
            &base.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-parent-of-home""#,
        );
        // Write config in home directory. Home is an ancestor of CWD, so its config is
        // loaded both as the explicit home entry and via ancestor traversal. Either way, it
        // should override the parent-of-home config.
        write_file(
            &home_dir.join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-home""#,
        );
        env::set_current_dir(&cwd).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let cwd_str = cwd
            .to_str()
            .expect("Temporary directory path is not valid UTF-8");
        let (_, config) = get_config(
            home_dir
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-home"));
    }

    #[test]
    fn cwd_config_overrides_ancestor_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let base = tempdir().expect("Failed to create temporary directory");
        let cwd = base.path().join("child");
        fs::create_dir_all(&cwd).expect("Failed to create nested directory");
        // Write config in parent of CWD (ancestor).
        write_file(
            &base.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-ancestor""#,
        );
        // Write config in CWD.
        write_file(
            &cwd.join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        env::set_current_dir(&cwd).expect("Failed to set current directory");
        let cli_args = default_cli_args(Command::Config);

        let cwd_str = cwd
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cwd"));
    }

    #[test]
    fn cwd_local_config_overrides_cwd_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cwd-local"));
    }

    #[test]
    fn env_var_overrides_cwd_local_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-env"));
    }

    #[test]
    fn cli_arg_overrides_env_var() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
        let cli_args = CliArgsBuilder::new(Command::Config)
            .dockerfile("from-cli")
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cli"));
    }

    #[test]
    fn full_priority_chain_cli_arg_wins() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );
        write_file(
            &home_dir.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // SAFETY:`set_var` is safe here because `cargo nextest` runs each test in its own process,
        // so there are no other threads to race with.
        unsafe {
            env::set_var("AGENTCONTAINER_DOCKERFILE", "from-env");
        };
        let cli_args = CliArgsBuilder::new(Command::Config)
            .dockerfile("from-cli")
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cli"));
    }
}

mod merging_cli_args {
    use super::*;

    #[test]
    fn cli_none_does_not_override_lower_sources() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
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

        assert_eq!(config.dockerfile, format!("{cwd_str}/from-cwd"));
    }

    #[test]
    fn bool_cli_false_does_not_override_lower_sources() {
        let home_dir = tempdir().expect("Failed to create temporary directory");
        let cwd = tempdir().expect("Failed to create temporary directory");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            "allow_stale = true",
        );
        env::set_current_dir(cwd.path()).expect("Failed to set current directory");
        // `allow_stale` is `false` here because the flag was not passed on the CLI; it must not
        // override the `true` set in the config file.
        let cli_args = default_cli_args(Command::Config);

        let (_, config) = get_config(
            home_dir
                .path()
                .to_str()
                .expect("Temporary directory path is not valid UTF-8"),
            &cli_args,
        )
        .expect("`get_config` failed");

        assert!(config.allow_stale);
    }
}
