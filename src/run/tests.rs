use super::build_docker_run_args;
use crate::config::{Config, EnvironmentVariableEntry, MountpointEntry};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Construct a `Config` for use in tests, without going through CLI parsing or `figment`.
fn make_config() -> Config {
    Config {
        dockerfile: String::from(".agentcontainer/Dockerfile"),
        build_context: String::from("."),
        project_name: String::from("myproject"),
        username: String::from("alice"),
        target: None,
        allow_stale: false,
        force_rebuild: false,
        no_build_cache: false,
        no_rebuild: false,
        mountpoints: HashMap::new(),
        environment_variables: HashMap::new(),
    }
}

/// Check if `args` contains a consecutive `[flag, value]` pair.
///
/// Uses `.windows(2)` which guarantees each window has exactly 2 elements, making the indexing
/// safe.
#[expect(
    clippy::missing_asserts_for_indexing,
    reason = "`.windows(2)` guarantees each window has exactly 2 elements."
)]
fn has_flag_pair(args: &[String], flag: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0] == flag && pair[1] == value)
}

// ---------------------------------------------------------------------------
// Tests for `build_docker_run_args()`.
// ---------------------------------------------------------------------------

mod build_docker_run_args {
    use super::*;

    #[test]
    fn includes_fixed_flags() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert_eq!(args[0], "run");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "-i");
        assert_eq!(args[3], "--init");
        assert_eq!(args[4], "--rm");
    }

    #[test]
    fn includes_user_mapping() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1001, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "--user", "1000:1001"),
            "`--user 1000:1001` not found in args: {args:?}"
        );
        assert!(
            has_flag_pair(&args, "--group-add", "0"),
            "`--group-add 0` not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_container_name() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "--name", "agentcontainer_myproject_42"),
            "`--name agentcontainer_myproject_42` not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_working_directory() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "-w", "/home/user/project"),
            "`-w /home/user/project` not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_current_dir_mount() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "-v", "/home/user/project:/home/user/project"),
            "Current directory mount not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_worktree_mount_when_present() {
        let config = make_config();
        let worktree = PathBuf::from("/home/user/main-repo");
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            Some(&worktree),
            42,
        );

        assert!(
            has_flag_pair(&args, "-v", "/home/user/main-repo:/home/user/main-repo"),
            "Worktree mount not found in args: {args:?}"
        );
    }

    #[test]
    fn no_worktree_mount_when_absent() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        // Only one -v flag (for the current dir).
        let volume_count = args.iter().filter(|arg| *arg == "-v").count();
        assert_eq!(
            volume_count, 1,
            "Expected exactly one `-v` flag, got {volume_count}."
        );
    }

    #[test]
    fn includes_config_mountpoints() {
        let mut config = make_config();
        config.mountpoints.insert(
            String::from("/container/path"),
            MountpointEntry::Active(String::from("/host/path")),
        );
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "-v", "/host/path:/container/path"),
            "Config mountpoint not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_env_var_with_value() {
        let mut config = make_config();
        config.environment_variables.insert(
            String::from("MY_VAR"),
            EnvironmentVariableEntry::Value(String::from("my_value")),
        );
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "-e", "MY_VAR=my_value"),
            "Environment variable with value not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_env_var_inherit() {
        let mut config = make_config();
        config.environment_variables.insert(
            String::from("INHERITED_VAR"),
            EnvironmentVariableEntry::Inherit,
        );
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        assert!(
            has_flag_pair(&args, "-e", "INHERITED_VAR"),
            "Inherited environment variable not found in args: {args:?}"
        );
    }

    #[test]
    fn image_name_is_last_argument() {
        let config = make_config();
        let args = build_docker_run_args(&config, 1000, 1000, "/home/user/project", None, 42);

        let expected_image = config.get_image_name();
        assert_eq!(
            args.last().expect("Args should not be empty."),
            &expected_image,
            "Image name should be the last argument."
        );
    }
}
