#!/usr/bin/env zsh
# comprehensive strategy validation with hold-out OOS test, slippage sweep,
# monte carlo shuffle, walk-forward stability, and per-ticker analysis.
#
# usage: ./scripts/validate_strategy.sh [--quick] [backtest overrides...]
#   --quick  skip slippage sweep, reduce monte carlo to 100 iterations (~14 min)
#   all other args forwarded to backtest binary

set -euo pipefail

# ─── configuration ───────────────────────────────────────────────────────────

CAPITAL=10000
LOOKBACK=3
COST_ARGS="--slippage-bps 2.0 --half-spread 0.005"
BACKTEST=(cargo run -p backtest --release --)

# in-sample: 100-day set (Mon/Wed, same as backtest_100days.sh)
IS_DATES=(
    2025-03-10 2025-03-12 2025-03-17 2025-03-19 2025-03-24 2025-03-26
    2025-03-31 2025-04-02 2025-04-07 2025-04-09 2025-04-14 2025-04-16
    2025-04-21 2025-04-23 2025-04-28 2025-04-30 2025-05-05 2025-05-07
    2025-05-12 2025-05-14 2025-05-19 2025-05-21 2025-05-28 2025-06-02
    2025-06-04 2025-06-09 2025-06-11 2025-06-16 2025-06-18 2025-06-23
    2025-06-25 2025-06-30 2025-07-02 2025-07-07 2025-07-09 2025-07-14
    2025-07-16 2025-07-21 2025-07-23 2025-07-28 2025-07-30 2025-08-04
    2025-08-06 2025-08-11 2025-08-13 2025-08-18 2025-08-20 2025-08-25
    2025-08-27 2025-09-03 2025-09-08 2025-09-10 2025-09-15 2025-09-17
    2025-09-22 2025-09-24 2025-09-29 2025-10-01 2025-10-06 2025-10-08
    2025-10-13 2025-10-15 2025-10-20 2025-10-22 2025-10-27 2025-10-29
    2025-11-03 2025-11-05 2025-11-10 2025-11-12 2025-11-17 2025-11-19
    2025-11-24 2025-11-26 2025-12-01 2025-12-03 2025-12-08 2025-12-10
    2025-12-15 2025-12-17 2025-12-22 2025-12-24 2025-12-29 2025-12-31
    2026-01-05 2026-01-07 2026-01-12 2026-01-14 2026-01-21 2026-01-26
    2026-01-28 2026-02-02 2026-02-04 2026-02-09 2026-02-11 2026-02-18
    2026-02-23 2026-02-25 2026-03-02
)

# hold-out: ~40 Thursday dates NEVER used in any optimization script
HOLDOUT_DATES=(
    2025-03-13 2025-03-20 2025-03-27 2025-04-03 2025-04-10
    2025-04-17 2025-04-24 2025-05-01 2025-05-08 2025-05-15
    2025-05-22 2025-05-29 2025-06-05 2025-06-12 2025-06-19
    2025-06-26 2025-07-10 2025-07-17 2025-07-24 2025-07-31
    2025-08-07 2025-08-14 2025-08-21 2025-08-28 2025-09-04
    2025-09-11 2025-09-18 2025-09-25 2025-10-02 2025-10-09
    2025-10-16 2025-10-23 2025-10-30 2025-11-06 2025-11-13
    2025-11-20 2025-12-04 2025-12-11 2025-12-18 2026-01-08
)

# 20-day subset for slippage sweep (fast)
SWEEP_DATES=(
    2025-03-17 2025-04-07 2025-04-28 2025-05-19 2025-06-09
    2025-06-30 2025-07-21 2025-08-11 2025-08-25 2025-09-15
    2025-10-06 2025-10-27 2025-11-17 2025-12-08 2025-12-29
    2026-01-12 2026-01-27 2026-02-10 2026-02-24 2026-03-02
)

# ─── parse arguments ─────────────────────────────────────────────────────────

QUICK=false
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick) QUICK=true; shift ;;
        *) EXTRA_ARGS+=("$1"); shift ;;
    esac
done

# ─── helpers ─────────────────────────────────────────────────────────────────

