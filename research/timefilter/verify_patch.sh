#!/usr/bin/env bash
# parity check: replay a pipeline-format patch alone (standard research flags + --patch-json, no session
# flags) on given days and compare its trade rows with the swept cell's rows for those days.
# usage: research/timefilter/verify_patch.sh <swept tag> <patch.json> <date> [<date> ...]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TAG="$1"; PATCH="$2"; shift 2
set -a; source "$ROOT/.env"; set +a
OUT="${OUT_DIR:-$ROOT/logs/sweeps}"
COMMON=(--lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv
        --bars-dir "$ROOT/data/bars_iex" --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY)
exec 9>"$ROOT/logs/.research.lock"
flock -w 14400 9 || { echo "lock timeout" >&2; exit 1; }
for d in "$@"; do
    "$ROOT/target/release/backtest" --date "$d" "${COMMON[@]}" --patch-json "$PATCH" > "$OUT/tfv_$d.csv" 2> "$OUT/tfv_$d.err"
    a=$(grep '^trade,' "$OUT/tfv_$d.csv" | sort)
    b=$(grep "^trade,$d," "$ROOT/data/${TAG}_${d:0:4}_trades.csv" | sort)
    same=$([[ "$a" == "$b" ]] && echo identical || echo DIFFERENT)
    echo "$d: patch $(grep -c '^trade,' "$OUT/tfv_$d.csv") trades, sweep $(grep -c "^trade,$d," "$ROOT/data/${TAG}_${d:0:4}_trades.csv") -> $same"
    grep '^trade,' "$OUT/tfv_$d.csv" | cut -d, -f2-6,10,13,14
done
flock -u 9
