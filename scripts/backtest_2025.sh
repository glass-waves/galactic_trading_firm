#!/usr/bin/env bash
# run backtest for every US market open day in 2025, collect per-day total P&L,
# and print a summary with average daily P&L at the end.
#
# usage: ./scripts/backtest_2025.sh [--capital 100000] [--lookback-days 5] [--slippage-bps 2.0] [--half-spread 0.005]
#
# requires: cargo built backtest binary, DATABASE_URL, APCA_API_KEY_ID, APCA_API_SECRET_KEY

set -euo pipefail

CAPITAL="${CAPITAL:-100000}"
LOOKBACK="${LOOKBACK:-5}"
COST_ARGS=""

# parse optional args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --capital) CAPITAL="$2"; shift 2 ;;
        --lookback-days) LOOKBACK="$2"; shift 2 ;;
        --slippage-bps) COST_ARGS="$COST_ARGS --slippage-bps $2"; shift 2 ;;
        --half-spread) COST_ARGS="$COST_ARGS --half-spread $2"; shift 2 ;;
        --commission-per-share) COST_ARGS="$COST_ARGS --commission-per-share $2"; shift 2 ;;
        --sec-fee-per-million) COST_ARGS="$COST_ARGS --sec-fee-per-million $2"; shift 2 ;;
        --finra-taf-per-share) COST_ARGS="$COST_ARGS --finra-taf-per-share $2"; shift 2 ;;
        *) echo "unknown arg: $1"; exit 1 ;;
    esac
done

# 2025 US market holidays (NYSE/NASDAQ closed)
HOLIDAYS="
2025-01-01
2025-01-09
2025-01-20
2025-02-17
2025-04-18
2025-05-26
2025-06-19
2025-07-04
2025-09-01
2025-11-27
2025-12-25
"

BACKTEST_BIN="cargo run -p backtest --"
RESULTS_FILE=$(mktemp)
trap 'rm -f "$RESULTS_FILE"' EXIT

echo "backtest sweep: all 2025 market days"
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days"
if [[ -n "$COST_ARGS" ]]; then
    echo "costs:$COST_ARGS"
fi
echo "---"

total_pnl=0
total_trades=0
day_count=0
win_days=0
loss_days=0

# iterate every day in 2025
current="2025-01-01"
end="2025-12-31"

while [[ "$current" < "$end" ]] || [[ "$current" == "$end" ]]; do
    # get day of week (1=Mon..7=Sun on macOS/BSD date)
    dow=$(date -j -f "%Y-%m-%d" "$current" "+%u" 2>/dev/null)

    # skip weekends
    if [[ "$dow" -ge 6 ]]; then
        current=$(date -j -v+1d -f "%Y-%m-%d" "$current" "+%Y-%m-%d" 2>/dev/null)
        continue
    fi

    # skip holidays
    if echo "$HOLIDAYS" | grep -qw "$current"; then
        current=$(date -j -v+1d -f "%Y-%m-%d" "$current" "+%Y-%m-%d" 2>/dev/null)
        continue
    fi

    # run backtest, capture output
    output=$($BACKTEST_BIN --date "$current" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS 2>&1) || true

    # extract the "total" line: e.g. "  total  +$123.45  profit  (7 trades)"
    total_line=$(echo "$output" | grep -E "^\s+total\s" || true)

    if [[ -z "$total_line" ]]; then
        printf "  %s  no result\n" "$current"
        current=$(date -j -v+1d -f "%Y-%m-%d" "$current" "+%Y-%m-%d" 2>/dev/null)
        continue
    fi

    # parse P&L amount (handle +$X.XX and -$X.XX)
    day_pnl=$(echo "$total_line" | sed -E 's/.*[+-]\$([0-9.]+).*/\1/')
    day_sign=$(echo "$total_line" | sed -E 's/.*([+-])\$.*/\1/')
    if [[ "$day_sign" == "-" ]]; then
        day_pnl="-$day_pnl"
    fi

    # parse trade count
    day_trades=$(echo "$total_line" | sed -E 's/.*\(([0-9]+) trades\).*/\1/')

    # accumulate
    total_pnl=$(echo "$total_pnl + $day_pnl" | bc -l)
    total_trades=$(( total_trades + day_trades ))
    day_count=$(( day_count + 1 ))

    if (( $(echo "$day_pnl > 0" | bc -l) )); then
        win_days=$(( win_days + 1 ))
    elif (( $(echo "$day_pnl < 0" | bc -l) )); then
        loss_days=$(( loss_days + 1 ))
    fi

    # log to results file
    echo "$current $day_pnl $day_trades" >> "$RESULTS_FILE"

    # print progress
    printf "  %s  %s\$%s  (%s trades)\n" "$current" "$day_sign" "$(echo "$day_pnl" | tr -d '-')" "$day_trades"

    current=$(date -j -v+1d -f "%Y-%m-%d" "$current" "+%Y-%m-%d" 2>/dev/null)
done

echo ""
echo "=== 2025 backtest summary ==="
echo "trading days:   $day_count"
echo "total trades:   $total_trades"

if [[ "$day_count" -gt 0 ]]; then
    avg_pnl=$(echo "scale=2; $total_pnl / $day_count" | bc -l)
    avg_trades=$(echo "scale=1; $total_trades / $day_count" | bc -l)
    flat_days=$(( day_count - win_days - loss_days ))

    sign="+"
    if (( $(echo "$total_pnl < 0" | bc -l) )); then sign="-"; fi
    avg_sign="+"
    if (( $(echo "$avg_pnl < 0" | bc -l) )); then avg_sign="-"; fi

    printf "total P&L:      %s\$%.2f\n" "$sign" "$(echo "$total_pnl" | tr -d '-')"
    printf "avg daily P&L:  %s\$%.2f\n" "$avg_sign" "$(echo "$avg_pnl" | tr -d '-')"
    printf "avg trades/day: %s\n" "$avg_trades"
    printf "win days:       %d  loss days: %d  flat days: %d\n" "$win_days" "$loss_days" "$flat_days"
    if [[ "$day_count" -gt 0 ]]; then
        win_pct=$(echo "scale=1; $win_days * 100 / $day_count" | bc -l)
        printf "win day rate:   %s%%\n" "$win_pct"
    fi
fi
