use super::build_docker_run_args;
use crate::config::{Config, EnvironmentVariableEntry, VolumeEntry};
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
        volumes: HashMap::new(),
        environment_variables: HashMap::new(),
        pre_run: None,
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
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            false,
            &[],
        );

        assert_eq!(
            &args[..3],
            &["run", "--init", "--rm"],
            "Fixed flags must appear at the start in this exact order: {args:?}"
        );
    }

    #[test]
    fn includes_tty_flags_when_stdin_is_terminal() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            args.contains(&String::from("-t")),
            "`-t` should be present when stdin is a TTY: {args:?}"
        );
        assert!(
            args.contains(&String::from("-i")),
            "`-i` should be present when stdin is a TTY: {args:?}"
        );
    }

    #[test]
    fn omits_tty_flags_when_stdin_is_not_terminal() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            false,
            &[],
        );

        assert!(
            !args.contains(&String::from("-t")),
            "`-t` should not be present when stdin is not a TTY: {args:?}"
        );
        assert!(
            !args.contains(&String::from("-i")),
            "`-i` should not be present when stdin is not a TTY: {args:?}"
        );
    }

    #[test]
    fn includes_user_mapping() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1001,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

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
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "--name", "agentcontainer_myproject_42"),
            "`--name agentcontainer_myproject_42` not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_working_directory() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "-w", "/home/user/project"),
            "`-w /home/user/project` not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_current_dir_volume() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "-v", "/home/user/project:/home/user/project"),
            "Current directory volume not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_worktree_volume_when_present() {
        let config = make_config();
        let worktree = PathBuf::from("/home/user/main-repo");
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            Some(&worktree),
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "-v", "/home/user/main-repo:/home/user/main-repo"),
            "Worktree volume not found in args: {args:?}"
        );
    }

    #[test]
    fn no_worktree_volume_when_absent() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        // Only one -v flag (for the current dir).
        let volume_count = args.iter().filter(|arg| *arg == "-v").count();
        assert_eq!(
            volume_count, 1,
            "Expected exactly one `-v` flag, got {volume_count}"
        );
    }

    #[test]
    fn includes_config_volumes() {
        let mut config = make_config();
        config.volumes.insert(
            String::from("/container/path"),
            VolumeEntry::Active(String::from("/host/path")),
        );
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "-v", "/host/path:/container/path"),
            "Config volume not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_same_path_volume() {
        let mut config = make_config();
        config
            .volumes
            .insert(String::from("/shared/data"), VolumeEntry::SamePath);
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "-v", "/shared/data:/shared/data"),
            "Same-path volume not found in args: {args:?}"
        );
    }

    #[test]
    fn includes_env_var_with_value() {
        let mut config = make_config();
        config.environment_variables.insert(
            String::from("MY_VAR"),
            EnvironmentVariableEntry::Value(String::from("my_value")),
        );
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

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
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        assert!(
            has_flag_pair(&args, "-e", "INHERITED_VAR"),
            "Inherited environment variable not found in args: {args:?}"
        );
    }

    #[test]
    fn image_name_is_last_when_no_container_args() {
        let config = make_config();
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &[],
        );

        let expected_image = config.get_image_name();
        assert_eq!(
            args.last().expect("Args should not be empty"),
            &expected_image,
            "Image name should be the last argument when there are no container args"
        );
    }

    #[test]
    fn pre_run_extra_args_appear_before_image_name() {
        let config = make_config();
        let pre_run_extra_args = vec![
            String::from("--volume"),
            String::from("/host/path:/container/path"),
        ];
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &[],
            true,
            &pre_run_extra_args,
        );

        let expected_image = config.get_image_name();
        let image_position = args
            .iter()
            .position(|arg| arg == &expected_image)
            .expect("Image name not found in args");

        // The two extra args should appear right before the image name.
        assert_eq!(
            args[image_position - 2],
            "--volume",
            "Pre-run extra args should appear before the image name: {args:?}"
        );
        assert_eq!(
            args[image_position - 1],
            "/host/path:/container/path",
            "Pre-run extra args should appear before the image name: {args:?}"
        );
    }

    #[test]
    fn pre_run_extra_args_and_container_args_coexist() {
        let config = make_config();
        let container_args = vec![String::from("bash")];
        let pre_run_extra_args = vec![String::from("--network"), String::from("host")];
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &container_args,
            true,
            &pre_run_extra_args,
        );

        let expected_image = config.get_image_name();
        let image_position = args
            .iter()
            .position(|arg| arg == &expected_image)
            .expect("Image name not found in args");

        // Pre-run args before image.
        assert_eq!(
            args[image_position - 2],
            "--network",
            "Pre-run extra args should appear before the image name: {args:?}"
        );
        assert_eq!(
            args[image_position - 1],
            "host",
            "Pre-run extra args should appear before the image name: {args:?}"
        );

        // Container args after image.
        assert_eq!(
            &args[image_position + 1..],
            &["bash"],
            "Container args should appear after the image name: {args:?}"
        );
    }

    #[test]
    fn container_args_appear_after_image_name() {
        let config = make_config();
        let container_args = vec![
            String::from("--print"),
            String::from("--output-format"),
            String::from("json"),
        ];
        let args = build_docker_run_args(
            &config,
            1000,
            1000,
            "/home/user/project",
            None,
            42,
            &container_args,
            true,
            &[],
        );

        let expected_image = config.get_image_name();
        let image_position = args
            .iter()
            .position(|arg| arg == &expected_image)
            .expect("Image name not found in args");

        let trailing_args = &args[image_position + 1..];
        assert_eq!(
            trailing_args,
            &["--print", "--output-format", "json"],
            "Container args should appear after the image name"
        );
    }
}
