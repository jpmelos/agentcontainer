#!/usr/bin/env bash
set -euo pipefail
trap 'echo "Exit status $? at line $LINENO from: $BASH_COMMAND"' ERR
cd "$(dirname "${BASH_SOURCE[0]}")/.."

LINE_LIMIT=1000

if [ $# -eq 0 ]; then
    while IFS= read -r -d '' file; do
        set -- "$@" "$file"
    done < <(find . \( -name "*.rs" -o -name "*.sh" \) -print0)
fi

for file in "$@"; do
    lines=$(wc -l < "$file" | tr -d ' ')
    if [ "$lines" -gt "$LINE_LIMIT" ]; then
        echo "Error: $file has $lines lines, exceeding the $LINE_LIMIT line limit"
        exit 1
    fi
done
