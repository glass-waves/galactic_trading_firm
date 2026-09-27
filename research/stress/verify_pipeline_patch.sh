#!/usr/bin/env bash
# one-day parity check: the flag-based sweep form (session opened by --no-new-entries-after /
# --force-exit-by + the cell patch) vs the pipeline form (one patch carrying the `session` key).
# usage: research/stress/verify_pipeline_patch.sh <cell> <pipeline.json> <date> [<date> ...]
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CELL="$1"; PIPE="$2"; shift 2
set -a; source "$ROOT/.env"; set +a
OUT="${OUT_DIR:-/tmp}"
COMMON=(--lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv
        --bars-dir "$ROOT/data/bars_iex" --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY)
for d in "$@"; do
    "$ROOT/target/release/backtest" --date "$d" "${COMMON[@]}" --no-new-entries-after 15:30 --force-exit-by 15:55 \
        --patch-json "$ROOT/research/stress/${CELL}.json" > "$OUT/flags_$d.csv" 2> "$OUT/flags_$d.err"
    "$ROOT/target/release/backtest" --date "$d" "${COMMON[@]}" --patch-json "$PIPE" > "$OUT/pipe_$d.csv" 2> "$OUT/pipe_$d.err"
    same=$(cmp -s <(grep '^trade,' "$OUT/flags_$d.csv") <(grep '^trade,' "$OUT/pipe_$d.csv") && echo identical || echo DIFFERENT)
    echo "$d: flags $(grep -c '^trade,' "$OUT/flags_$d.csv") trades, pipeline $(grep -c '^trade,' "$OUT/pipe_$d.csv") trades -> $same"
    grep '^trade,' "$OUT/pipe_$d.csv" | cut -d, -f2-6,9,10,13,14
done
