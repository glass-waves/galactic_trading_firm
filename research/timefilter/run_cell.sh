#!/usr/bin/env bash
# run one time-consistent-filter cell as a five-year IEX sweep under the shared research lock.
# usage: research/timefilter/run_cell.sh <cell> [extra backtest args...]
#   <cell> = research/timefilter/<cell>.json; tag tf_<cell>. env DUMP_TICKS=1 dumps every bar.
# no session flags: the patch's own `session` key opens the day (15:30 / 15:55); the `_am`
# windows carry the promoted 11:30 / 11:55 clock themselves.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CELL="$1"; shift
TAG="tf_${CELL}"
BASE="--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY"
mkdir -p "$ROOT/logs/sweeps"
exec 9>"$ROOT/logs/.research.lock"
echo "[$TAG] waiting for logs/.research.lock ..." >&2
flock -w 14400 9 || { echo "[$TAG] lock timeout" >&2; exit 1; }
echo "[$TAG] $(date +%H:%M:%S) start" >&2
# shellcheck disable=SC2086
BARS_DIR="$ROOT/data/bars_iex" "$ROOT/scripts/run_cached_sweep.sh" "$TAG" $BASE \
    --patch-json "$ROOT/research/timefilter/${CELL}.json" "$@"
"$ROOT/research/stress/fixup_skipped.sh" "$TAG"
echo "[$TAG] $(date +%H:%M:%S) done" >&2
flock -u 9
cd "$ROOT" && python3 research/entries/summarize.py "$TAG" --by-window
