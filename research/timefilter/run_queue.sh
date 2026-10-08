#!/usr/bin/env bash
# run several cells back to back (each takes the shared lock itself).
# usage: research/timefilter/run_queue.sh <cell> [<cell> ...]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
for c in "$@"; do
    "$ROOT/research/timefilter/run_cell.sh" "$c" > "$ROOT/logs/sweeps/tf_${c}_main.log" 2>&1
done