# run a single date, extract total P&L and per-ticker lines
# sets: DAY_PNL, DAY_TRADES, TICKER_LINES[]
run_date() {
    local date="$1"
    shift
    local extra_cost_args="${*:-}"

    local output
    output=$("${BACKTEST[@]}" --date "$date" --capital "$CAPITAL" --lookback-days "$LOOKBACK" \
        ${=COST_ARGS} ${=extra_cost_args} ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1) || true

    # check for rate limiting / data quality issues
    if echo "$output" | grep -q "rate limited"; then
        echo "  WARNING: rate limit retry on $date" >&2
    fi
    if echo "$output" | grep -q "WARNING.*candles"; then
        echo "  WARNING: low candle count on $date — results may be unreliable" >&2
    fi

    # parse total line
    local total_line
    total_line=$(echo "$output" | grep -E '^\s+total\s' || true)

    if [[ -z "$total_line" ]]; then
        DAY_PNL=""
        DAY_TRADES=0
        TICKER_LINES=()
        return
    fi

    local raw_pnl raw_sign
    raw_pnl=$(echo "$total_line" | sed -E 's/.*[+-]\$([0-9.]+).*/\1/')
    raw_sign=$(echo "$total_line" | sed -E 's/.*([+-])\$.*/\1/')
    if [[ "$raw_sign" == "-" ]]; then
        DAY_PNL="-$raw_pnl"
    else
        DAY_PNL="$raw_pnl"
    fi
    DAY_TRADES=$(echo "$total_line" | sed -E 's/.*\(([0-9]+) trades\).*/\1/')

    # parse per-ticker lines (e.g. "  SPY    +$234.50  (12 trades)")
    TICKER_LINES=()
    while IFS= read -r line; do
        TICKER_LINES+=("$line")
    done < <(echo "$output" | grep -E '^\s+[A-Z]{2,5}\s+[+-]\$' || true)
}

# parse a ticker line into TICK_NAME, TICK_PNL, TICK_TRADES
parse_ticker_line() {
    local line="$1"
    TICK_NAME=$(echo "$line" | awk '{print $1}')
    local raw_pnl raw_sign
    raw_pnl=$(echo "$line" | sed -E 's/.*[+-]\$([0-9.]+).*/\1/')
    raw_sign=$(echo "$line" | sed -E 's/.*([+-])\$.*/\1/')
    if [[ "$raw_sign" == "-" ]]; then
        TICK_PNL="-$raw_pnl"
    else
        TICK_PNL="$raw_pnl"
    fi
    TICK_TRADES=$(echo "$line" | sed -E 's/.*\(([0-9]+) trades\).*/\1/')
}

fmt_pnl() {
    local val="$1"
    if (( $(echo "$val >= 0" | bc -l) )); then
        printf "+\$%.2f" "$val"
    else
        local abs
        abs=$(echo "$val" | tr -d '-')
        printf -- "-\$%.2f" "$abs"
    fi
}

fmt_pct() {
    local val="$1"
    printf "%.0f%%" "$val"
}

# ─── section 1: in-sample baseline ──────────────────────────────────────────

echo "=== comprehensive strategy validation ==="
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK}  costs: ${COST_ARGS}"
if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    echo "overrides: ${EXTRA_ARGS[*]}"
fi
echo ""

echo "--- section 1: in-sample baseline (${#IS_DATES[@]} dates) ---"

IS_DAILY_PNL=()
IS_DAILY_DATES=()
is_total_pnl=0
is_total_trades=0
is_win_days=0
is_day_count=0
is_gross_wins=0
is_gross_losses=0

# per-ticker accumulators (associative arrays)
declare -A IS_TICKER_PNL
declare -A IS_TICKER_TRADES
declare -A IS_TICKER_WIN_PNL
declare -A IS_TICKER_LOSS_PNL
declare -A IS_TICKER_WIN_DAYS

