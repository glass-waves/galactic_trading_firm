#!/usr/bin/env bash
# run the promoted config over every weekday in [start, end], one process per day,
# writing data/<tag>_<year>_trades.csv (one file per calendar year, header once).
#
# usage: ./scripts/backtest_range.sh <tag> <YYYY-MM-DD start> <YYYY-MM-DD end> [extra backtest args...]
# env:   DAY_SLEEP (seconds between days, default 3 — keeps a single stream under
#        alpaca's 200 req/min free-plan limit), CAPITAL (10000), LOOKBACK (5),
#        COST_ARGS ("--slippage-bps 3.0 --half-spread 0.005")
#
# days with no data (holidays) are skipped. progress goes to stderr.
set -uo pipefail
if [[ $# -lt 3 ]]; then
    echo "usage: $0 <tag> <start> <end> [extra args...]" >&2; exit 1
fi
TAG="$1"; START="$2"; END="$3"; shift 3
EXTRA=("$@")
DAY_SLEEP="${DAY_SLEEP:-3}"
CAPITAL="${CAPITAL:-10000}"
LOOKBACK="${LOOKBACK:-8}"
COST_ARGS="${COST_ARGS:---slippage-bps 3.0 --half-spread 0.005}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
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
    # a day counts only if every ticker produced a summary row and nothing errored
    # (alpaca rate limits fail individual tickers; a partial day would silently bias the leg)
    if "$BIN" --date "$d" --lookback-days "$LOOKBACK" --capital "$CAPITAL" $COST_ARGS --output-trades-csv "${EXTRA[@]}" > "$tmp" 2>"$tmp.err" \
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
        echo "[$TAG] $N/$TOTAL $d skipped (no data / error)" >&2
    fi
    rm -f "$tmp" "$tmp.err"
    sleep "$DAY_SLEEP"
done
echo "[$TAG] done: $OK days, $SKIP skipped" >&2
