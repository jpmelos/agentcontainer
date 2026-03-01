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

| Key          | Default                      | Description             |
| ------------ | ---------------------------- | ----------------------- |
| `dockerfile` | `.agentcontainer/Dockerfile` | Path to the Dockerfile. |

### Example configuration file

```toml
dockerfile = ".agentcontainer/Dockerfile"
```

### Environment variables

Each configuration key maps to an `AGENTCONTAINER_<KEY>` environment variable,
where `<KEY>` is the uppercase version of the key name. For example:

```sh
AGENTCONTAINER_DOCKERFILE=".agentcontainer/Dockerfile"
```

## Commands

### `config`

Print the resolved configuration, with all sources merged, as TOML.

```
agentcontainer config
```
