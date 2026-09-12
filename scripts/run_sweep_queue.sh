#!/usr/bin/env bash
# run a queue of legs N-wide, then repair every leg's rate-limit gaps sequentially.
# usage: ./scripts/run_sweep_queue.sh <parallelism> "tag|start|end|args" ...
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${SWEEP_LOG:-$ROOT/data/sweep_progress.log}"
N="$1"; shift
"$ROOT/scripts/run_parallel_sweep.sh" "$N" "$@"
for leg in "$@"; do
    IFS='|' read -r tag start end extra <<< "$leg"
    DAY_SLEEP=2 "$ROOT/scripts/fill_sweep_gaps.sh" "$tag" "$start" "$end" >> "$LOG" 2>&1
done
echo "sweep queue finished $(date -Is)" >> "$LOG"
