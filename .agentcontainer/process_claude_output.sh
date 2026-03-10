#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND" >&2' ERR

# Post-run hook that processes Claude Code's JSON output from `--print
# --output-format json` mode. Extracts the result text, cost, and duration.
#
# Input: file path ($1) containing raw Claude Code JSON stdout.
# Output: the extracted result text, cost, and duration.

for cmd in jq bc; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "Required tool '$cmd' is not installed." >&2
        exit 1
    fi
done

input_file="$1"

# Extract the first line from the output. Claude Code's JSON output may be
# followed by terminal escape sequences on subsequent lines.
json_line=$(head -n1 "$input_file")

# If we can't find a JSON line, or it's not Claude Code result JSON, pass
# through the original output unchanged.
if [[ -z "$json_line" ]] \
    || ! echo "$json_line" | jq -e '.type == "result"' > /dev/null 2>&1; then
    cat "$input_file"
    exit 0
fi

# Extract fields from the JSON.
result=$(jq -r '.result // ""' <<< "$json_line")
cost=$(jq -r '.total_cost_usd // 0' <<< "$json_line")
duration_ms=$(jq -r '.duration_ms // 0' <<< "$json_line")
duration=$(printf "%.3f" "$(bc <<< "scale=3; $duration_ms / 1000")")

# Round cost up to the next cent.
round_up_cents() {
    local value=$1
    local rounded
    rounded=$(bc << EOF
scale=0
tmp = $value * 100
tmp = tmp / 1
if (tmp < $value * 100) tmp = tmp + 1
scale=2
tmp / 100
EOF
    )
    printf "%.2f" "$rounded"
}

# Format duration as XhYmZ.WWWs.
format_duration() {
    local total_seconds=$1

    # Handle zero or empty input.
    if [[ -z "$total_seconds" ]] \
        || [[ $(bc <<< "$total_seconds == 0") -eq 1 ]]; then
        echo "0.000s"
        return
    fi

    local hours=$(bc <<< "$total_seconds / 3600")
    local remainder=$(bc <<< "$total_seconds - ($hours * 3600)")
    local minutes=$(bc <<< "$remainder / 60")
    local seconds=$(bc <<< "$remainder - ($minutes * 60)")

    local formatted=""
    if [ "$hours" -gt 0 ]; then
        formatted="${hours}h"
    fi
    if [ "$minutes" -gt 0 ] || [ "$hours" -gt 0 ]; then
        formatted="${formatted}${minutes}m"
    fi
    formatted="${formatted}$(printf "%.3f" "$seconds")s"
    echo "$formatted"
}

# Output the result, cost, and duration to stdout.
echo "$result"
echo ""
echo "Cost: \$$(round_up_cents "$cost")"
echo "Duration: $(format_duration "$duration")"
