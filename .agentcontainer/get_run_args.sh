#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR

# Read the current hookable arguments from the file passed as `$1`, append extra
# `--volume` entries, and output the result as a TOML document with an `args`
# key.
#
# Uses `toml` (`toml-cli`) to parse the input, so any valid TOML document with
# an `args` key is accepted (single-line, multi-line, etc.). Uses `jq` to merge
# arrays. The output is a JSON-style inline array, which is valid TOML.

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

input_file="$1"

# Extract the existing args as a JSON array.
existing=$(toml get "$input_file" args)

# Build the extra entries as a JSON array, properly quoting all values.
extra=$(jq -n \
    --arg a1 "--volume" --arg v1 "${docker_sock}:/var/run/docker.sock" \
    '[$a1, $v1]')

# Merge the existing and extra arrays into a TOML document.
merged=$(jq -cn --argjson existing "$existing" --argjson extra "$extra" \
    '$existing + $extra')
echo "args = ${merged}"
