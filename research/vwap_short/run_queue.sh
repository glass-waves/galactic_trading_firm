#!/usr/bin/env bash
# run several cells back to back, one at a time (each takes the shared lock itself).
# usage: research/vwap_short/run_queue.sh <cell> [<cell> ...]   env MAX_POS, TAG_SUFFIX, EXTRA pass through
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
for c in "$@"; do
    # shellcheck disable=SC2086
    "$ROOT/research/vwap_short/run_cell.sh" "$c" ${EXTRA:-} > "$ROOT/logs/sweeps/vw_${c}${TAG_SUFFIX:-}_main.log" 2>&1
done
