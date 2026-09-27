#!/usr/bin/env bash
# re-run the days a sweep skipped for a transient reason (e.g. the bar cache being rewritten
# by another process mid-read) and append them to the tag's year CSVs.
# usage: research/stress/fixup_skipped.sh <tag>      (args are read back from data/<tag>.args)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TAG="$1"
read -r -a ARGS < "$ROOT/data/${TAG}.args"
n=0
for y in 2022 2023 2024 2025 2026; do
    log="$ROOT/logs/sweeps/${TAG}_${y}.log"
    [[ -f "$log" ]] || continue
    for d in $(grep -oE "[0-9]{4}-[0-9]{2}-[0-9]{2} skipped:" "$log" | cut -d' ' -f1 | sort -u); do
        n=$((n+1))
        echo "[$TAG] re-running $d" >&2
        BARS_DIR="$ROOT/data/bars_iex" "$ROOT/scripts/backtest_cached_range.sh" "$TAG" "$d" "$d" "${ARGS[@]}" \
            >> "$ROOT/logs/sweeps/${TAG}_${y}.fixup.log" 2>&1
    done
done
echo "[$TAG] fixup: $n day(s) re-run" >&2