for date in "${IS_DATES[@]}"; do
    run_date "$date"

    if [[ -z "$DAY_PNL" ]]; then
        printf "  %s  (no data)\n" "$date"
        continue
    fi

    IS_DAILY_PNL+=("$DAY_PNL")
    IS_DAILY_DATES+=("$date")
    is_total_pnl=$(echo "$is_total_pnl + $DAY_PNL" | bc -l)
    is_total_trades=$(( is_total_trades + DAY_TRADES ))
    is_day_count=$(( is_day_count + 1 ))

    if (( $(echo "$DAY_PNL > 0.001" | bc -l) )); then
        is_win_days=$(( is_win_days + 1 ))
        is_gross_wins=$(echo "$is_gross_wins + $DAY_PNL" | bc -l)
    elif (( $(echo "$DAY_PNL < -0.001" | bc -l) )); then
        local_abs=$(echo "$DAY_PNL" | tr -d '-')
        is_gross_losses=$(echo "$is_gross_losses + $local_abs" | bc -l)
    fi

    # accumulate per-ticker
    for tline in "${TICKER_LINES[@]}"; do
        parse_ticker_line "$tline"
        IS_TICKER_PNL[$TICK_NAME]=$(echo "${IS_TICKER_PNL[$TICK_NAME]:-0} + $TICK_PNL" | bc -l)
        IS_TICKER_TRADES[$TICK_NAME]=$(( ${IS_TICKER_TRADES[$TICK_NAME]:-0} + TICK_TRADES ))
        if (( $(echo "$TICK_PNL > 0.001" | bc -l) )); then
            IS_TICKER_WIN_PNL[$TICK_NAME]=$(echo "${IS_TICKER_WIN_PNL[$TICK_NAME]:-0} + $TICK_PNL" | bc -l)
            IS_TICKER_WIN_DAYS[$TICK_NAME]=$(( ${IS_TICKER_WIN_DAYS[$TICK_NAME]:-0} + 1 ))
        elif (( $(echo "$TICK_PNL < -0.001" | bc -l) )); then
            local_abs=$(echo "$TICK_PNL" | tr -d '-')
            IS_TICKER_LOSS_PNL[$TICK_NAME]=$(echo "${IS_TICKER_LOSS_PNL[$TICK_NAME]:-0} + $local_abs" | bc -l)
        fi
    done

    local_sign="+"
    if (( $(echo "$DAY_PNL < 0" | bc -l) )); then local_sign="-"; fi
    printf "  %s  %s\$%-10s  %2d trades\n" "$date" "$local_sign" "$(echo "$DAY_PNL" | tr -d '-')" "$DAY_TRADES"
done

# compute IS aggregates
is_win_rate=0
if [[ "$is_day_count" -gt 0 ]]; then
    is_win_rate=$(echo "scale=1; $is_win_days * 100 / $is_day_count" | bc -l)
fi
is_pf="n/a"
if (( $(echo "$is_gross_losses > 0" | bc -l) )); then
    is_pf=$(echo "scale=2; $is_gross_wins / $is_gross_losses" | bc -l)
elif (( $(echo "$is_gross_wins > 0" | bc -l) )); then
    is_pf="inf"
fi

echo ""
printf "  in-sample: P&L %s  PF %s  win %s  (%d days, %d trades)\n" \
    "$(fmt_pnl "$is_total_pnl")" "$is_pf" "$(fmt_pct "$is_win_rate")" "$is_day_count" "$is_total_trades"
echo ""

# ─── section 2: hold-out OOS test ───────────────────────────────────────────

echo "--- section 2: hold-out out-of-sample test (${#HOLDOUT_DATES[@]} dates, never used in optimization) ---"

OOS_DAILY_PNL=()
OOS_DAILY_DATES=()
oos_total_pnl=0
oos_total_trades=0
oos_win_days=0
oos_day_count=0
oos_gross_wins=0
oos_gross_losses=0

declare -A OOS_TICKER_PNL
declare -A OOS_TICKER_TRADES
declare -A OOS_TICKER_WIN_PNL
declare -A OOS_TICKER_LOSS_PNL
declare -A OOS_TICKER_WIN_DAYS

