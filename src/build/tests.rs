use super::{
    BuildError, BuildOutcome, DockerBuildError, assemble_docker_build_args, build,
    build_docker_build_hookable_args,
};
use crate::config::{BuildArgumentEntry, Config};
use crate::utils::clock::Clock;
use crate::utils::docker::DockerBackend;
use crate::utils::fs::Filesystem;
use anyhow::anyhow;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone as _, Utc};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Error as IoError;
use std::process::Output;

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

/// Return the image name produced by the default `make_config` config.
fn default_image_name() -> String {
    make_config().get_image_name()
}

/// A UTC timestamp that represents "today" at noon.
fn today_noon_utc() -> DateTime<Utc> {
    let today = Local::now().date_naive();
    let noon = NaiveTime::from_hms_opt(12, 0, 0).expect("Valid time");
    let naive = NaiveDateTime::new(today, noon);
    // Convert via local timezone so "today" means the same calendar day as the clock mock.
    Local
        .from_local_datetime(&naive)
        .single()
        .expect("Unambiguous local time")
        .with_timezone(&Utc)
}

/// A UTC timestamp that is one day before `today_noon_utc`.
fn yesterday_noon_utc() -> DateTime<Utc> {
    let yesterday = Local::now().date_naive().pred_opt().expect("Valid date");
    let noon = NaiveTime::from_hms_opt(12, 0, 0).expect("Valid time");
    let naive = NaiveDateTime::new(yesterday, noon);
    Local
        .from_local_datetime(&naive)
        .single()
        .expect("Unambiguous local time")
        .with_timezone(&Utc)
}

/// A UTC timestamp far in the past (a fixed date well before "today").
fn long_ago_utc() -> DateTime<Utc> {
    let date = NaiveDate::from_ymd_opt(2000, 1, 1).expect("Valid date");
    let noon = NaiveTime::from_hms_opt(12, 0, 0).expect("Valid time");
    Utc.from_utc_datetime(&NaiveDateTime::new(date, noon))
}

/// Produce a `DockerBuildError::NonZeroExit` with a dummy exit status obtained by running a
/// process that exits with a non-zero code.
fn make_non_zero_exit_error() -> DockerBuildError {
    use std::process::Command;
    let status = Command::new("sh")
        .args(["-c", "exit 1"])
        .status()
        .expect("Failed to run shell to produce a non-zero exit status");
    DockerBuildError::NonZeroExit(status)
}

/// Configurable mock for `DockerBackend`.
struct MockDocker {
    /// Value returned by `fetch_image_last_tag_timestamp`.
    existing_image_last_tagged: Result<Option<DateTime<Utc>>, anyhow::Error>,
    /// Value returned by `run_docker_build`.
    build_result: Result<(), DockerBuildError>,
    /// Captures the full args passed to `run_docker_build`.
    received_build_args: RefCell<Vec<String>>,
}

impl MockDocker {
    /// The image does not exist and `docker build` succeeds.
    fn no_image_build_succeeds() -> Self {
        Self {
            existing_image_last_tagged: Ok(None),
            build_result: Ok(()),
            received_build_args: RefCell::new(Vec::new()),
        }
    }

    /// The image does not exist and `docker build` fails with a non-zero exit status.
    fn no_image_build_fails() -> Self {
        Self {
            existing_image_last_tagged: Ok(None),
            build_result: Err(make_non_zero_exit_error()),
            received_build_args: RefCell::new(Vec::new()),
        }
    }

    /// The image exists (last tagged at the given time) and `docker build` succeeds.
    fn image_exists_build_succeeds(last_tagged: DateTime<Utc>) -> Self {
        Self {
            existing_image_last_tagged: Ok(Some(last_tagged)),
            build_result: Ok(()),
            received_build_args: RefCell::new(Vec::new()),
        }
    }

    /// The image exists (last tagged at the given time) and `docker build` fails.
    fn image_exists_build_fails(last_tagged: DateTime<Utc>) -> Self {
        Self {
            existing_image_last_tagged: Ok(Some(last_tagged)),
            build_result: Err(make_non_zero_exit_error()),
            received_build_args: RefCell::new(Vec::new()),
        }
    }

    /// `fetch_image_last_tag_timestamp` itself returns an error.
    fn fetch_image_last_tag_timestamp_fails() -> Self {
        Self {
            existing_image_last_tagged: Err(anyhow!("docker inspect failed")),
            build_result: Ok(()),
            received_build_args: RefCell::new(Vec::new()),
        }
    }
}

impl DockerBackend for MockDocker {
    fn fetch_image_last_tag_timestamp(
        &self,
        _image_name: &str,
    ) -> Result<Option<DateTime<Utc>>, anyhow::Error> {
        match self.existing_image_last_tagged.as_ref() {
            Ok(value) => Ok(*value),
            Err(error) => Err(anyhow!("{error}")),
        }
    }

