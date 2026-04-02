#!/usr/bin/env bash
# run backtest for every trading day in a given year, output CSV
# usage: ./scripts/backtest_year.sh <year> [extra_args...]
# example: ./scripts/backtest_year.sh 2025 --add-momentum-persistence 0.05
#
# outputs a single CSV to stdout (redirect to file).
# progress is written to stderr.
#
# when --compound is passed (in extra_args), each day's ending capital
# carries over to the next day. without it, each day starts at $CAPITAL.

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

# check for --compound flag and --capital override, remove from extra args
COMPOUND=false
FILTERED_ARGS=()
SKIP_NEXT=false
for i in "${!EXTRA_ARGS[@]}"; do
    if $SKIP_NEXT; then
        SKIP_NEXT=false
        continue
    fi
    arg="${EXTRA_ARGS[$i]}"
    if [[ "$arg" == "--compound" ]]; then
        COMPOUND=true
    elif [[ "$arg" == "--capital" ]]; then
        # consume --capital and its value, use as starting capital
        next_i=$((i + 1))
        if [[ $next_i -lt ${#EXTRA_ARGS[@]} ]]; then
            CAPITAL="${EXTRA_ARGS[$next_i]}"
            SKIP_NEXT=true
        fi
    else
        FILTERED_ARGS+=("$arg")
    fi
done
EXTRA_ARGS=("${FILTERED_ARGS[@]+"${FILTERED_ARGS[@]}"}")

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
CURRENT_CAPITAL=$CAPITAL

echo "=== year backtest: $YEAR ===" >&2
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK}  costs: ${COST_ARGS}  compound: ${COMPOUND}" >&2
if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    echo "overrides: ${EXTRA_ARGS[*]}" >&2
fi
echo "total trading days: $TOTAL" >&2
echo "" >&2

for date in $DATES; do
    COUNT=$(( COUNT + 1 ))

    # use compounded capital or fixed capital
    if $COMPOUND; then
        DAY_CAPITAL=$(printf '%.0f' "$CURRENT_CAPITAL")
    else
        DAY_CAPITAL=$CAPITAL
    fi

    output=$($BACKTEST --date "$date" --capital "$DAY_CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS --output-trades-csv ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>/dev/null) || true

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

    # update compounding capital
    if $COMPOUND; then
        CURRENT_CAPITAL=$(echo "$CURRENT_CAPITAL + $day_pnl" | bc -l)
        # floor at 10% of initial to prevent total wipeout
        MIN_CAPITAL=$(echo "$CAPITAL * 0.10" | bc -l)
        if (( $(echo "$CURRENT_CAPITAL < $MIN_CAPITAL" | bc -l) )); then
            CURRENT_CAPITAL=$MIN_CAPITAL
        fi
    fi

    if (( $(echo "$day_pnl < 0" | bc -l) )); then
        sign=""
    else
        sign="+"
    fi

    cum_sign="+"
    if (( $(echo "$CUM_PNL < 0" | bc -l) )); then
        cum_sign=""
    fi

    if $COMPOUND; then
        printf "  [%d/%d] %s  %s\$%s  (%s trades)  cum: %s\$%.2f  capital: \$%.0f\n" \
            "$COUNT" "$TOTAL" "$date" "$sign" "$day_pnl" "$day_trades" "$cum_sign" "$CUM_PNL" "$CURRENT_CAPITAL" >&2
    else
        printf "  [%d/%d] %s  %s\$%s  (%s trades)  cum: %s\$%.2f\n" \
            "$COUNT" "$TOTAL" "$date" "$sign" "$day_pnl" "$day_trades" "$cum_sign" "$CUM_PNL" >&2
    fi
done

echo "" >&2
if $COMPOUND; then
    echo "=== done: $TOTAL days, $ERRORS errors, cumulative P&L: \$$(printf '%.2f' "$CUM_PNL"), final capital: \$$(printf '%.0f' "$CURRENT_CAPITAL") ===" >&2
else
    echo "=== done: $TOTAL days, $ERRORS errors, cumulative P&L: \$$(printf '%.2f' "$CUM_PNL") ===" >&2
fi