for date in "${HOLDOUT_DATES[@]}"; do
    run_date "$date"

    if [[ -z "$DAY_PNL" ]]; then
        printf "  %s  (no data)\n" "$date"
        continue
    fi

    OOS_DAILY_PNL+=("$DAY_PNL")
    OOS_DAILY_DATES+=("$date")
    oos_total_pnl=$(echo "$oos_total_pnl + $DAY_PNL" | bc -l)
    oos_total_trades=$(( oos_total_trades + DAY_TRADES ))
    oos_day_count=$(( oos_day_count + 1 ))

    if (( $(echo "$DAY_PNL > 0.001" | bc -l) )); then
        oos_win_days=$(( oos_win_days + 1 ))
        oos_gross_wins=$(echo "$oos_gross_wins + $DAY_PNL" | bc -l)
    elif (( $(echo "$DAY_PNL < -0.001" | bc -l) )); then
        local_abs=$(echo "$DAY_PNL" | tr -d '-')
        oos_gross_losses=$(echo "$oos_gross_losses + $local_abs" | bc -l)
    fi

    # accumulate per-ticker
    for tline in "${TICKER_LINES[@]}"; do
        parse_ticker_line "$tline"
        OOS_TICKER_PNL[$TICK_NAME]=$(echo "${OOS_TICKER_PNL[$TICK_NAME]:-0} + $TICK_PNL" | bc -l)
        OOS_TICKER_TRADES[$TICK_NAME]=$(( ${OOS_TICKER_TRADES[$TICK_NAME]:-0} + TICK_TRADES ))
        if (( $(echo "$TICK_PNL > 0.001" | bc -l) )); then
            OOS_TICKER_WIN_PNL[$TICK_NAME]=$(echo "${OOS_TICKER_WIN_PNL[$TICK_NAME]:-0} + $TICK_PNL" | bc -l)
            OOS_TICKER_WIN_DAYS[$TICK_NAME]=$(( ${OOS_TICKER_WIN_DAYS[$TICK_NAME]:-0} + 1 ))
        elif (( $(echo "$TICK_PNL < -0.001" | bc -l) )); then
            local_abs=$(echo "$TICK_PNL" | tr -d '-')
            OOS_TICKER_LOSS_PNL[$TICK_NAME]=$(echo "${OOS_TICKER_LOSS_PNL[$TICK_NAME]:-0} + $local_abs" | bc -l)
        fi
    done

    local_sign="+"
    if (( $(echo "$DAY_PNL < 0" | bc -l) )); then local_sign="-"; fi
    printf "  %s  %s\$%-10s  %2d trades\n" "$date" "$local_sign" "$(echo "$DAY_PNL" | tr -d '-')" "$DAY_TRADES"
done

# compute OOS aggregates
oos_win_rate=0
if [[ "$oos_day_count" -gt 0 ]]; then
    oos_win_rate=$(echo "scale=1; $oos_win_days * 100 / $oos_day_count" | bc -l)
fi
oos_pf="n/a"
oos_pf_num=0
if (( $(echo "$oos_gross_losses > 0" | bc -l) )); then
    oos_pf=$(echo "scale=2; $oos_gross_wins / $oos_gross_losses" | bc -l)
    oos_pf_num="$oos_pf"
elif (( $(echo "$oos_gross_wins > 0" | bc -l) )); then
    oos_pf="inf"
    oos_pf_num=999
fi

# per-day P&L ratio
oos_per_day_ratio="n/a"
if [[ "$is_day_count" -gt 0 ]] && [[ "$oos_day_count" -gt 0 ]]; then
    is_per_day=$(echo "scale=6; $is_total_pnl / $is_day_count" | bc -l)
    oos_per_day=$(echo "scale=6; $oos_total_pnl / $oos_day_count" | bc -l)
    if (( $(echo "$is_per_day > 0.001" | bc -l) )); then
        oos_per_day_ratio=$(echo "scale=0; $oos_per_day * 100 / $is_per_day" | bc -l)
    fi
fi

echo ""
printf "  in-sample (%d-day):     P&L %s  PF %s  win %s\n" \
    "$is_day_count" "$(fmt_pnl "$is_total_pnl")" "$is_pf" "$(fmt_pct "$is_win_rate")"
printf "  out-of-sample (%d-day): P&L %s  PF %s  win %s\n" \
    "$oos_day_count" "$(fmt_pnl "$oos_total_pnl")" "$oos_pf" "$(fmt_pct "$oos_win_rate")"
if [[ "$oos_per_day_ratio" != "n/a" ]]; then
    printf "  OOS/IS per-day P&L ratio: %s%%\n" "$oos_per_day_ratio"
