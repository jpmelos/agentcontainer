//! Git-related utilities.

use anyhow::Context as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// Abstraction over Git operations.
pub(crate) trait GitContext {
    /// Return the root of the main worktree if `current_dir` is inside a linked worktree.
    ///
    /// Returns `Ok(None)` if `current_dir` is not a linked worktree (i.e., `.git` is a directory
    /// or does not exist). Returns `Ok(Some(path))` if `current_dir` is a linked worktree and the
    /// main worktree root differs from `current_dir`.
    fn main_worktree_root(&self, current_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error>;
}

/// Real implementation that shells out to the `git` CLI.
#[derive(Debug)]
pub(crate) struct RealGitContext;

impl GitContext for RealGitContext {
    fn main_worktree_root(&self, current_dir: &Path) -> Result<Option<PathBuf>, anyhow::Error> {
        debug!(?current_dir, "Detecting Git worktree");

        let git_path = current_dir.join(".git");

        let Ok(metadata) = fs::symlink_metadata(&git_path) else {
            debug!("No .git found, not a Git repository");
            return Ok(None);
        };

        // If `.git` is a directory, this is a normal repository, not a linked worktree.
        if metadata.is_dir() {
            debug!(".git is a directory, not a linked worktree");
            return Ok(None);
        }

        // `.git` is a file, indicating a linked worktree. Find the main worktree.
        debug!(".git is a file, detecting linked worktree");
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(current_dir)
            .output()
            .context("Failed to run `git worktree list --porcelain`")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`git worktree list --porcelain` failed: {stderr}");
        }

        let stdout = String::from_utf8(output.stdout)
            .context("`git worktree list` output is not valid UTF-8")?;

        for line in stdout.lines() {
            if let Some(path_str) = line.strip_prefix("worktree ") {
                let main_worktree = PathBuf::from(path_str);
                if main_worktree != current_dir {
                    debug!(?main_worktree, "Found main worktree root");
                    return Ok(Some(main_worktree));
                }
                debug!("Current directory is the main worktree");
                return Ok(None);
            }
        }

        Ok(None)
    }
}
