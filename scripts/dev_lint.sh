#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

#/ Lint the project.

if ! docker container inspect -f '{{.State.Running}}' agentcontainer-dev 2> /dev/null \
    | grep -q true; then
    echo "Container 'agentcontainer-dev' is not running. Starting it first..."
    bash scripts/dev_run.sh
fi

# If `-n` is present, just pass through.
# If `--all-files` or `--files` is present, just pass through.
# Otherwise, add `--files` with modified files if any, or add `--all-files` if
# there are no modified files.
has_flag_n=false
has_flag_all_files=false
has_flag_files=false
for arg in "$@"; do
    case "${arg}" in
        -n) has_flag_n=true ;;
        --all-files) has_flag_all_files=true ;;
        --files) has_flag_files=true ;;
    esac
done

extra_args=()
if [[ "${has_flag_n}" == false ]] \
    && [[ "${has_flag_all_files}" == false ]] \
    && [[ "${has_flag_files}" == false ]]; then
    mapfile -t -d '' modified_files < <(
        git status --porcelain -z | while IFS= read -r -d '' entry; do
            # Each entry starts with a two-character status and a space. For
            # renames/copies (R/C), a second NUL-delimited field follows with
            # the destination path; skip the source path.
            status="${entry:0:2}"
            path="${entry:3}"
            if [[ "${status}" == "D " ]] || [[ "${status}" == " D" ]]; then
                # Skip deleted files.
                continue
            elif [[ "${status}" == R* ]] || [[ "${status}" == C* ]]; then
                # Consume the destination (new) path and emit it.
                IFS= read -r -d '' dest
                printf '%s\0' "${dest}"
            else
                printf '%s\0' "${path}"
            fi
        done
    )
    if [[ ${#modified_files[@]} -gt 0 ]]; then
        extra_args=(--files "${modified_files[@]}")
    else
        extra_args=(--all-files)
    fi
fi

echo "Running pre-commit run" "$@" "${extra_args[@]}"

tty_flag=()
if [ -t 0 ]; then
    tty_flag=(-t)
fi

docker exec "${tty_flag[@]}" agentcontainer-dev pre-commit run "$@" "${extra_args[@]}"
