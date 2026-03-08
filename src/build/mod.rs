//! Build the agent container Docker image.

use crate::config::Config;
use crate::utils::clock::Clock;
use crate::utils::docker::{DockerBackend, DockerBuildError};
use crate::utils::fs::Filesystem;
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::debug;

/// Successful outcomes from `build` that the presentation layer may want to report.
#[derive(Debug)]
pub(crate) enum BuildOutcome {
    /// The build succeeded.
    Built,
    /// The image is up to date and no rebuild was necessary.
    UpToDate,
    /// The `--no-rebuild` flag was set and a valid image already exists.
    SkippedNoRebuild,
    /// The build failed but `--allow-stale` was set and a previous image is being used.
    UsingStaleAfterFailure {
        /// The error from the failed build attempt.
        build_error: DockerBuildError,
    },
}

/// Errors that can be returned from `build`.
#[derive(Debug, Error)]
pub(crate) enum BuildError {
    /// `--no-rebuild` was specified but no image exists yet.
    #[error("Image `{image_name}` does not exist and `--no-rebuild` was specified")]
    NoRebuildButNoImage {
        /// The image name that was expected to exist.
        image_name: String,
    },
    /// An internal error occurred while checking whether a rebuild is needed.
    #[error("Failed to determine whether the image needs to be rebuilt: {0}")]
    StalenessCheck(#[source] anyhow::Error),
    /// `docker build` failed and there is no existing image to fall back to.
    #[error("Build of `{image_name}` failed and no existing image is available")]
    BuildFailedNoFallback {
        /// The image name that failed to build.
        image_name: String,
        /// The underlying `docker build` error.
        #[source]
        build_error: DockerBuildError,
    },
    /// The docker build failed and a stale image exists, but `--allow-stale` was not set.
    #[error(
        "Build of `{image_name}` failed and a stale image exists; use `--allow-stale` to use it"
    )]
    BuildFailedStaleExists {
        /// The image name that failed to build.
        image_name: String,
        /// The underlying `docker build` error.
        #[source]
        build_error: DockerBuildError,
    },
}

/// Build the agent container image according to the configuration.
pub(crate) fn build(
    config: &Config,
    docker: &impl DockerBackend,
    filesystem: &impl Filesystem,
    clock: &impl Clock,
    pre_build_extra_args: &[String],
) -> Result<BuildOutcome, BuildError> {
    let image_name = config.get_image_name();
    debug!(%image_name, "Checking if image needs to be rebuilt");

    let existing_image_created_at = docker
        .fetch_image_creation_timestamp(&image_name)
        .map_err(BuildError::StalenessCheck)?;
    let image_exists = existing_image_created_at.is_some();
    debug!(?existing_image_created_at, "Image existence check complete");

    if config.no_rebuild {
        if !image_exists {
            debug!("No rebuild requested, but no image exists");
            return Err(BuildError::NoRebuildButNoImage { image_name });
        }
        debug!("No rebuild requested");
        return Ok(BuildOutcome::SkippedNoRebuild);
    }

    let needs_rebuild = config.force_rebuild
        || should_rebuild(config, existing_image_created_at, filesystem, clock)
            .map_err(BuildError::StalenessCheck)?;
    if !needs_rebuild {
        debug!("Image is up to date, no rebuild needed");
        return Ok(BuildOutcome::UpToDate);
    }

    if config.force_rebuild {
        debug!("Force rebuild requested");
    }

    debug!(%image_name, "Building image");
    match docker.run_docker_build(config, &image_name, pre_build_extra_args) {
        Ok(()) => {
            debug!("Image built");
            Ok(BuildOutcome::Built)
        }
        Err(build_error) => {
            if config.allow_stale && image_exists {
                debug!(%build_error, "Image failed to build; using stale");
                Ok(BuildOutcome::UsingStaleAfterFailure { build_error })
            } else if image_exists {
                debug!(%build_error, "Image failed to build; stale exists but not requested");
                Err(BuildError::BuildFailedStaleExists {
                    image_name,
                    build_error,
                })
            } else {
                debug!(%build_error, "Image failed to build; no existing image");
                Err(BuildError::BuildFailedNoFallback {
                    image_name,
                    build_error,
                })
            }
        }
    }
}

/// Determine whether the image needs to be rebuilt.
///
/// Returns `true` if:
/// - The image does not exist (`existing_image_created_at` is `None`).
/// - The Dockerfile was modified after the image was created.
/// - The image was created before the start of today (local time).
fn should_rebuild(
    config: &Config,
    existing_image_created_at: Option<DateTime<Utc>>,
    filesystem: &impl Filesystem,
    clock: &impl Clock,
) -> Result<bool, anyhow::Error> {
    let Some(image_created_at) = existing_image_created_at else {
        debug!("No existing image found, rebuild needed");
        return Ok(true);
    };

    // Get Dockerfile modification time.
    let dockerfile_mtime = filesystem.file_mtime(&config.dockerfile)?;
    debug!(
        ?dockerfile_mtime,
        ?image_created_at,
        "Comparing Dockerfile mtime to image creation time"
    );

    if dockerfile_mtime > image_created_at {
        debug!("Dockerfile modified after image was created, rebuild needed");
        return Ok(true);
    }

    // Check if the image was created before the start of today (local time).
    let now_local = clock.now();
    let today_local = now_local.date_naive();
    let image_created_at_local = image_created_at
        .with_timezone(now_local.offset())
        .date_naive();
    debug!(
        ?image_created_at_local,
        ?today_local,
        "Comparing image creation time to current time"
    );
    if image_created_at_local < today_local {
        debug!(
            %image_created_at_local,
            %today_local,
            "Image was created before today, rebuild needed",
        );
        return Ok(true);
    }

    debug!("Rebuild not needed");
    Ok(false)
}

#[cfg(test)]
mod tests;
