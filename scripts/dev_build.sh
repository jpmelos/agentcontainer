#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND"' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

docker build --build-arg "USERNAME=$(whoami)" --tag agentcontainer-dev .
