# agentcontainer

A standard way to declare and run agent containers for your projects.

## Installation

```
cargo install --locked agentcontainer
```

## Configuration

`agentcontainer` reads configuration from the following sources, listed from
lowest to highest priority:

| Source                | Path                                   |
| --------------------- | -------------------------------------- |
| XDG global config     | `~/.config/agentcontainer/config.toml` |
| Home dotfile          | `~/.agentcontainer.toml`               |
| Project config        | `.agentcontainer/config.toml`          |
| Local project config  | `.agentcontainer/config.local.toml`    |
| Environment variables | `AGENTCONTAINER_<KEY>`                 |
| CLI arguments         | `--<key>` flags                        |

### Configuration keys

| Key                     | Default                                            | Description                                                                                                       |
| ----------------------- | -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `dockerfile`            | `.agentcontainer/Dockerfile`                       | Path to the Dockerfile.                                                                                           |
| `build_context`         | `.`                                                | Directory used as the Docker build context.                                                                       |
| `project_name`          | Last component of the current directory, slugified | Name used in the Docker image tag.                                                                                |
| `username`              | Current OS user (from `whoami`)                    | Username embedded in the image tag and passed as the `USERNAME` build argument.                                   |
| `target`                | _(none)_                                           | Docker build `--target`. When set, appended to the image tag.                                                     |
| `allow_stale`           | `false`                                            | Use an existing image if the build fails, instead of returning an error.                                          |
| `force_rebuild`         | `false`                                            | Rebuild unconditionally, bypassing the staleness check.                                                           |
| `no_build_cache`        | `false`                                            | Pass `--no-cache` to `docker build`.                                                                              |
| `no_rebuild`            | `false`                                            | Skip the build entirely. Errors if no image exists yet.                                                           |
| `mountpoints`           | _(empty)_                                          | Host-to-container volume mounts. See [Mountpoints](#mountpoints).                                                 |
| `environment_variables` | _(empty)_                                          | Environment variables for the container. See [Container environment variables](#container-environment-variables). |
| `pre_run`               | _(none)_                                           | Path to an executable to run before `docker run`. See [Pre-run hook](#pre-run-hook).                              |

`force_rebuild` and `no_rebuild` are mutually exclusive.

### Mountpoints

The `mountpoints` table maps container paths to host paths. In TOML, each key
is a container path and the value is either:

- A string: an explicit host path to mount at the container path.
- `true`: mount at the same path in the container as on the host (the key is
  used as both host and container path).
- `false`: remove a mountpoint inherited from a lower-priority config source.

```toml
[mountpoints]
"/workspace" = "/home/alice/projects/myproject"
"/data" = "/mnt/shared-data"
"/home/alice/.ssh" = true                       # mount at the same path inside the container
"/unwanted" = false                             # suppress a mountpoint defined in a lower-priority source
```

On the CLI, use `--mountpoint` (repeatable):

```sh
# Mount a host path into the container.
agentcontainer build --mountpoint /home/alice/projects/myproject:/workspace

# Mount at the same path inside the container (same-path shorthand).
agentcontainer build --mountpoint /home/alice/.ssh

# Remove a mountpoint inherited from config files.
agentcontainer build --mountpoint '!/unwanted'
```

### Container environment variables

The `environment_variables` table defines environment variables to pass into
the container. Values can be:

- A string: pass this literal value.
- `true`: inherit the variable from the host environment.
- `false`: remove a variable inherited from a lower-priority config source.

```toml
[environment_variables]
EDITOR = "nvim"
SSH_AUTH_SOCK = true # inherit from host
OLD_VAR = false      # suppress from a lower-priority source
```

On the CLI, use `--environment-variable` (repeatable):

```sh
# Set a literal value.
agentcontainer build --environment-variable EDITOR=nvim

# Inherit from the host.
agentcontainer build --environment-variable SSH_AUTH_SOCK

# Remove a variable inherited from config files.
agentcontainer build --environment-variable '!OLD_VAR'
```

Variable keys must be valid POSIX identifiers: start with a letter or
underscore, followed by ASCII letters, digits, or underscores.

### Pre-run hook

The `pre_run` option specifies a path to an executable that runs before
`docker run`. Its stdout is parsed as a TOML array of strings, and these
strings are injected as extra arguments to the `docker run` command (after all
built-in flags, but before the image name).

This provides a way to dynamically compute Docker flags at runtime based on the
host environment.

The hook must:

- Exit with status 0.
- Print a valid TOML array of strings to stdout (e.g. `["--network", "host"]`).
- Produce valid UTF-8 output.

Example hook script:

```sh
#!/bin/sh
echo '["--network", "host"]'
```

In TOML configuration:

```toml
pre_run = "./hooks/pre-run.sh"
```

On the CLI:

```sh
agentcontainer run --pre-run ./hooks/pre-run.sh
```

### Image naming

The Docker image tag is derived from the resolved configuration:

```
agentcontainer_<username>_<project_name>:latest
agentcontainer_<username>_<project_name>_<target>:latest  # when target is set
```

`username`, `project_name`, and `target` are all slugified before being
embedded in the tag: lowercased, non-alphanumeric characters replaced with `_`,
consecutive underscores collapsed, and leading/trailing underscores trimmed.

`username` and `project_name` must contain at least one alphanumeric character.
If the slug of either value would be empty, `get_config` returns an error.

### Container naming

When running a container, the name is derived from the project name and a
random numeric suffix:

```
agentcontainer_<project_name>_<suffix>
```

The slugified project name is truncated to 41 characters (with any trailing
underscore removed after truncation) so that the full container name never
exceeds Docker's 63-character limit for container names.

### Example configuration file

```toml
dockerfile = ".agentcontainer/Dockerfile"
build_context = "."
project_name = "myproject"
username = "alice"

[mountpoints]
"/workspace" = "/home/alice/projects/myproject"
"/home/alice/.ssh" = true  # same path on host and in container

[environment_variables]
EDITOR = "nvim"
SSH_AUTH_SOCK = true
```

### Environment variables

Each configuration key maps to an `AGENTCONTAINER_<KEY>` environment variable,
where `<KEY>` is the uppercase version of the key name. Values are parsed as
TOML. For example:

```sh
AGENTCONTAINER_DOCKERFILE=".agentcontainer/Dockerfile"
AGENTCONTAINER_BUILD_CONTEXT="."
AGENTCONTAINER_PROJECT_NAME="myproject"
AGENTCONTAINER_USERNAME="alice"
AGENTCONTAINER_MOUNTPOINTS='{"/workspace" = "/home/alice/projects/myproject", "/home/alice/.ssh" = true}'
AGENTCONTAINER_ENVIRONMENT_VARIABLES='{EDITOR = "nvim", SSH_AUTH_SOCK = true}'
AGENTCONTAINER_PRE_RUN="./hooks/pre-run.sh"
```

## Commands

### `config`

Print the resolved configuration, with all sources merged, as TOML.

```
agentcontainer config
```

### `build`

Build the agent container Docker image.

```
agentcontainer build
```

The build is skipped if the image is already up to date. A rebuild is triggered
when any of the following is true:

- No image exists yet.
- The Dockerfile was modified after the image was created.
- The image was created before the start of today (local time).
- `force_rebuild` is set.

The following build arguments are passed automatically:

| Build argument | Value                                |
| -------------- | ------------------------------------ |
| `USERNAME`     | The raw `username` config value.     |
| `BUILD_DATE`   | Today's date in `YYYY-MM-DD` format. |

### `run`

Run the agent container. The image is automatically built (or rebuilt if stale)
before starting the container. This replaces the current process with
`docker run` via `exec`.

```
agentcontainer run [-- <container-args>...]
```

Arguments after `--` are passed through to the container's entrypoint.
Everything before `--` is parsed by `agentcontainer` itself and will error on
unrecognized flags.

```sh
# Run interactively with no extra arguments.
agentcontainer run

# Pass arguments to the container entrypoint.
agentcontainer run -- --print --output-format json

# Global flags still go before the subcommand.
agentcontainer --project-name foo run -- --help
```

The build step honors all build-related configuration keys (`force_rebuild`,
`no_rebuild`, `allow_stale`, `no_build_cache`, etc.). If the image is already
up to date, the build is skipped and the container starts immediately.

The container is started with:

- **`--init`**: the container uses an init process.
- **`--rm`**: the container is automatically removed on exit.
- **UID/GID mapping**: the container runs as the current user and group, with
  group `0` added via `--group-add`.
- **Current directory mount**: the working directory is mounted into the
  container at the same path and set as the container's working directory.
- **Git worktree mount**: if the current directory is a linked Git worktree,
  the main worktree root is also mounted so that Git objects are accessible.
- **Configured mountpoints and environment variables**: as defined in the
  configuration.
- **TTY mode**: `-t` (allocate pseudo-TTY) and `-i` (keep stdin open) are only
  added when standard input is a TTY. This means piped or scripted invocations
  won't cause Docker to hang or emit spurious warnings.