    fn run_docker_build(&self, args: &[String]) -> Result<(), DockerBuildError> {
        *self.received_build_args.borrow_mut() = args.to_vec();
        match self.build_result.as_ref() {
            Ok(&()) => Ok(()),
            Err(&DockerBuildError::NonZeroExit(status)) => {
                Err(DockerBuildError::NonZeroExit(status))
            }
            Err(&DockerBuildError::SpawnFailed(_)) => {
                unreachable!("Tests do not produce SpawnFailed errors")
            }
        }
    }

    fn exec_docker_run(&self, _args: &[String]) -> Result<Infallible, IoError> {
        unreachable!("Build tests do not call `exec_docker_run`")
    }

    fn spawn_docker_run(&self, _args: &[String]) -> Result<Output, IoError> {
        unreachable!("Build tests do not call `spawn_docker_run`")
    }
}

/// Configurable mock for `Filesystem`.
struct MockFilesystem {
    /// Value returned by `file_mtime`.
    mtime: Result<DateTime<Utc>, anyhow::Error>,
}

impl MockFilesystem {
    /// `file_mtime` returns the given timestamp.
    fn with_mtime(mtime: DateTime<Utc>) -> Self {
        Self { mtime: Ok(mtime) }
    }

    /// `file_mtime` returns an error.
    fn failing() -> Self {
        Self {
            mtime: Err(anyhow!("Failed to read metadata")),
        }
    }
}

impl Filesystem for MockFilesystem {
    fn file_mtime(&self, _path: &str) -> Result<DateTime<Utc>, anyhow::Error> {
        match self.mtime.as_ref() {
            Ok(timestamp) => Ok(*timestamp),
            Err(error) => Err(anyhow!("{error}")),
        }
    }
}

/// Configurable mock for `Clock`.
struct MockClock {
    now: DateTime<Local>,
}

