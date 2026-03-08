#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR

cargo install --locked cargo-binstall@1.17.6
cargo binstall --no-confirm cargo-nextest@0.9.129 cargo-deny@0.19.0 cargo-machete@0.9.1
