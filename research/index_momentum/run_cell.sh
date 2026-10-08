#!/usr/bin/env bash
# run one index-momentum cell as a five-year IEX sweep under the shared research lock.
# usage: research/index_momentum/run_cell.sh <cell> [extra backtest args...]
#   <cell> = research/index_momentum/<cell>.json; tag im_<cell> (env TAG overrides).
#   env LOOKBACK (calendar days of warmup, default 22: the 14-session noise area needs ~20),
#   COST_ARGS (default 3 bps + $0.005/share per leg) pass through to backtest_cached_range.sh
#   and to the skipped-day fixup.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CELL="$1"; shift
TAG="${TAG:-im_${CELL}}"
export LOOKBACK="${LOOKBACK:-22}"
[[ -n "${COST_ARGS:-}" ]] && export COST_ARGS
BASE="--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY"
mkdir -p "$ROOT/logs/sweeps"
exec 9>"$ROOT/logs/.research.lock"
echo "[$TAG] waiting for logs/.research.lock ..." >&2
flock -w 14400 9 || { echo "[$TAG] lock timeout" >&2; exit 1; }
echo "[$TAG] $(date +%H:%M:%S) start (lookback $LOOKBACK d, costs ${COST_ARGS:-default})" >&2
# shellcheck disable=SC2086
BARS_DIR="$ROOT/data/bars_iex" "$ROOT/scripts/run_cached_sweep.sh" "$TAG" $BASE \
    --patch-json "$ROOT/research/index_momentum/${CELL}.json" "$@"
"$ROOT/research/stress/fixup_skipped.sh" "$TAG"
echo "[$TAG] $(date +%H:%M:%S) done" >&2
flock -u 9
