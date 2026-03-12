#!/usr/bin/env bash
# run backtest for every trading day in a given year, output CSV
# usage: ./scripts/backtest_year.sh <year> [extra_args...]
# example: ./scripts/backtest_year.sh 2025 --add-momentum-persistence 0.05
#
# outputs a single CSV to stdout (redirect to file).
# progress is written to stderr.

set -uo pipefail

if [[ $# -lt 1 ]]; then
    echo "usage: $0 <year> [extra_args...]" >&2
    exit 1
fi

YEAR="$1"
shift
EXTRA_ARGS=("$@")

CAPITAL=10000
LOOKBACK=3
COST_ARGS="--slippage-bps 2.0 --half-spread 0.005"
BACKTEST="cargo run -p backtest --release --"

# generate all weekdays for the year
DATES=$(python3 -c "
import datetime
d = datetime.date($YEAR, 1, 1)
end = datetime.date($YEAR, 12, 31)
while d <= end:
    if d.weekday() < 5:
        print(d.isoformat())
    d += datetime.timedelta(days=1)
")

TOTAL=$(echo "$DATES" | wc -l | tr -d ' ')
COUNT=0
HEADER_PRINTED=0
ERRORS=0
CUM_PNL=0

echo "=== year backtest: $YEAR ===" >&2
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK}  costs: ${COST_ARGS}" >&2
if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    echo "overrides: ${EXTRA_ARGS[*]}" >&2
fi
echo "total trading days: $TOTAL" >&2
echo "" >&2

for date in $DATES; do
    COUNT=$(( COUNT + 1 ))

    output=$($BACKTEST --date "$date" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS --output-trades-csv ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>/dev/null) || true

    if [[ -z "$output" ]]; then
        echo "  [$COUNT/$TOTAL] $date  ERROR (no output)" >&2
        ERRORS=$(( ERRORS + 1 ))
        continue
    fi

    # print header only once (first line of first successful run)
    if [[ "$HEADER_PRINTED" -eq 0 ]]; then
        echo "$output" | head -1
        HEADER_PRINTED=1
    fi

    # print all non-header lines
    echo "$output" | tail -n +2

    # extract summary for progress
    day_pnl=$(echo "$output" | grep "^summary" | awk -F',' '{sum += $10} END {printf "%.2f", sum}')
    day_trades=$(echo "$output" | grep "^trade," | wc -l | tr -d ' ')
    CUM_PNL=$(echo "$CUM_PNL + $day_pnl" | bc -l)

    if (( $(echo "$day_pnl < 0" | bc -l) )); then
        sign=""
    else
        sign="+"
    fi

    cum_sign="+"
    if (( $(echo "$CUM_PNL < 0" | bc -l) )); then
        cum_sign=""
    fi

    printf "  [%d/%d] %s  %s\$%s  (%s trades)  cum: %s\$%.2f\n" \
        "$COUNT" "$TOTAL" "$date" "$sign" "$day_pnl" "$day_trades" "$cum_sign" "$CUM_PNL" >&2
done

echo "" >&2
echo "=== done: $TOTAL days, $ERRORS errors, cumulative P&L: \$$(printf '%.2f' "$CUM_PNL") ===" >&2