fi
echo ""

# ─── section 3: slippage sensitivity sweep ──────────────────────────────────

if $QUICK; then
    echo "--- section 3: slippage sensitivity (SKIPPED in --quick mode) ---"
    echo ""
    slippage_pass=true
    slippage_breakeven="n/a"
else
    echo "--- section 3: slippage sensitivity sweep (${#SWEEP_DATES[@]}-day × 6 levels) ---"

    SLIPPAGE_LEVELS=(1 2 4 6 8 10)
    declare -A SLIPPAGE_PNL

    for bps in "${SLIPPAGE_LEVELS[@]}"; do
        sweep_pnl=0
        sweep_count=0
        for date in "${SWEEP_DATES[@]}"; do
            run_date "$date" "--slippage-bps $bps"

            if [[ -n "$DAY_PNL" ]]; then
                sweep_pnl=$(echo "$sweep_pnl + $DAY_PNL" | bc -l)
                sweep_count=$(( sweep_count + 1 ))
            fi
        done
        SLIPPAGE_PNL[$bps]="$sweep_pnl"
        printf "  %2d bps:  %s  (%d days)\n" "$bps" "$(fmt_pnl "$sweep_pnl")" "$sweep_count"
    done

    # find breakeven via linear interpolation
    slippage_breakeven="n/a"
    prev_bps=""
    prev_pnl=""
    for bps in "${SLIPPAGE_LEVELS[@]}"; do
        cur_pnl="${SLIPPAGE_PNL[$bps]}"
        if [[ -n "$prev_bps" ]]; then
            # check if sign changed between prev and cur
            if (( $(echo "$prev_pnl > 0 && $cur_pnl <= 0" | bc -l) )) || \
               (( $(echo "$prev_pnl >= 0 && $cur_pnl < 0" | bc -l) )); then
                # linear interpolation: breakeven = prev_bps + (prev_pnl / (prev_pnl - cur_pnl)) * (bps - prev_bps)
                slippage_breakeven=$(echo "scale=1; $prev_bps + ($prev_pnl / ($prev_pnl - $cur_pnl)) * ($bps - $prev_bps)" | bc -l)
                break
            fi
        fi
        prev_bps="$bps"
        prev_pnl="$cur_pnl"
    done

    # check if profitable at the highest level (all positive)
    if [[ "$slippage_breakeven" == "n/a" ]]; then
        last_pnl="${SLIPPAGE_PNL[${SLIPPAGE_LEVELS[-1]}]}"
        if (( $(echo "$last_pnl > 0" | bc -l) )); then
            slippage_breakeven=">10"
        fi
    fi

    printf "  breakeven: %s bps\n" "$slippage_breakeven"

    # pass if profitable at 6 bps
    slippage_pass=false
    pnl_at_6="${SLIPPAGE_PNL[6]:-0}"
    if (( $(echo "$pnl_at_6 > 0" | bc -l) )); then
        slippage_pass=true
    fi
    echo ""
fi

# ─── section 4: monte carlo daily P&L shuffle ──────────────────────────────

MC_ITERS=1000
if $QUICK; then MC_ITERS=100; fi

echo "--- section 4: monte carlo daily P&L shuffle ($MC_ITERS iterations) ---"

