use super::{BuildError, BuildOutcome, DockerBuildError, build};
use crate::config::Config;
use crate::utils::clock::Clock;
use crate::utils::docker::DockerBackend;
use crate::utils::fs::Filesystem;
use anyhow::anyhow;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone as _, Utc};
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Error as IoError;

// ---------------------------------------------------------------------------
// Helpers: minimal Config constructor.
// ---------------------------------------------------------------------------

/// Construct a `Config` for use in tests, without going through CLI parsing or `figment`.
fn make_config(
    dockerfile: &str,
    allow_stale: bool,
    force_rebuild: bool,
    no_rebuild: bool,
) -> Config {
    Config {
        dockerfile: String::from(dockerfile),
        build_context: String::from("."),
        project_name: String::from("myproject"),
        username: String::from("alice"),
        target: None,
        allow_stale,
        force_rebuild,
        no_build_cache: false,
        no_rebuild,
        volumes: HashMap::new(),
        environment_variables: HashMap::new(),
        pre_run: None,
    }
}

/// Return the image name produced by the default `make_config` config.
fn default_image_name() -> String {
    make_config(".agentcontainer/Dockerfile", false, false, false).get_image_name()
}

// ---------------------------------------------------------------------------
// Helpers: fixed timestamps for deterministic time-based logic.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Mocks.
// ---------------------------------------------------------------------------

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
    /// Value returned by `fetch_image_creation_timestamp`.
    existing_image_created: Result<Option<DateTime<Utc>>, anyhow::Error>,
    /// Value returned by `run_docker_build`.
    build_result: Result<(), DockerBuildError>,
}

impl MockDocker {
    /// The image does not exist and `docker build` succeeds.
    fn no_image_build_succeeds() -> Self {
        Self {
            existing_image_created: Ok(None),
            build_result: Ok(()),
        }
    }

    /// The image does not exist and `docker build` fails with a non-zero exit status.
    fn no_image_build_fails() -> Self {
        Self {
            existing_image_created: Ok(None),
            build_result: Err(make_non_zero_exit_error()),
        }
    }

    /// The image exists (created at the given time) and `docker build` succeeds.
    fn image_exists_build_succeeds(created: DateTime<Utc>) -> Self {
        Self {
            existing_image_created: Ok(Some(created)),
            build_result: Ok(()),
        }
    }

    /// The image exists (created at the given time) and `docker build` fails.
    fn image_exists_build_fails(created: DateTime<Utc>) -> Self {
        Self {
            existing_image_created: Ok(Some(created)),
            build_result: Err(make_non_zero_exit_error()),
        }
    }

    /// `fetch_image_creation_timestamp` itself returns an error.
    fn fetch_image_creation_timestamp_fails() -> Self {
        Self {
            existing_image_created: Err(anyhow!("docker inspect failed")),
            build_result: Ok(()),
        }
    }
}

impl DockerBackend for MockDocker {
    fn fetch_image_creation_timestamp(
        &self,
        _image_name: &str,
    ) -> Result<Option<DateTime<Utc>>, anyhow::Error> {
        match self.existing_image_created.as_ref() {
            Ok(value) => Ok(*value),
            Err(error) => Err(anyhow!("{error}")),
        }
    }

    fn run_docker_build(
        &self,
        _config: &Config,
        _image_name: &str,
        _build_date: &str,
    ) -> Result<(), DockerBuildError> {
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

// ---------------------------------------------------------------------------
// Tests for `build()`.
// ---------------------------------------------------------------------------

mod build {
    use super::*;

    #[test]
    fn no_rebuild_with_existing_image_returns_skipped_no_rebuild() {
        let config = make_config(".agentcontainer/Dockerfile", false, false, true);
        let docker = MockDocker::image_exists_build_succeeds(yesterday_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock)
            .expect("`build` should succeed when image exists and `--no-rebuild` is set");

        assert!(
            matches!(outcome, BuildOutcome::SkippedNoRebuild),
            "Expected `SkippedNoRebuild`, got: {outcome:?}"
        );
    }

    #[test]
    fn no_rebuild_with_no_image_returns_error() {
        let config = make_config(".agentcontainer/Dockerfile", false, false, true);
        let docker = MockDocker::no_image_build_succeeds();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock)
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
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::fetch_image_creation_timestamp_fails();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock)
            .expect_err("`build` should propagate a timestamp fetch failure");

        assert!(
            matches!(error, BuildError::StalenessCheck(_)),
            "Expected `StalenessCheck`, got: {error:?}"
        );
    }

