//! Integration tests for the `agentcontainer` CLI binary.

#![allow(
    unused_crate_dependencies,
    reason = "Integration test crates inherit all development dependencies from `Cargo.toml`, even \
    those not used in this file. This is a known Cargo limitation."
)]

#[cfg(test)]
mod tests {
    use assert_cmd::{cargo_bin, prelude::OutputAssertExt as _};
    use predicates::str::contains;
    use std::{fs, path::Path, process::Command};
    use tempfile::tempdir;

    /// Write content to a file, creating parent directories as needed.
    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories for test file.");
        }
        fs::write(path, content).expect("Failed to write test file.");
    }

    #[test]
    fn config_subcommand_prints_default_config() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");

        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env("HOME", home_dir.path())
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = ".agentcontainer/Dockerfile""#));
    }

    #[test]
    fn config_subcommand_respects_dockerfile_cli_flag() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");

        Command::new(cargo_bin!("agentcontainer"))
            .args(["--dockerfile", "custom.Dockerfile", "config"])
            .env("HOME", home_dir.path())
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = "custom.Dockerfile""#));
    }

    #[test]
    fn config_subcommand_respects_env_var() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");

        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env("HOME", home_dir.path())
            .env("AGENTCONTAINER_DOCKERFILE", "from-env")
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = "from-env""#));
    }

    #[test]
    fn config_subcommand_respects_cwd_config_file() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.toml"),
            r#"dockerfile = "from-cwd""#,
        );

        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env("HOME", home_dir.path())
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = "from-cwd""#));
    }

    #[test]
    fn config_subcommand_respects_cwd_local_config_file() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &cwd.path().join(".agentcontainer/config.local.toml"),
            r#"dockerfile = "from-cwd-local""#,
        );

        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env("HOME", home_dir.path())
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = "from-cwd-local""#));
    }

    #[test]
    fn config_subcommand_respects_home_dotfile() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".agentcontainer.toml"),
            r#"dockerfile = "from-home-dotfile""#,
        );

        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env("HOME", home_dir.path())
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = "from-home-dotfile""#));
    }

    #[test]
    fn config_subcommand_respects_xdg_config_file() {
        let home_dir = tempdir().expect("Failed to create temporary directory.");
        let cwd = tempdir().expect("Failed to create temporary directory.");
        write_file(
            &home_dir.path().join(".config/agentcontainer/config.toml"),
            r#"dockerfile = "from-xdg""#,
        );

        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env("HOME", home_dir.path())
            .current_dir(cwd.path())
            .assert()
            .success()
            .stdout(contains(r#"dockerfile = "from-xdg""#));
    }

    #[test]
    fn missing_home_env_var_produces_useful_error() {
        Command::new(cargo_bin!("agentcontainer"))
            .arg("config")
            .env_remove("HOME")
            .assert()
            .failure()
            .stderr(contains("HOME environment variable is not set"));
    }
}