# feed IS daily P&Ls to awk for Fisher-Yates shuffle simulation
mc_result=$(printf '%s\n' "${IS_DAILY_PNL[@]}" | awk -v iters="$MC_ITERS" '
BEGIN { srand() }
{
    pnl[NR] = $1 + 0
    n = NR
}
END {
    for (iter = 1; iter <= iters; iter++) {
        # Fisher-Yates shuffle
        for (i = 1; i <= n; i++) shuf[i] = pnl[i]
        for (i = n; i > 1; i--) {
            j = int(rand() * i) + 1
            tmp = shuf[i]; shuf[i] = shuf[j]; shuf[j] = tmp
        }
        # compute total P&L and max drawdown for this shuffle
        total = 0; peak = 0; max_dd = 0
        for (i = 1; i <= n; i++) {
            total += shuf[i]
            if (total > peak) peak = total
            dd = peak - total
            if (dd > max_dd) max_dd = dd
        }
        results[iter] = total
        dd_results[iter] = max_dd
    }
    # sort results (simple insertion sort)
    for (i = 2; i <= iters; i++) {
        key = results[i]
        j = i - 1
        while (j >= 1 && results[j] > key) {
            results[j+1] = results[j]
            j--
        }
        results[j+1] = key
    }
    # sort dd results
    for (i = 2; i <= iters; i++) {
        key = dd_results[i]
        j = i - 1
        while (j >= 1 && dd_results[j] > key) {
            dd_results[j+1] = dd_results[j]
            j--
        }
        dd_results[j+1] = key
    }
    # percentiles (1-indexed)
    p5  = int(iters * 0.05) + 1
    p50 = int(iters * 0.50) + 1
    p95 = int(iters * 0.95) + 1
    printf "%.2f %.2f %.2f %.2f %.2f %.2f\n", \
        results[p5], results[p50], results[p95], \
        dd_results[p5], dd_results[p50], dd_results[p95]
}')

mc_pnl_5=$(echo "$mc_result" | awk '{print $1}')
mc_pnl_50=$(echo "$mc_result" | awk '{print $2}')
mc_pnl_95=$(echo "$mc_result" | awk '{print $3}')
mc_dd_5=$(echo "$mc_result" | awk '{print $4}')
mc_dd_50=$(echo "$mc_result" | awk '{print $5}')
mc_dd_95=$(echo "$mc_result" | awk '{print $6}')

printf "  P&L percentiles:       5th: %s  50th: %s  95th: %s\n" \
    "$(fmt_pnl "$mc_pnl_5")" "$(fmt_pnl "$mc_pnl_50")" "$(fmt_pnl "$mc_pnl_95")"
printf "  max drawdown pctiles:  5th: \$%.2f  50th: \$%.2f  95th: \$%.2f\n" \
    "$mc_dd_5" "$mc_dd_50" "$mc_dd_95"

mc_pass=false
if (( $(echo "$mc_pnl_5 > 0" | bc -l) )); then
    mc_pass=true
fi
echo ""

# ─── section 5: walk-forward stability ──────────────────────────────────────

echo "--- section 5: walk-forward stability ---"

# merge IS + OOS daily P&Ls chronologically
# combine dates and pnls into sortable pairs
ALL_PAIRS=()
for ((i=1; i<=${#IS_DAILY_DATES[@]}; i++)); do
    ALL_PAIRS+=("${IS_DAILY_DATES[$i]}|${IS_DAILY_PNL[$i]}")
done
for ((i=1; i<=${#OOS_DAILY_DATES[@]}; i++)); do
    ALL_PAIRS+=("${OOS_DAILY_DATES[$i]}|${OOS_DAILY_PNL[$i]}")
done

# sort chronologically
IFS=$'\n' SORTED_PAIRS=($(printf '%s\n' "${ALL_PAIRS[@]}" | sort)); unset IFS

ALL_DAILY_PNL=()
for pair in "${SORTED_PAIRS[@]}"; do
    ALL_DAILY_PNL+=("${pair#*|}")
done

total_all=${#ALL_DAILY_PNL[@]}

# sliding window: 20 train / 5 test, stride 5
TRAIN_SIZE=20
TEST_SIZE=5
STRIDE=5

wf_windows=0
wf_oos_profitable=0
wf_is_total=0
wf_oos_total=0

i=1
while (( i + TRAIN_SIZE + TEST_SIZE - 1 <= total_all )); do
    # compute IS (train) sum
    is_sum=0
    for (( j=i; j < i + TRAIN_SIZE; j++ )); do
        is_sum=$(echo "$is_sum + ${ALL_DAILY_PNL[$j]}" | bc -l)
    done

    # compute OOS (test) sum
    oos_sum=0
    for (( j=i + TRAIN_SIZE; j < i + TRAIN_SIZE + TEST_SIZE; j++ )); do
        oos_sum=$(echo "$oos_sum + ${ALL_DAILY_PNL[$j]}" | bc -l)
    done

    wf_is_total=$(echo "$wf_is_total + $is_sum" | bc -l)
    wf_oos_total=$(echo "$wf_oos_total + $oos_sum" | bc -l)
    wf_windows=$(( wf_windows + 1 ))

    if (( $(echo "$oos_sum > 0" | bc -l) )); then
        wf_oos_profitable=$(( wf_oos_profitable + 1 ))
    fi

    i=$(( i + STRIDE ))
done

wf_profitable_pct=0
wf_efficiency="n/a"
if [[ "$wf_windows" -gt 0 ]]; then
    wf_profitable_pct=$(echo "scale=0; $wf_oos_profitable * 100 / $wf_windows" | bc -l)
    if (( $(echo "$wf_is_total > 0.001" | bc -l) )); then
        wf_efficiency=$(echo "scale=0; $wf_oos_total * 100 / $wf_is_total" | bc -l)
    fi
fi

printf "  windows: %d (train=%d, test=%d, stride=%d)\n" "$wf_windows" "$TRAIN_SIZE" "$TEST_SIZE" "$STRIDE"
printf "  OOS profitable windows: %d/%d (%s%%)\n" "$wf_oos_profitable" "$wf_windows" "$wf_profitable_pct"
if [[ "$wf_efficiency" != "n/a" ]]; then
    printf "  walk-forward efficiency (WFE): %s%%\n" "$wf_efficiency"
fi

wf_pass=false
if (( $(echo "$wf_profitable_pct > 60" | bc -l) )) && [[ "$wf_efficiency" != "n/a" ]] && (( $(echo "$wf_efficiency > 50" | bc -l) )); then
    wf_pass=true
elif (( $(echo "$wf_profitable_pct > 60" | bc -l) )) && [[ "$wf_efficiency" == "n/a" ]]; then
    wf_pass=true  # if IS total <= 0, WFE is undefined but profitability still matters
fi
echo ""

# ─── section 6: per-ticker analysis ─────────────────────────────────────────

echo "--- section 6: per-ticker analysis ---"

# collect all ticker names
declare -A ALL_TICKER_NAMES
for t in ${(k)IS_TICKER_PNL}; do ALL_TICKER_NAMES[$t]=1; done
for t in ${(k)OOS_TICKER_PNL}; do ALL_TICKER_NAMES[$t]=1; done

ticker_pass=true

printf "  %-6s  %12s  %7s  %10s  %6s  (%s / %s)\n" \
    "ticker" "total P&L" "trades" "win days" "PF" "IS P&L" "OOS P&L"

for ticker in $(echo "${(k)ALL_TICKER_NAMES}" | tr ' ' '\n' | sort); do
    is_pnl="${IS_TICKER_PNL[$ticker]:-0}"
    oos_pnl="${OOS_TICKER_PNL[$ticker]:-0}"
    combined_pnl=$(echo "$is_pnl + $oos_pnl" | bc -l)

    is_trades="${IS_TICKER_TRADES[$ticker]:-0}"
    oos_trades="${OOS_TICKER_TRADES[$ticker]:-0}"
    combined_trades=$(( is_trades + oos_trades ))

    is_win="${IS_TICKER_WIN_DAYS[$ticker]:-0}"
    oos_win="${OOS_TICKER_WIN_DAYS[$ticker]:-0}"
    combined_win=$(( is_win + oos_win ))
    combined_days=$(( is_day_count + oos_day_count ))

    is_gwins="${IS_TICKER_WIN_PNL[$ticker]:-0}"
    oos_gwins="${OOS_TICKER_WIN_PNL[$ticker]:-0}"
    combined_gwins=$(echo "$is_gwins + $oos_gwins" | bc -l)

    is_glosses="${IS_TICKER_LOSS_PNL[$ticker]:-0}"
    oos_glosses="${OOS_TICKER_LOSS_PNL[$ticker]:-0}"
    combined_glosses=$(echo "$is_glosses + $oos_glosses" | bc -l)

    tick_pf="n/a"
    tick_pf_num=0
    if (( $(echo "$combined_glosses > 0" | bc -l) )); then
        tick_pf=$(echo "scale=1; $combined_gwins / $combined_glosses" | bc -l)
        tick_pf_num="$tick_pf"
    elif (( $(echo "$combined_gwins > 0" | bc -l) )); then
        tick_pf="inf"
        tick_pf_num=999
    fi

    flag=""
    if (( $(echo "$tick_pf_num < 0.8" | bc -l) )) && [[ "$tick_pf" != "n/a" ]]; then
        flag=" *** FLAGGED"
        ticker_pass=false
    fi

    printf "  %-6s  %12s  %7d  %4d/%-4d  %6s  (%s / %s)%s\n" \
        "$ticker" \
        "$(fmt_pnl "$combined_pnl")" \
        "$combined_trades" \
        "$combined_win" "$combined_days" \
        "$tick_pf" \
        "$(fmt_pnl "$is_pnl")" \
        "$(fmt_pnl "$oos_pnl")" \
        "$flag"
done

echo ""

# ─── section 7: report card ─────────────────────────────────────────────────

echo "=== validation report card ==="

pass_count=0
total_checks=0

# check 1: hold-out OOS
total_checks=$(( total_checks + 1 ))
oos_check_pass=false
if (( $(echo "$oos_total_pnl > 0" | bc -l) )) && (( $(echo "$oos_pf_num > 1.0" | bc -l) )); then
    oos_check_pass=true
fi
# also pass if PF is "inf" (no losses, positive P&L)
if (( $(echo "$oos_total_pnl > 0" | bc -l) )) && [[ "$oos_pf" == "inf" ]]; then
    oos_check_pass=true
fi

if $oos_check_pass; then
    pass_count=$(( pass_count + 1 ))
    printf "  [PASS] hold-out OOS test        P&L %s  PF %s" "$(fmt_pnl "$oos_total_pnl")" "$oos_pf"
else
    printf "  [FAIL] hold-out OOS test        P&L %s  PF %s" "$(fmt_pnl "$oos_total_pnl")" "$oos_pf"
fi
if [[ "$oos_per_day_ratio" != "n/a" ]]; then
    printf "  (%s%% per-day ratio)" "$oos_per_day_ratio"
fi
echo ""

# check 2: slippage sensitivity
total_checks=$(( total_checks + 1 ))
if $slippage_pass; then
    pass_count=$(( pass_count + 1 ))
    if $QUICK; then
        printf "  [SKIP] slippage sensitivity     (skipped in --quick mode)\n"
        total_checks=$(( total_checks - 1 ))  # don't count skipped
    else
        printf "  [PASS] slippage sensitivity     profitable at 6 bps (breakeven: %s bps)\n" "$slippage_breakeven"
    fi
else
    printf "  [FAIL] slippage sensitivity     NOT profitable at 6 bps (breakeven: %s bps)\n" "$slippage_breakeven"
fi

# check 3: monte carlo
total_checks=$(( total_checks + 1 ))
if $mc_pass; then
    pass_count=$(( pass_count + 1 ))
    printf "  [PASS] monte carlo shuffle      5th pctl: %s\n" "$(fmt_pnl "$mc_pnl_5")"
else
    printf "  [FAIL] monte carlo shuffle      5th pctl: %s\n" "$(fmt_pnl "$mc_pnl_5")"
fi

# check 4: walk-forward
total_checks=$(( total_checks + 1 ))
if $wf_pass; then
    pass_count=$(( pass_count + 1 ))
    printf "  [PASS] walk-forward stability   %s%% OOS profitable" "$wf_profitable_pct"
else
    printf "  [FAIL] walk-forward stability   %s%% OOS profitable" "$wf_profitable_pct"
fi
if [[ "$wf_efficiency" != "n/a" ]]; then
    printf ", WFE %s%%" "$wf_efficiency"
fi
echo ""

# check 5: per-ticker
total_checks=$(( total_checks + 1 ))
if $ticker_pass; then
    pass_count=$(( pass_count + 1 ))
    printf "  [PASS] per-ticker analysis      all tickers PF >= 0.8\n"
else
    printf "  [FAIL] per-ticker analysis      one or more tickers PF < 0.8\n"
fi

echo "  ---"
if [[ "$pass_count" -eq "$total_checks" ]]; then
    printf "  OVERALL: PASS (%d/%d)\n" "$pass_count" "$total_checks"
else
    printf "  OVERALL: FAIL (%d/%d)\n" "$pass_count" "$total_checks"
fi
echo ""
