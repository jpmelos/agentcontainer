#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR

# Read the current hookable arguments from the file passed as `$1`, append extra
# `--build-arg` entries, and output the result as a TOML document with an `args`
# key.

# Uses `toml` (`toml-cli`) to parse the input, so any valid TOML document with
# an `args` key is accepted (single-line, multi-line, etc.). Uses `jq` to merge
# arrays. The output is a JSON-style inline array, which is valid TOML.
for cmd in toml jq; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "Required tool '$cmd' is not installed." >&2
        exit 1
    fi
done

input_file="$1"

# Extract the existing args as a JSON array.
existing=$(toml get "$input_file" args)

# Build the extra entries as a JSON array, properly quoting all values.
extra=$(jq -n \
    --arg a1 "--build-arg" --arg v1 "UID=$(id -u)" \
    --arg a2 "--build-arg" --arg v2 "GID=$(id -g)" \
    --arg a3 "--build-arg" --arg v3 "BUILD_DATE=$(date "+%Y-%m-%d")" \
    '[$a1, $v1, $a2, $v2, $a3, $v3]')

# Merge the existing and extra arrays into a TOML document.
merged=$(jq -cn --argjson existing "$existing" --argjson extra "$extra" \
    '$existing + $extra')
echo "args = ${merged}"
