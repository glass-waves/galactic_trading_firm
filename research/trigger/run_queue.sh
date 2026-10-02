#!/usr/bin/env bash
# run several cells back to back (each takes the shared lock itself).
# usage: research/trigger/run_queue.sh <cell> [<cell> ...]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
for c in "$@"; do
    "$ROOT/research/trigger/run_cell.sh" "$c" > "$ROOT/logs/sweeps/tr_${c}_main.log" 2>&1
done
