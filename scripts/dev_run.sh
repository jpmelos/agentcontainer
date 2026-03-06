#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND"' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

if ! docker image inspect agentcontainer-dev &>/dev/null; then
    echo "Image 'agentcontainer-dev' not found. Building it first..."
    bash scripts/dev_build.sh
fi

# Remove existing container if it's already running.
docker rm -f agentcontainer-dev &>/dev/null || true

docker run --name agentcontainer-dev --rm -d -v "$(pwd):$(pwd)" -w "$(pwd)" agentcontainer-dev
