#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

#/ Run tests.

if ! docker container inspect -f '{{.State.Running}}' agentcontainer-dev 2>/dev/null \
        | grep -q true; then
    echo "Container 'agentcontainer-dev' is not running. Starting it first..."
    bash scripts/dev_run.sh
fi

tty_flag=()
if [ -t 0 ]; then
    tty_flag=(-t)
fi

docker exec "${tty_flag[@]}" agentcontainer-dev cargo nextest run "$@"
