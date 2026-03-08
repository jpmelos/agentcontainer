//! Configuration error types.

use std::io::Error as IoError;
use thiserror::Error as ThisError;

/// Errors that can be returned from `get_config`.
#[derive(Debug, ThisError)]
pub(crate) enum ConfigError {
    /// The current working directory could not be determined.
    #[error("Failed to determine the current working directory: {0}")]
    CurrentWorkingDirectoryUnavailable(IoError),
    /// The current working directory is not valid UTF-8.
    #[error("The current working directory is not valid UTF-8")]
    CurrentWorkingDirectoryNotUtf8,
    /// The `dockerfile` path is empty.
    #[error("`dockerfile` must not be empty")]
    EmptyDockerfile,
    /// The `build_context` path is empty.
    #[error("`build_context` must not be empty")]
    EmptyBuildContext,
    /// A build argument CLI argument could not be parsed.
    #[error("Invalid build argument value {value:?}: expected \"KEY=value\" or \"!KEY\"")]
    InvalidBuildArgument {
        /// The raw value that failed parsing.
        value: String,
    },
    /// A build argument key is not a valid identifier.
    #[error(
        "Invalid build argument key {key:?}: must start with a letter or underscore and \
         contain only ASCII letters, digits, and underscores"
    )]
    InvalidBuildArgumentKey {
        /// The key that failed validation.
        key: String,
    },
    /// A `pre_build` entry is an empty string.
    #[error("`pre_build` entries must not be empty strings")]
    EmptyPreBuild,
    /// The project name contains no alphanumeric characters and cannot produce a valid slug.
    #[error("Invalid `project_name` value {project_name:?}: contains no alphanumeric characters")]
    InvalidProjectName {
        /// The raw project name that failed slugification.
        project_name: String,
    },
    /// The username contains no alphanumeric characters and cannot produce a valid slug.
    #[error("Invalid `username` value {username:?}: contains no alphanumeric characters")]
    InvalidUsername {
        /// The raw username that failed slugification.
        username: String,
    },
    /// The `target` value is empty.
    #[error("`target` must not be empty; use \"!\" to suppress an inherited value")]
    EmptyTarget,
    /// The `target` value contains no alphanumeric characters and cannot be slugified.
    #[error("Invalid `target` value {target:?}: contains no alphanumeric characters")]
    InvalidTarget {
        /// The raw target value that failed slugification.
        target: String,
    },
    /// `force_rebuild` and `no_rebuild` were both set, which is contradictory.
    #[error("`force_rebuild` and `no_rebuild` are mutually exclusive")]
    ConflictingRebuildFlags,
    /// A volume value could not be parsed (bad CLI format).
    #[error(
        "Invalid volume value {value:?}: expected \"/host:/container\", \"/path\", or \
         \"!/container\""
    )]
    InvalidVolume {
        /// The raw value that failed parsing.
        value: String,
    },
    /// A volume container path is not absolute.
    #[error("Invalid volume path {path:?}: container paths must be absolute (start with \"/\")")]
    InvalidVolumePath {
        /// The container path that is not absolute.
        path: String,
    },
    /// An environment variable CLI argument could not be parsed.
    #[error(
        "Invalid environment variable value {value:?}: expected \"KEY=value\", \"KEY\", or \
         \"!KEY\""
    )]
    InvalidEnvironmentVariable {
        /// The raw value that failed parsing.
        value: String,
    },
    /// An environment variable key is not a valid identifier.
    #[error(
        "Invalid environment variable key {key:?}: must start with a letter or underscore and \
         contain only ASCII letters, digits, and underscores"
    )]
    InvalidEnvironmentVariableKey {
        /// The key that failed validation.
        key: String,
    },
    /// A `pre_run` entry is an empty string.
    #[error("`pre_run` entries must not be empty strings")]
    EmptyPreRun,
    /// Figment failed to extract the configuration.
    #[error("Failed to load configuration: {0}")]
    Extract(Box<figment::Error>),
}

impl From<figment::Error> for ConfigError {
    fn from(error: figment::Error) -> Self {
        Self::Extract(Box::new(error))
    }
}
