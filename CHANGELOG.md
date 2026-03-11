# Changelog

## Next

- The `config` subcommand now prints a comment header listing all configuration
  sources in priority order (lowest to highest), showing which config files
  were found on disk alongside built-in defaults, environment variables, and
  CLI arguments.
- Add `post_run` hooks for post-processing `docker run` stdout through a
  pipeline of user-configured executables. When `post_run` hooks are
  configured, `docker run` is spawned as a child process instead of replacing
  the current process via `exec`, and TTY allocation is disabled.

### Breaking

- Pre-hook (`pre_build`, `pre_run`) stderr is now inherited instead of
  captured. Hook diagnostic output streams to the terminal in real-time, but is
  no longer included in the error message when a hook exits with a non-zero
  status.

## v0.1.0

- Initial release.