impl MockClock {
    /// The clock reads the real local "now". Suitable for tests that only care about "today".
    fn real_now() -> Self {
        Self { now: Local::now() }
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Local> {
        self.now
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

mod build_docker_build_hookable_args {
    use super::*;

    #[test]
    fn empty_when_no_build_arguments() {
        let config = make_config();
        let args = build_docker_build_hookable_args(&config);
        assert!(args.is_empty(), "Expected empty hookable args: {args:?}");
    }

    #[test]
    fn includes_build_arg_with_value() {
        let mut config = make_config();
        config.build_arguments.insert(
            String::from("MY_ARG"),
            BuildArgumentEntry::Value(String::from("my_value")),
        );
        let args = build_docker_build_hookable_args(&config);
        assert!(
            has_flag_pair(&args, "--build-arg", "MY_ARG=my_value"),
            "Expected `--build-arg MY_ARG=my_value` in hookable args: {args:?}"
        );
    }

    #[test]
    fn includes_build_arg_inherit() {
        let mut config = make_config();
        config
            .build_arguments
            .insert(String::from("INHERITED"), BuildArgumentEntry::Inherit);
        let args = build_docker_build_hookable_args(&config);
        assert!(
            has_flag_pair(&args, "--build-arg", "INHERITED"),
            "Expected `--build-arg INHERITED` in hookable args: {args:?}"
        );
    }
}

mod assemble_docker_build_args {
    use super::*;

    #[test]
    fn starts_with_build_subcommand() {
        let config = make_config();
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert_eq!(args[0], "build", "First arg should be `build`: {args:?}");
    }

    #[test]
    fn includes_file_flag() {
        let config = make_config();
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert!(
            has_flag_pair(&args, "--file", &config.dockerfile),
            "Expected `--file` (long form) in args: {args:?}"
        );
    }

    #[test]
    fn includes_tag_flag() {
        let config = make_config();
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert!(
            has_flag_pair(&args, "--tag", &image_name),
            "Expected `--tag` (long form) in args: {args:?}"
        );
    }

    #[test]
    fn includes_target_when_set() {
        let config = Config {
            target: Some(String::from("builder")),
            ..make_config()
        };
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert!(
            has_flag_pair(&args, "--target", "builder"),
            "Expected `--target builder` in args: {args:?}"
        );
    }

    #[test]
    fn includes_no_cache_when_set() {
        let config = Config {
            no_build_cache: true,
            ..make_config()
        };
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert!(
            args.contains(&String::from("--no-cache")),
            "Expected `--no-cache` in assembled args: {args:?}"
        );
    }

    #[test]
    fn omits_no_cache_when_not_set() {
        let config = make_config();
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert!(
            !args.contains(&String::from("--no-cache")),
            "`--no-cache` should not be present when not set: {args:?}"
        );
    }

    #[test]
    fn build_context_is_last() {
        let config = make_config();
        let image_name = config.get_image_name();
        let args = assemble_docker_build_args(&config, &image_name, &[]);
        assert_eq!(
            args.last().expect("Args should not be empty"),
            &config.build_context,
            "Build context should be the last argument: {args:?}"
        );
    }

    #[test]
    fn hookable_args_appear_before_build_context() {
        let config = make_config();
        let image_name = config.get_image_name();
        let hookable = vec![String::from("--label"), String::from("foo=bar")];
        let args = assemble_docker_build_args(&config, &image_name, &hookable);

        let context_position = args.len() - 1;
        assert_eq!(
            args[context_position - 2],
            "--label",
            "Hookable args should appear before build context: {args:?}"
        );
        assert_eq!(
            args[context_position - 1],
            "foo=bar",
            "Hookable args should appear before build context: {args:?}"
        );
    }
}

mod build {
    use super::*;

    #[test]
    fn no_rebuild_with_existing_image_returns_skipped_no_rebuild() {
        let config = Config {
            no_rebuild: true,
            ..make_config()
        };
        let docker = MockDocker::image_exists_build_succeeds(yesterday_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock, &[])
            .expect("`build` should succeed when image exists and `--no-rebuild` is set");

        assert!(
            matches!(outcome, BuildOutcome::SkippedNoRebuild),
            "Expected `SkippedNoRebuild`, got: {outcome:?}"
        );
    }

    #[test]
    fn no_rebuild_with_no_image_returns_error() {
        let config = Config {
            no_rebuild: true,
            ..make_config()
        };
        let docker = MockDocker::no_image_build_succeeds();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock, &[])
            .expect_err("`build` should fail when no image exists and `--no-rebuild` is set");

        assert!(
            matches!(
                error,
                BuildError::NoRebuildButNoImage { ref image_name }
                if image_name == &default_image_name()
            ),
            "Expected `NoRebuildButNoImage`, got: {error:?}"
        );
    }

    #[test]
    fn fetch_timestamp_failure_returns_staleness_check_error() {
        let config = make_config();
        let docker = MockDocker::fetch_image_last_tag_timestamp_fails();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock, &[])
            .expect_err("`build` should propagate a timestamp fetch failure");

        assert!(
            matches!(error, BuildError::StalenessCheck(_)),
            "Expected `StalenessCheck`, got: {error:?}"
        );
    }

    #[test]
    fn force_rebuild_triggers_build_and_returns_built() {
        // Image exists and would otherwise be considered up to date (last tagged today, Dockerfile
        // `mtime` is old), but `force_rebuild` must bypass the staleness check.
        let config = Config {
            force_rebuild: true,
            ..make_config()
        };
        let docker = MockDocker::image_exists_build_succeeds(today_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock, &[])
            .expect("`build` should succeed with `force_rebuild`");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built`, got: {outcome:?}"
        );
    }

    #[test]
    fn up_to_date_image_returns_up_to_date() {
        // Image last tagged today, Dockerfile mtime is long before the image was tagged.
        let config = make_config();
        let docker = MockDocker::image_exists_build_succeeds(today_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock, &[])
            .expect("`build` should succeed when the image is up to date");

        assert!(
            matches!(outcome, BuildOutcome::UpToDate),
            "Expected `UpToDate`, got: {outcome:?}"
        );
    }

    #[test]
    fn staleness_check_failure_returns_staleness_check_error() {
        // Image exists (so `should_rebuild` is entered), but the filesystem mock fails.
        let config = make_config();
        let docker = MockDocker::image_exists_build_succeeds(yesterday_noon_utc());
        let filesystem = MockFilesystem::failing();
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock, &[])
            .expect_err("`build` should propagate a filesystem error as a staleness check error");

        assert!(
            matches!(error, BuildError::StalenessCheck(_)),
            "Expected `StalenessCheck`, got: {error:?}"
        );
    }

    #[test]
    fn build_failure_with_no_existing_image_returns_no_fallback_error() {
        let config = make_config();
        let docker = MockDocker::no_image_build_fails();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock, &[])
            .expect_err("`build` should fail when the build fails and there is no existing image");

        assert!(
            matches!(
                error,
                BuildError::BuildFailedNoFallback { ref image_name, .. }
                if image_name == &default_image_name()
            ),
            "Expected `BuildFailedNoFallback`, got: {error:?}"
        );
    }

    #[test]
    fn build_failure_with_stale_image_and_allow_stale_returns_using_stale() {
        let config = Config {
            allow_stale: true,
            ..make_config()
        };
        let docker = MockDocker::image_exists_build_fails(yesterday_noon_utc());
        // Dockerfile `mtime` is old so yesterday's image would normally be stale; we just need
        // `should_rebuild` to return `true` so `docker build` is attempted.
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock, &[]).expect(
            "`build` should succeed (using stale) when build fails and `--allow-stale` is set",
        );

        assert!(
            matches!(outcome, BuildOutcome::UsingStaleAfterFailure { .. }),
            "Expected `UsingStaleAfterFailure`, got: {outcome:?}"
        );
    }

    #[test]
    fn build_failure_with_stale_image_and_no_allow_stale_returns_stale_exists_error() {
        let config = make_config();
        let docker = MockDocker::image_exists_build_fails(yesterday_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock, &[]).expect_err(
            "`build` should fail when build fails, stale image exists, and `--allow-stale` is not \
             set",
        );

        assert!(
            matches!(
                error,
                BuildError::BuildFailedStaleExists { ref image_name, .. }
                if image_name == &default_image_name()
            ),
            "Expected `BuildFailedStaleExists`, got: {error:?}"
        );
    }

    #[test]
    fn hookable_args_are_forwarded_to_docker_build() {
        let config = make_config();
        let docker = MockDocker::no_image_build_succeeds();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();
        let hookable_args = vec![String::from("--label"), String::from("foo=bar")];

        let outcome = build(&config, &docker, &filesystem, &clock, &hookable_args)
            .expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built`, got: {outcome:?}"
        );

        // Verify the full assembled args contain the hookable args.
        let received_args = docker.received_build_args.borrow();
        assert!(
            has_flag_pair(&received_args, "--label", "foo=bar"),
            "Expected hookable args in assembled build command: {received_args:?}"
        );
    }
}

