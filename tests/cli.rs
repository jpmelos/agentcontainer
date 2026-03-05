//! Integration tests for the `agentcontainer` CLI binary.

#![allow(
    unused_crate_dependencies,
    reason = "Integration test crates inherit all development dependencies from `Cargo.toml`, even \
        those not used in this file. This is a known Cargo limitation."
)]

#[cfg(test)]
mod tests {
    use assert_cmd::{cargo_bin, prelude::OutputAssertExt as _};
    use predicates::str::{contains, diff};
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

        // Compute the expected `project_name` default the same way the production code does:
        // the last component of the CWD path, as-is.
        let expected_project_name = cwd
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_owned();

        // Compute the expected `username` default the same way the production code does.
        let expected_username = whoami::username().unwrap_or_else(|_| String::from("unknown"));

        // Unset `AGENTCONTAINER_*` environment variables in case they are set in the environment,
        // which would override the defaults we are testing.
        let mut command = Command::new(cargo_bin!("agentcontainer"));
        command
            .arg("config")
            .env("HOME", home_dir.path())
            .env_remove("AGENTCONTAINER_DOCKERFILE")
            .env_remove("AGENTCONTAINER_PROJECT_NAME")
            .env_remove("AGENTCONTAINER_USERNAME")
            .env_remove("AGENTCONTAINER_TARGET")
            .env_remove("AGENTCONTAINER_ALLOW_STALE")
            .env_remove("AGENTCONTAINER_FORCE_REBUILD")
            .env_remove("AGENTCONTAINER_NO_BUILD_CACHE")
            .env_remove("AGENTCONTAINER_NO_REBUILD")
            .env_remove("AGENTCONTAINER_MOUNTPOINTS")
            .env_remove("AGENTCONTAINER_ENVIRONMENT_VARIABLES")
            .current_dir(cwd.path());

        let expected_output = format!(
            "dockerfile = \".agentcontainer/Dockerfile\"\n\
             project_name = \"{expected_project_name}\"\n\
             username = \"{expected_username}\"\n"
        );

        command.assert().success().stdout(diff(expected_output));
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
