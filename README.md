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
| `project_name`          | Last component of the current directory, slugified | Name used in the Docker image tag.                                                                                |
| `username`              | Current OS user (from `whoami`)                    | Username embedded in the image tag and passed as the `USERNAME` build argument.                                   |
| `target`                | _(none)_                                           | Docker build `--target`. When set, appended to the image tag.                                                     |
| `allow_stale`           | `false`                                            | Use an existing image if the build fails, instead of returning an error.                                          |
| `force_rebuild`         | `false`                                            | Rebuild unconditionally, bypassing the staleness check.                                                           |
| `no_build_cache`        | `false`                                            | Pass `--no-cache` to `docker build`.                                                                              |
| `no_rebuild`            | `false`                                            | Skip the build entirely. Errors if no image exists yet.                                                           |
| `mountpoints`           | _(empty)_                                          | Host-to-container volume mounts. See [Mountpoints](#mountpoints).                                                 |
| `environment_variables` | _(empty)_                                          | Environment variables for the container. See [Container environment variables](#container-environment-variables). |

`force_rebuild` and `no_rebuild` are mutually exclusive.

### Mountpoints

The `mountpoints` table maps container paths to host paths. In TOML, each key
is a container path and the value is either a host path string or `false` to
remove a mountpoint inherited from a lower-priority config source.

```toml
[mountpoints]
"/workspace" = "/home/alice/projects/myproject"
"/data" = "/mnt/shared-data"
"/unwanted" = false                             # suppress a mountpoint defined in a lower-priority source
```

On the CLI, use `--mountpoint` (repeatable):

```sh
# Mount a host path into the container.
agentcontainer build --mountpoint /home/alice/projects/myproject:/workspace

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

### Image naming

The Docker image tag is derived from the resolved configuration:

```
agentcontainer-<username>-<project_name>:latest
agentcontainer-<username>-<project_name>-<target>:latest  # when target is set
```

`username`, `project_name`, and `target` are all slugified before being
embedded in the tag: lowercased, non-alphanumeric characters replaced with `-`,
consecutive dashes collapsed, and leading/trailing dashes trimmed. If the
result is empty for `username` or `project_name`, `unknown` is used as a
fallback.

### Example configuration file

```toml
dockerfile = ".agentcontainer/Dockerfile"
project_name = "myproject"
username = "alice"

[mountpoints]
"/workspace" = "/home/alice/projects/myproject"

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
AGENTCONTAINER_PROJECT_NAME="myproject"
AGENTCONTAINER_USERNAME="alice"
AGENTCONTAINER_MOUNTPOINTS='{"/workspace" = "/home/alice/projects/myproject"}'
AGENTCONTAINER_ENVIRONMENT_VARIABLES='{EDITOR = "nvim", SSH_AUTH_SOCK = true}'
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

**Note:** `agentcontainer build` must be run from the project root. The Docker
build context is always the current working directory.

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
