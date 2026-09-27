#!/usr/bin/env bash
# run one VWAP-short cell as a five-year IEX sweep under the shared research lock.
# usage: research/vwap_short/run_cell.sh <cell> [extra backtest args...]
#   <cell> = research/vwap_short/<cell>.json; tag vw_<cell> (plus TAG_SUFFIX); env MAX_POS (0.36).
# the session is opened by flags: --no-new-entries-after 15:30 --force-exit-by 15:55; the patch's
# `_am` windows carry the ordinary 11:30 / 11:55 clocks themselves (as in research/stress).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CELL="$1"; shift
TAG="vw_${CELL}${TAG_SUFFIX:-}"
BASE="--sizing-fraction 0.36 --max-position-pct ${MAX_POS:-0.36} --cross-index SPY --no-new-entries-after 15:30 --force-exit-by 15:55"
mkdir -p "$ROOT/logs/sweeps"
exec 9>"$ROOT/logs/.research.lock"
echo "[$TAG] waiting for logs/.research.lock ..." >&2
flock -w 14400 9 || { echo "[$TAG] lock timeout" >&2; exit 1; }
echo "[$TAG] $(date +%H:%M:%S) start" >&2
# shellcheck disable=SC2086
BARS_DIR="$ROOT/data/bars_iex" "$ROOT/scripts/run_cached_sweep.sh" "$TAG" $BASE \
    --patch-json "$ROOT/research/vwap_short/${CELL}.json" "$@"
"$ROOT/research/stress/fixup_skipped.sh" "$TAG"
echo "[$TAG] $(date +%H:%M:%S) done" >&2
flock -u 9
cd "$ROOT" && python3 research/entries/summarize.py "$TAG"
