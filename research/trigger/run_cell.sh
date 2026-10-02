#!/usr/bin/env bash
# run one entry-trigger cell as a five-year IEX sweep under the shared research lock.
# usage: research/trigger/run_cell.sh <cell> [extra backtest args...]
#   <cell> = research/trigger/<cell>.json; tag tr_<cell>. env DUMP_TICKS=1 dumps the morning bars.
# no session flags: every window keeps the promoted 11:30 / 11:55 clock (the `_am` scaffold carries
# it explicitly, as in research/stress and research/vwap_short).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CELL="$1"; shift
TAG="tr_${CELL}"
BASE="--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY"
mkdir -p "$ROOT/logs/sweeps"
exec 9>"$ROOT/logs/.research.lock"
echo "[$TAG] waiting for logs/.research.lock ..." >&2
flock -w 14400 9 || { echo "[$TAG] lock timeout" >&2; exit 1; }
echo "[$TAG] $(date +%H:%M:%S) start" >&2
# shellcheck disable=SC2086
BARS_DIR="$ROOT/data/bars_iex" "$ROOT/scripts/run_cached_sweep.sh" "$TAG" $BASE \
    --patch-json "$ROOT/research/trigger/${CELL}.json" "$@"
"$ROOT/research/stress/fixup_skipped.sh" "$TAG"
echo "[$TAG] $(date +%H:%M:%S) done" >&2
flock -u 9
cd "$ROOT" && python3 research/entries/summarize.py "$TAG" --by-window
