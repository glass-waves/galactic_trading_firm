#!/usr/bin/env bash
# run several backtest_range.sh legs concurrently. each leg is "tag|start|end|extra args".
# usage: ./scripts/run_parallel_sweep.sh <parallelism> "tag|start|end|args" ["tag|start|end|args" ...]
# alpaca budget: 5 legs in parallel produced partial-ticker failures on most days;
# 3 legs with DAY_SLEEP=2 is the safe ceiling on the free plan. progress: data/sweep_progress.log
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
N="$1"; shift
LOG="${SWEEP_LOG:-$ROOT/data/sweep_progress.log}"
export DAY_SLEEP="${DAY_SLEEP:-2}"
echo "parallel sweep ($N at a time) started $(date -Is): $#" >> "$LOG"
running=0
for leg in "$@"; do
    IFS='|' read -r tag start end extra <<< "$leg"
    printf '%s\n' "$extra" > "$ROOT/data/$tag.args"   # for fill_sweep_gaps.sh
    # shellcheck disable=SC2086
    "$ROOT/scripts/backtest_range.sh" "$tag" "$start" "$end" $extra >> "$LOG" 2>&1 &
    running=$((running+1))
    if [[ $running -ge $N ]]; then wait -n; running=$((running-1)); fi
done
wait
echo "parallel sweep finished $(date -Is)" >> "$LOG"
