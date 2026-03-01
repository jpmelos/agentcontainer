//! # agentcontainer
//!
//! A standard way to declare and run agent containers for your projects.

mod config;

use anyhow::{Context as _, Result};
use clap::Parser as _;
use config::{CliArgs, Command};
use std::env;

#[expect(
    clippy::print_stdout,
    reason = "This is a CLI application that needs to print output to stdout."
)]
fn main() -> Result<()> {
    let home_dir = env::var("HOME").context("HOME environment variable is not set")?;
    let cli_args = CliArgs::parse();

    let (command_ref, config) = config::get_config(&home_dir, &cli_args)?;

    match command_ref {
        &Command::Config => {
            let output = toml::to_string_pretty(&config)?;
            println!("{output}");
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