mod should_rebuild {
    use super::*;

    #[test]
    fn no_existing_image_triggers_rebuild() {
        let config = make_config();
        // No existing image.
        let docker = MockDocker::no_image_build_succeeds();
        // Filesystem mock is irrelevant because `should_rebuild` short-circuits when there is no
        // existing image, but we provide a value to avoid introducing an unnecessary failure path.
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome =
            build(&config, &docker, &filesystem, &clock, &[]).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built` when no image exists (rebuild always needed), got: {outcome:?}"
        );
    }

    #[test]
    fn dockerfile_newer_than_image_triggers_rebuild() {
        // Image was last tagged at a fixed point in the past.
        let image_last_tagged = long_ago_utc();
        // Dockerfile was modified after the image was last tagged.
        let dockerfile_mtime = image_last_tagged + chrono::Duration::seconds(1);

        let config = make_config();
        let docker = MockDocker::image_exists_build_succeeds(image_last_tagged);
        let filesystem = MockFilesystem::with_mtime(dockerfile_mtime);
        let clock = MockClock::real_now();

        let outcome =
            build(&config, &docker, &filesystem, &clock, &[]).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built` when Dockerfile is newer than the image, got: {outcome:?}"
        );
    }

    #[test]
    fn image_older_than_today_triggers_rebuild() {
        // Image was last tagged yesterday; Dockerfile `mtime` is long before the image.
        let image_last_tagged = yesterday_noon_utc();
        let dockerfile_mtime = long_ago_utc();

        let config = make_config();
        let docker = MockDocker::image_exists_build_succeeds(image_last_tagged);
        let filesystem = MockFilesystem::with_mtime(dockerfile_mtime);
        let clock = MockClock::real_now();

        let outcome =
            build(&config, &docker, &filesystem, &clock, &[]).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built` when the image is older than today, got: {outcome:?}"
        );
    }

    #[test]
    fn image_tagged_today_with_old_dockerfile_is_up_to_date() {
        // Image was last tagged today; Dockerfile was last modified long before the image.
        let image_last_tagged = today_noon_utc();
        let dockerfile_mtime = long_ago_utc();

        let config = make_config();
        let docker = MockDocker::image_exists_build_succeeds(image_last_tagged);
        let filesystem = MockFilesystem::with_mtime(dockerfile_mtime);
        let clock = MockClock::real_now();

        let outcome =
            build(&config, &docker, &filesystem, &clock, &[]).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::UpToDate),
            "Expected `UpToDate` when image was tagged today and Dockerfile is unchanged, \
            got: {outcome:?}"
        );
    }

    #[test]
    fn filesystem_error_reading_dockerfile_mtime_propagates_as_staleness_check_error() {
        // Image exists so `should_rebuild` is entered, but the filesystem mock fails.
        let config = make_config();
        let docker = MockDocker::image_exists_build_succeeds(yesterday_noon_utc());
        let filesystem = MockFilesystem::failing();
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock, &[])
            .expect_err("`build` should propagate the filesystem error");

        assert!(
            matches!(error, BuildError::StalenessCheck(_)),
            "Expected `StalenessCheck`, got: {error:?}"
        );
    }
}
