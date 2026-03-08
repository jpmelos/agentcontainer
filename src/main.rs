//! # agentcontainer
//!
//! A standard way to declare and run agent containers for your projects.

mod build;
mod config;
mod hooks;
mod run;
mod utils;

use anyhow::{Context as _, Result};
use build::BuildOutcome;
use clap::Parser as _;
use config::{CliArgs, Command};
use std::env;
use std::io::{IsTerminal as _, stdin};
use tracing::{info, warn};
use utils::clock::SystemClock;
use utils::docker::RealDockerBackend;
use utils::fs::RealFilesystem;
use utils::git::RealGitContext;

#[expect(
    clippy::print_stdout,
    reason = "This is a CLI application that needs to print output to stdout."
)]
fn main() -> Result<()> {
    utils::logging::init()?;

    let home_dir = env::var("HOME").context("HOME environment variable is not set")?;
    let cli_args = CliArgs::parse();

    let (command_ref, config) = config::get_config(&home_dir, &cli_args)?;

    match *command_ref {
        Command::Config => {
            let output = toml::to_string_pretty(&config)?;
            print!("{output}");
        }
        Command::Build => {
            handle_build(&config)?;
        }
        Command::Run { ref container_args } => {
            handle_build(&config)?;

            let current_dir = env::current_dir()
                .context("Failed to get current working directory")?
                .to_str()
                .context("Current directory path is not valid UTF-8")?
                .to_owned();

            // SAFETY: `getuid` is always safe to call; it merely reads the process's real UID.
            let uid = unsafe { libc::getuid() };
            // SAFETY: `getgid` is always safe to call; it merely reads the process's real GID.
            let gid = unsafe { libc::getgid() };

            let random_suffix = utils::random::random_name_suffix();
            let stdin_is_terminal = stdin().is_terminal();

            let pre_run_extra_args = hooks::execute_pre_run_hooks(&config.pre_run)?;

            info!("Running agent container");

            match run::run(
                &config,
                &RealDockerBackend,
                &RealGitContext,
                uid,
                gid,
                &current_dir,
                random_suffix,
                container_args,
                stdin_is_terminal,
                &pre_run_extra_args,
            ) {
                Ok(infallible) => match infallible {},
                Err(error) => return Err(error.into()),
            }
        }
    }

    Ok(())
}

/// Build the image and report the outcome to stderr.
fn handle_build(config: &config::Config) -> Result<()> {
    let pre_build_extra_args = hooks::execute_pre_build_hooks(&config.pre_build)?;

    info!("Building agent container");

    match build::build(
        config,
        &RealDockerBackend,
        &RealFilesystem,
        &SystemClock,
        &pre_build_extra_args,
    ) {
        Ok(BuildOutcome::SkippedNoRebuild) => {
            info!("Skipping rebuild (--no-rebuild)");
        }
        Ok(BuildOutcome::UpToDate) => {
            info!("Image is up to date, skipping build");
        }
        Ok(BuildOutcome::Built) => {
            info!("Image built");
        }
        Ok(BuildOutcome::UsingStaleAfterFailure { build_error }) => {
            warn!(%build_error, "Build failed but `--allow-stale` is set; using existing image");
        }
        Err(error) => {
            return Err(error.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // These development dependencies are only used in `tests/cli.rs` (integration tests), not in
    // the binary crate itself. The `unused_crate_dependencies` lint requires that every dependency
    // be referenced somewhere in the crate under test.
    use assert_cmd as _;
    use predicates as _;
    use tempfile as _;
}
