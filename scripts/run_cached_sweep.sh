#!/usr/bin/env bash
# five-year sweep from the local bar cache, one process per year in parallel.
# usage: scripts/run_cached_sweep.sh <tag> [extra backtest args...]
# env:   YEARS="2022 2023 2024 2025 2026" (default), DUMP_TICKS=1 to dump per-bar rows
set -uo pipefail
TAG="$1"; shift
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
YEARS="${YEARS:-2022 2023 2024 2025 2026}"
mkdir -p "$ROOT/logs/sweeps"
for y in $YEARS; do
    rm -f "$ROOT/data/${TAG}_${y}_trades.csv" "$ROOT/data/${TAG}_${y}_ticks.csv"
    start="$y-01-01"; end="$y-12-31"
    [[ "$y" == "2026" ]] && end="2026-09-10"
    "$ROOT/scripts/backtest_cached_range.sh" "$TAG" "$start" "$end" "$@" > "$ROOT/logs/sweeps/${TAG}_${y}.log" 2>&1 &
done
echo "$* " > "$ROOT/data/${TAG}.args"
wait
for y in $YEARS; do tail -1 "$ROOT/logs/sweeps/${TAG}_${y}.log"; done
