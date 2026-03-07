//! Abstraction over Docker CLI operations.

use crate::config::{BuildArgumentEntry, Config};
use anyhow::Context as _;
use chrono::{DateTime, Utc};
use std::convert::Infallible;
use std::io::Error as IoError;
use std::os::unix::process::CommandExt as _;
use std::process::{Command, ExitStatus, Stdio};
use thiserror::Error;

/// An error produced by the `docker build` invocation.
#[derive(Debug, Error)]
pub(crate) enum DockerBuildError {
    /// The `docker build` process could not be spawned.
    #[error("Failed to run `docker build`: {0}")]
    SpawnFailed(#[source] IoError),
    /// The `docker build` process exited with a non-zero status.
    #[error("`docker build` exited with {0}")]
    NonZeroExit(ExitStatus),
}

/// Abstraction over the Docker CLI operations required by the build process.
pub(crate) trait DockerBackend {
    /// Fetch the creation timestamp of a Docker image, if it exists locally.
    ///
    /// Returns `Some(timestamp)` if the image exists, or `None` if it does not. Returns an error
    /// if the `docker image inspect` command fails for a reason other than the image not existing,
    /// or if the timestamp cannot be parsed.
    fn fetch_image_creation_timestamp(
        &self,
        image_name: &str,
    ) -> Result<Option<DateTime<Utc>>, anyhow::Error>;

    /// Run `docker build` with the given configuration, image name, and extra arguments from the
    /// pre-build hook.
    fn run_docker_build(
        &self,
        config: &Config,
        image_name: &str,
        pre_build_extra_args: &[String],
    ) -> Result<(), DockerBuildError>;

    /// Replace the current process with `docker run` using the given arguments.
    ///
    /// On success, the current process is replaced and this method never returns. On failure,
    /// returns the I/O error from the `exec` system call.
    fn exec_docker_run(&self, args: &[String]) -> Result<Infallible, IoError>;
}

/// The real Docker backend that shells out to the `docker` CLI.
#[derive(Debug)]
pub(crate) struct RealDockerBackend;

impl DockerBackend for RealDockerBackend {
    fn fetch_image_creation_timestamp(
        &self,
        image_name: &str,
    ) -> Result<Option<DateTime<Utc>>, anyhow::Error> {
        let output = Command::new("docker")
            .args(["image", "inspect", image_name, "--format", "{{.Created}}"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to run `docker image inspect`")?;

        if !output.status.success() {
            // A non-zero exit status means the image does not exist.
            return Ok(None);
        }

        let timestamp_str = String::from_utf8(output.stdout)
            .context("`docker image inspect` output is not valid UTF-8")?;
        let timestamp_str = timestamp_str.trim();
        let image_created: DateTime<Utc> = timestamp_str
            .parse::<DateTime<Utc>>()
            .context("Failed to parse image creation timestamp as RFC 3339")?;

        Ok(Some(image_created))
    }

    fn run_docker_build(
        &self,
        config: &Config,
        image_name: &str,
        pre_build_extra_args: &[String],
    ) -> Result<(), DockerBuildError> {
        let mut command = Command::new("docker");
        command.arg("build");
        command.args(["-f", &config.dockerfile]);

        if let Some(ref target) = config.target {
            command.args(["--target", target]);
        }

        for (key, entry) in &config.build_arguments {
            match *entry {
                BuildArgumentEntry::Value(ref value) => {
                    command.args(["--build-arg", &format!("{key}={value}")]);
                }
                BuildArgumentEntry::Inherit => {
                    command.args(["--build-arg", key]);
                }
                BuildArgumentEntry::Remove => {
                    unreachable!(
                        "`Remove` entries are stripped by `clean_config` before `build` is called."
                    )
                }
            }
        }

        if config.no_build_cache {
            command.arg("--no-cache");
        }

        command.args(["-t", image_name]);

        // Extra arguments from the pre-build hook.
        command.args(pre_build_extra_args);

        command.arg(&config.build_context);

        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        let status = command.status().map_err(DockerBuildError::SpawnFailed)?;

        if status.success() {
            Ok(())
        } else {
            Err(DockerBuildError::NonZeroExit(status))
        }
    }

    fn exec_docker_run(&self, args: &[String]) -> Result<Infallible, IoError> {
        let error = Command::new("docker").args(args).exec();
        Err(error)
    }
}
