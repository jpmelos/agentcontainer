//! Abstraction over filesystem operations.

use anyhow::Context as _;
use chrono::{DateTime, Utc};
use std::fs;

/// Abstraction over the filesystem operations required by the build process.
pub(crate) trait Filesystem {
    /// Return the last-modified timestamp of the file at the given path.
    fn file_mtime(&self, path: &str) -> Result<DateTime<Utc>, anyhow::Error>;
}

/// The real filesystem that delegates to `std::fs`.
#[derive(Debug)]
pub(crate) struct RealFilesystem;

impl Filesystem for RealFilesystem {
    fn file_mtime(&self, path: &str) -> Result<DateTime<Utc>, anyhow::Error> {
        let metadata =
            fs::metadata(path).with_context(|| format!("Failed to read metadata for `{path}`"))?;
        let mtime = metadata
            .modified()
            .context("Failed to get modification time of file")?;
        Ok(mtime.into())
    }
}
