#!/usr/bin/env bash
# like backtest_range.sh but replays from the local bar cache (data/bars, built with
# `backtest --fetch-bars`). no alpaca calls, no rate limits, safe to run many in parallel.
#
# usage: scripts/backtest_cached_range.sh <tag> <start> <end> [extra backtest args...]
# env:   DUMP_TICKS=1  -> also writes data/<tag>_<year>_ticks.csv (per-bar diagnostics)
#        CAPITAL, LOOKBACK, COST_ARGS, BARS_DIR as in backtest_range.sh
set -uo pipefail
if [[ $# -lt 3 ]]; then
    echo "usage: $0 <tag> <start> <end> [extra args...]" >&2; exit 1
fi
TAG="$1"; START="$2"; END="$3"; shift 3
EXTRA=("$@")
CAPITAL="${CAPITAL:-10000}"
LOOKBACK="${LOOKBACK:-8}"
COST_ARGS="${COST_ARGS:---slippage-bps 3.0 --half-spread 0.005}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BARS_DIR="${BARS_DIR:-$ROOT/data/bars}"
BIN="$ROOT/target/release/backtest"
set -a; source "$ROOT/.env"; set +a
mkdir -p "$ROOT/data"
DATES=$(python3 -c "
import datetime
s=datetime.date.fromisoformat('$START'); e=datetime.date.fromisoformat('$END')
d=s
while d<=e:
    if d.weekday()<5: print(d.isoformat())
    d+=datetime.timedelta(days=1)")
TOTAL=$(echo "$DATES" | wc -l | tr -d ' ')
N=0; OK=0; SKIP=0
declare -A HEADER_DONE
for d in $DATES; do
    N=$((N+1))
    year="${d:0:4}"
    out="$ROOT/data/${TAG}_${year}_trades.csv"
    tmp=$(mktemp)
    DUMP_ARGS=()
    if [[ "${DUMP_TICKS:-0}" == "1" ]]; then
        DUMP_ARGS=(--dump-ticks "$ROOT/data/${TAG}_${year}_ticks.csv")
    fi
    if "$BIN" --date "$d" --lookback-days "$LOOKBACK" --capital "$CAPITAL" $COST_ARGS --output-trades-csv \
         --bars-dir "$BARS_DIR" "${DUMP_ARGS[@]}" "${EXTRA[@]}" > "$tmp" 2>"$tmp.err" \
       && grep -q "^summary," "$tmp" \
       && ! grep -qiE "error|failed" "$tmp.err" "$tmp"; then
        if [[ -z "${HEADER_DONE[$year]:-}" ]]; then
            if [[ ! -s "$out" ]]; then head -1 "$tmp" > "$out"; fi
            HEADER_DONE[$year]=1
        fi
        grep -vE "^row_type," "$tmp" >> "$out"
        OK=$((OK+1))
        pnl=$(grep "^summary," "$tmp" | awk -F, '{s+=$10} END {printf "%.2f", s}')
        trades=$(grep -c "^trade," "$tmp")
        echo "[$TAG] $N/$TOTAL $d pnl=$pnl trades=$trades" >&2
    else
        SKIP=$((SKIP+1))
        # holidays produce "no data" for every ticker: not an error, just skipped
        if ! grep -q "no data" "$tmp.err"; then
            echo "[$TAG] $N/$TOTAL $d skipped: $(grep -iE 'error|failed' "$tmp.err" "$tmp" | head -1)" >&2
        fi
    fi
    rm -f "$tmp" "$tmp.err"
done
echo "[$TAG] done: $OK days, $SKIP skipped" >&2
