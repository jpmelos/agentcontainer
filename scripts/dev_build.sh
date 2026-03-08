#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

docker build \
    --build-arg "UID=$(id -u)" \
    --build-arg "GID=$(id -g)" \
    --build-arg "HOME=$HOME" \
    --tag agentcontainer-dev .