    #[test]
    fn force_rebuild_triggers_build_and_returns_built() {
        // Image exists and would otherwise be considered up to date (created today, Dockerfile
        // `mtime` is old), but `force_rebuild` must bypass the staleness check.
        let config = make_config(".agentcontainer/Dockerfile", false, true, false);
        let docker = MockDocker::image_exists_build_succeeds(today_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock)
            .expect("`build` should succeed with `force_rebuild`");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built`, got: {outcome:?}"
        );
    }

    #[test]
    fn up_to_date_image_returns_up_to_date() {
        // Image created today, Dockerfile mtime is long before the image was built.
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_succeeds(today_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock)
            .expect("`build` should succeed when the image is up to date");

        assert!(
            matches!(outcome, BuildOutcome::UpToDate),
            "Expected `UpToDate`, got: {outcome:?}"
        );
    }

    #[test]
    fn staleness_check_failure_returns_staleness_check_error() {
        // Image exists (so `should_rebuild` is entered), but the filesystem mock fails.
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_succeeds(yesterday_noon_utc());
        let filesystem = MockFilesystem::failing();
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock)
            .expect_err("`build` should propagate a filesystem error as a staleness check error");

        assert!(
            matches!(error, BuildError::StalenessCheck(_)),
            "Expected `StalenessCheck`, got: {error:?}"
        );
    }

    #[test]
    fn build_failure_with_no_existing_image_returns_no_fallback_error() {
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::no_image_build_fails();
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock)
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
        let config = make_config(".agentcontainer/Dockerfile", true, false, false);
        let docker = MockDocker::image_exists_build_fails(yesterday_noon_utc());
        // Dockerfile `mtime` is old so yesterday's image would normally be stale; we just need
        // `should_rebuild` to return `true` so `docker build` is attempted.
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock).expect(
            "`build` should succeed (using stale) when build fails and `--allow-stale` is set",
        );

        assert!(
            matches!(outcome, BuildOutcome::UsingStaleAfterFailure { .. }),
            "Expected `UsingStaleAfterFailure`, got: {outcome:?}"
        );
    }

    #[test]
    fn build_failure_with_stale_image_and_no_allow_stale_returns_stale_exists_error() {
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_fails(yesterday_noon_utc());
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock).expect_err(
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
    fn build_date_passed_to_docker_is_formatted_as_yyyy_mm_dd() {
        use std::cell::Cell;

        /// A `DockerBackend` that captures the `build_date` argument.
        struct CapturingDocker {
            /// The `build_date` string received by `run_docker_build`.
            captured_build_date: Cell<Option<String>>,
        }

        impl DockerBackend for CapturingDocker {
            fn fetch_image_creation_timestamp(
                &self,
                _image_name: &str,
            ) -> Result<Option<DateTime<Utc>>, anyhow::Error> {
                // No existing image, so a build will always be triggered.
                Ok(None)
            }

            fn run_docker_build(
                &self,
                _config: &Config,
                _image_name: &str,
                build_date: &str,
            ) -> Result<(), DockerBuildError> {
                self.captured_build_date.set(Some(String::from(build_date)));
                Ok(())
            }

            fn exec_docker_run(&self, _args: &[String]) -> Result<Infallible, IoError> {
                unreachable!("Build tests do not call `exec_docker_run`")
            }
        }

        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = CapturingDocker {
            captured_build_date: Cell::new(None),
        };
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        build(&config, &docker, &filesystem, &clock).expect("`build` should succeed");

        let build_date = docker
            .captured_build_date
            .take()
            .expect("`run_docker_build` should have been called");

        // Must match `YYYY-MM-DD`.
        let today = clock.now().format("%Y-%m-%d").to_string();
        assert_eq!(
            build_date, today,
            "Expected build date `{today}`, got `{build_date}`"
        );
    }
}

mod should_rebuild_fn {
    // ---------------------------------------------------------------------------
    // Tests for `should_rebuild()` via `build()`.
    //
    // `should_rebuild` is private, so we drive it through `build()` with
    // `force_rebuild = false`, `no_rebuild = false`, and a docker mock that
    // always succeeds. The outcome (`UpToDate` vs. `Built`) reveals what
    // `should_rebuild` returned.
    // ---------------------------------------------------------------------------
    use super::*;

    #[test]
    fn no_existing_image_triggers_rebuild() {
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        // No existing image.
        let docker = MockDocker::no_image_build_succeeds();
        // Filesystem mock is irrelevant because `should_rebuild` short-circuits when there is no
        // existing image, but we provide a value to avoid introducing an unnecessary failure path.
        let filesystem = MockFilesystem::with_mtime(long_ago_utc());
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built` when no image exists (rebuild always needed), got: {outcome:?}"
        );
    }

    #[test]
    fn dockerfile_newer_than_image_triggers_rebuild() {
        // Image was created at a fixed point in the past.
        let image_created = long_ago_utc();
        // Dockerfile was modified after the image was created.
        let dockerfile_mtime = image_created + chrono::Duration::seconds(1);

        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_succeeds(image_created);
        let filesystem = MockFilesystem::with_mtime(dockerfile_mtime);
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built` when Dockerfile is newer than the image, got: {outcome:?}"
        );
    }

    #[test]
    fn image_older_than_today_triggers_rebuild() {
        // Image was created yesterday; Dockerfile `mtime` is long before the image.
        let image_created = yesterday_noon_utc();
        let dockerfile_mtime = long_ago_utc();

        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_succeeds(image_created);
        let filesystem = MockFilesystem::with_mtime(dockerfile_mtime);
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::Built),
            "Expected `Built` when the image is older than today, got: {outcome:?}"
        );
    }

    #[test]
    fn image_created_today_with_old_dockerfile_is_up_to_date() {
        // Image was created today; Dockerfile was last modified long before the image.
        let image_created = today_noon_utc();
        let dockerfile_mtime = long_ago_utc();

        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_succeeds(image_created);
        let filesystem = MockFilesystem::with_mtime(dockerfile_mtime);
        let clock = MockClock::real_now();

        let outcome = build(&config, &docker, &filesystem, &clock).expect("`build` should succeed");

        assert!(
            matches!(outcome, BuildOutcome::UpToDate),
            "Expected `UpToDate` when image was built today and Dockerfile is unchanged, \
            got: {outcome:?}"
        );
    }

    #[test]
    fn filesystem_error_reading_dockerfile_mtime_propagates_as_staleness_check_error() {
        // Image exists so `should_rebuild` is entered, but the filesystem mock fails.
        let config = make_config(".agentcontainer/Dockerfile", false, false, false);
        let docker = MockDocker::image_exists_build_succeeds(yesterday_noon_utc());
        let filesystem = MockFilesystem::failing();
        let clock = MockClock::real_now();

        let error = build(&config, &docker, &filesystem, &clock)
            .expect_err("`build` should propagate the filesystem error");

        assert!(
            matches!(error, BuildError::StalenessCheck(_)),
            "Expected `StalenessCheck`, got: {error:?}"
        );
    }
}
