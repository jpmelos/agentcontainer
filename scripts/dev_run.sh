#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

if ! docker image inspect agentcontainer-dev &> /dev/null; then
    echo "Image 'agentcontainer-dev' not found. Building it first..."
    bash scripts/dev_build.sh
fi

# Remove existing container if it's already running.
docker rm --force agentcontainer-dev &> /dev/null || true

docker run \
    --name agentcontainer-dev \
    --rm \
    --detach \
    --volume "$(pwd):$(pwd)" \
    --workdir "$(pwd)" \
    agentcontainer-dev
