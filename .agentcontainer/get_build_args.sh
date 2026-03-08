#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR

# Output extra `docker build` arguments as a TOML array to be used to build the
# image that will be used by `agentcontainer`.
cat << EOF
[
    "--build-arg", "UID=$(id -u)",
    "--build-arg", "GID=$(id -g)",
    "--build-arg", "BUILD_DATE=$(date "+%Y-%m-%d")"
]
EOF
