#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR

# Output extra `docker run` arguments as a TOML array to be used to run the
# container that will be used by `agentcontainer`.

# Detect the Docker socket path on the host.
if [[ -S "${HOME}/.docker/run/docker.sock" ]]; then
    # macOS Docker Desktop.
    docker_sock="${HOME}/.docker/run/docker.sock"
elif [[ -S "/var/run/docker.sock" ]]; then
    # Linux default.
    docker_sock="/var/run/docker.sock"
else
    echo "Could not find Docker socket." >&2
    exit 1
fi

cat << EOF
[
    "--volume", "${docker_sock}:/var/run/docker.sock"
]
EOF
