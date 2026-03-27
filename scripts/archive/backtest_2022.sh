#!/usr/bin/env bash
# run backtest on every trading day in 2022 (bear market stress test)
# generates ~251 trading dates dynamically, skipping weekends and NYSE holidays
# logs all verbose output to logs/backtest_2022_full.log for post-analysis
#
# market context: sustained bear market, SPY -19%, QQQ -33%, VIX averaged ~25,
# aggressive rate hike cycle, tech selloff. tests strategy survival in downtrend.
#
# usage: ./scripts/backtest_2022.sh [--compound] [extra backtest args...]

set -euo pipefail

YEAR=2022

# NYSE market holidays for 2022
HOLIDAYS=(
    2022-01-17  # MLK day
    2022-02-21  # presidents' day
    2022-04-15  # good friday
    2022-05-30  # memorial day
    2022-06-20  # juneteenth (observed, first year as federal holiday)
    2022-07-04  # independence day
    2022-09-05  # labor day
    2022-11-24  # thanksgiving
    2022-12-26  # christmas (observed, dec 25 = sunday)
)

# generate all trading dates (weekdays minus holidays)
DATES=()
d="${YEAR}-01-01"
end_exclusive="$((YEAR + 1))-01-01"

while [[ "$d" < "$end_exclusive" ]]; do
    dow=$(date -j -f "%Y-%m-%d" "$d" "+%u" 2>/dev/null)
    if [[ "$dow" -le 5 ]]; then
        is_holiday=false
        for h in "${HOLIDAYS[@]}"; do
            if [[ "$d" == "$h" ]]; then
                is_holiday=true
                break
            fi
        done
        if ! $is_holiday; then
            DATES+=("$d")
        fi
    fi
    d=$(date -j -v+1d -f "%Y-%m-%d" "$d" "+%Y-%m-%d" 2>/dev/null)
done

CAPITAL=10000
LOOKBACK=3
COST_ARGS="--slippage-bps 2.0 --half-spread 0.005"
COMPOUND=false
EXTRA_ARGS=()

# parse script args — known flags handled here, rest forwarded to backtest
while [[ $# -gt 0 ]]; do
    case "$1" in
        --compound) COMPOUND=true; shift ;;
        *) EXTRA_ARGS+=("$1"); shift ;;
    esac
done

BACKTEST="cargo run -p backtest --release --"

# ensure log directory exists
mkdir -p logs

LOG_FILE="logs/backtest_${YEAR}_full.log"
STARTED_AT=$(date "+%Y-%m-%d %H:%M:%S")

echo "=== ${YEAR} full-year backtest (${#DATES[@]} trading days) ==="
if $COMPOUND; then
    echo "capital: \$${CAPITAL} (compounding)  lookback: ${LOOKBACK} days  costs: ${COST_ARGS}"
else
    echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days  costs: ${COST_ARGS}"
fi
if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    echo "overrides: ${EXTRA_ARGS[*]}"
fi
echo "log: ${LOG_FILE}"
echo ""

# initialize log file with run metadata
cat > "$LOG_FILE" <<HEADER
=== ${YEAR} full-year backtest ===
started: ${STARTED_AT}
trading days: ${#DATES[@]}
capital: \$${CAPITAL}
lookback: ${LOOKBACK} days
costs: ${COST_ARGS}
compound: ${COMPOUND}
extra args: ${EXTRA_ARGS[*]+"${EXTRA_ARGS[*]}"}

HEADER

# --- tracking variables ---
total_pnl=0
total_trades=0
win_days=0
loss_days=0
flat_days=0
biggest_win=0
biggest_win_date=""
biggest_loss=0
biggest_loss_date=""
day_count=0
gross_wins=0
gross_losses=0
sum_win_pnl=0
sum_loss_pnl=0
max_consec_loss=0
cur_consec_loss=0
rate_limit_warnings=0
data_quality_warnings=0
error_days=0

# drawdown tracking
peak_equity=0
max_drawdown=0
max_drawdown_date=""

# temp files for per-ticker and monthly tracking
TICKER_LOG=$(mktemp)
MONTHLY_LOG=$(mktemp)
trap "rm -f $TICKER_LOG $MONTHLY_LOG" EXIT

total_dates=${#DATES[@]}
date_idx=0
current_capital=$CAPITAL

for date in "${DATES[@]}"; do
    date_idx=$(( date_idx + 1 ))
    equity_args=""
    if $COMPOUND; then
        equity_args="--output-equity"
    fi
    output=$($BACKTEST --date "$date" --capital "$current_capital" --lookback-days "$LOOKBACK" --verbose $COST_ARGS $equity_args ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1) || true

    # log full verbose output for post-analysis
    echo "--- ${date} (day ${date_idx}/${total_dates}) ---" >> "$LOG_FILE"
    echo "$output" >> "$LOG_FILE"
    echo "" >> "$LOG_FILE"

    # update capital for next day if compounding
    if $COMPOUND; then
        equity_line=$(echo "$output" | grep "^ENDING_EQUITY=" || true)
        if [[ -n "$equity_line" ]]; then
            current_capital=$(echo "$equity_line" | sed 's/ENDING_EQUITY=//')
        fi
    fi

    # check for rate limiting / data quality issues
    if echo "$output" | grep -q "rate limited"; then
        rate_limit_warnings=$(( rate_limit_warnings + 1 ))
    fi
    if echo "$output" | grep -q "WARNING.*candles"; then
        data_quality_warnings=$(( data_quality_warnings + 1 ))
    fi

    # parse per-ticker results from output lines like: "  SPY    +$123.45  (3 trades)"
    for ticker in SPY QQQ AAPL MSFT NVDA; do
        ticker_line=$(echo "$output" | grep -E "^\s+${ticker}\s" | head -1 || true)
        if [[ -n "$ticker_line" ]]; then
            t_pnl=$(echo "$ticker_line" | sed -E 's/.*[+-]\$([0-9.]+).*/\1/')
            t_sign=$(echo "$ticker_line" | sed -E 's/.*([+-])\$.*/\1/')
            if [[ "$t_sign" == "-" ]]; then t_pnl="-$t_pnl"; fi
            t_trades=$(echo "$ticker_line" | sed -E 's/.*\(([0-9]+) trades?\).*/\1/')
            echo "$ticker $t_pnl $t_trades $date" >> "$TICKER_LOG"
        fi
    done

    total_line=$(echo "$output" | grep -E "^\s+total\s" || true)

    if [[ -z "$total_line" ]]; then
        printf "  [%d/%d]  %s  no result\n" "$date_idx" "$total_dates" "$date"
        error_days=$(( error_days + 1 ))
        day_count=$(( day_count + 1 ))
        month=${date:0:7}
        echo "$month 0 0" >> "$MONTHLY_LOG"
        continue
    fi

    day_pnl=$(echo "$total_line" | sed -E 's/.*[+-]\$([0-9.]+).*/\1/')
    day_sign=$(echo "$total_line" | sed -E 's/.*([+-])\$.*/\1/')
    if [[ "$day_sign" == "-" ]]; then
        day_pnl="-$day_pnl"
    fi

    day_trades=$(echo "$total_line" | sed -E 's/.*\(([0-9]+) trades\).*/\1/')

    total_pnl=$(echo "$total_pnl + $day_pnl" | bc -l)
    total_trades=$(( total_trades + day_trades ))
    day_count=$(( day_count + 1 ))

    # monthly tracking
    month=${date:0:7}
    echo "$month $day_pnl $day_trades" >> "$MONTHLY_LOG"

    # drawdown tracking (peak-to-trough on cumulative P&L)
    if (( $(echo "$total_pnl > $peak_equity" | bc -l) )); then
        peak_equity="$total_pnl"
    fi
    current_dd=$(echo "$peak_equity - $total_pnl" | bc -l)
    if (( $(echo "$current_dd > $max_drawdown" | bc -l) )); then
        max_drawdown="$current_dd"
        max_drawdown_date="$date"
    fi

    if (( $(echo "$day_pnl > 0.001" | bc -l) )); then
        win_days=$(( win_days + 1 ))
        gross_wins=$(echo "$gross_wins + $day_pnl" | bc -l)
        sum_win_pnl=$(echo "$sum_win_pnl + $day_pnl" | bc -l)
        cur_consec_loss=0
        if (( $(echo "$day_pnl > $biggest_win" | bc -l) )); then
            biggest_win="$day_pnl"
            biggest_win_date="$date"
        fi
    elif (( $(echo "$day_pnl < -0.001" | bc -l) )); then
        loss_days=$(( loss_days + 1 ))
        abs_day_loss=$(echo "$day_pnl" | tr -d '-')
        gross_losses=$(echo "$gross_losses + $abs_day_loss" | bc -l)
        sum_loss_pnl=$(echo "$sum_loss_pnl + $day_pnl" | bc -l)
        cur_consec_loss=$(( cur_consec_loss + 1 ))
        if [[ "$cur_consec_loss" -gt "$max_consec_loss" ]]; then
            max_consec_loss=$cur_consec_loss
        fi
        if (( $(echo "$day_pnl < $biggest_loss" | bc -l) )); then
            biggest_loss="$day_pnl"
            biggest_loss_date="$date"
        fi
    else
        flat_days=$(( flat_days + 1 ))
        cur_consec_loss=0
    fi

    printf "  [%d/%d]  %s  %s\$%-10s  %2d trades\n" "$date_idx" "$total_dates" "$date" "$day_sign" "$(echo "$day_pnl" | tr -d '-')" "$day_trades"
done

ENDED_AT=$(date "+%Y-%m-%d %H:%M:%S")

echo ""
echo "=== summary ==="
printf "year:           %d\n" "$YEAR"
printf "days:           %d (win: %d  loss: %d  flat: %d  error: %d)\n" "$day_count" "$win_days" "$loss_days" "$flat_days" "$error_days"
active_days=$(( win_days + loss_days ))
if [[ "$day_count" -gt 0 ]]; then
    active_pct=$(echo "scale=1; $active_days * 100 / $day_count" | bc -l)
    printf "active days:    %d / %d (%s%%)\n" "$active_days" "$day_count" "$active_pct"
fi
printf "total trades:   %d\n" "$total_trades"

sign="+"; abs_pnl="$total_pnl"
if (( $(echo "$total_pnl < 0" | bc -l) )); then sign="-"; abs_pnl=$(echo "$total_pnl" | tr -d '-'); fi
printf "total P&L:      %s\$%.2f\n" "$sign" "$abs_pnl"

if [[ "$day_count" -gt 0 ]]; then
    avg_pnl=$(echo "scale=2; $total_pnl / $day_count" | bc -l)
    avg_sign="+"; abs_avg=$(echo "$avg_pnl" | tr -d '-')
    if (( $(echo "$avg_pnl < 0" | bc -l) )); then avg_sign="-"; fi
    printf "avg daily P&L:  %s\$%s\n" "$avg_sign" "$abs_avg"
fi

if [[ -n "$biggest_win_date" ]]; then
    printf "biggest win:    +\$%.2f (%s)\n" "$biggest_win" "$biggest_win_date"
fi
if [[ -n "$biggest_loss_date" ]]; then
    abs_loss=$(echo "$biggest_loss" | tr -d '-')
    printf "biggest loss:   -\$%.2f (%s)\n" "$abs_loss" "$biggest_loss_date"
fi

if [[ "$day_count" -gt 0 ]]; then
    avg_trades=$(echo "scale=1; $total_trades / $day_count" | bc -l)
    printf "avg trades/day: %s\n" "$avg_trades"
fi
if [[ "$win_days" -gt 0 ]] && [[ "$day_count" -gt 0 ]]; then
    win_pct=$(echo "scale=1; $win_days * 100 / $day_count" | bc -l)
    printf "win day rate:   %s%%\n" "$win_pct"
fi

# profit factor
if (( $(echo "$gross_losses > 0" | bc -l) )); then
    pf=$(echo "scale=2; $gross_wins / $gross_losses" | bc -l)
    printf "profit factor:  %s\n" "$pf"
elif (( $(echo "$gross_wins > 0" | bc -l) )); then
    printf "profit factor:  inf (no losses)\n"
else
    printf "profit factor:  n/a\n"
fi

# avg win vs avg loss
if [[ "$win_days" -gt 0 ]]; then
    avg_win=$(echo "scale=2; $sum_win_pnl / $win_days" | bc -l)
    printf "avg win day:    +\$%s\n" "$avg_win"
fi
if [[ "$loss_days" -gt 0 ]]; then
    avg_loss=$(echo "scale=2; $sum_loss_pnl / $loss_days" | bc -l)
    abs_avg_loss=$(echo "$avg_loss" | tr -d '-')
    printf "avg loss day:   -\$%s\n" "$abs_avg_loss"
fi
if [[ "$win_days" -gt 0 ]] && [[ "$loss_days" -gt 0 ]]; then
    avg_win=$(echo "scale=4; $sum_win_pnl / $win_days" | bc -l)
    avg_loss_abs=$(echo "scale=4; ($sum_loss_pnl * -1) / $loss_days" | bc -l)
    if (( $(echo "$avg_loss_abs > 0" | bc -l) )); then
        wl_ratio=$(echo "scale=2; $avg_win / $avg_loss_abs" | bc -l)
        printf "win/loss ratio: %s\n" "$wl_ratio"
    fi
fi

printf "max consec loss days: %d\n" "$max_consec_loss"

# drawdown
if (( $(echo "$max_drawdown > 0" | bc -l) )); then
    dd_pct=$(echo "scale=2; $max_drawdown * 100 / $CAPITAL" | bc -l)
    printf "max drawdown:   \$%.2f (%.1f%% of capital, at %s)\n" "$max_drawdown" "$dd_pct" "$max_drawdown_date"
fi

# return on capital
if [[ "$day_count" -gt 0 ]]; then
    roc=$(echo "scale=2; $total_pnl * 100 / $CAPITAL" | bc -l)
    roc_sign="+"; abs_roc=$(echo "$roc" | tr -d '-')
    if (( $(echo "$roc < 0" | bc -l) )); then roc_sign="-"; fi
    printf "return on cap:  %s%s%%\n" "$roc_sign" "$abs_roc"
fi

# === per-ticker breakdown ===
echo ""
echo "=== per-ticker breakdown ==="
printf "  %-6s  %10s  %6s  %8s  %8s\n" "ticker" "P&L" "trades" "act.days" "avg/day"
for t in SPY QQQ AAPL MSFT NVDA; do
    if grep -q "^$t " "$TICKER_LOG" 2>/dev/null; then
        t_total_pnl=$(awk -v ticker="$t" '$1==ticker {sum+=$2} END {printf "%.2f", sum}' "$TICKER_LOG")
        t_total_trades=$(awk -v ticker="$t" '$1==ticker {sum+=$3} END {printf "%d", sum}' "$TICKER_LOG")
        t_active_days=$(awk -v ticker="$t" '$1==ticker && $3+0>0 {count++} END {printf "%d", count+0}' "$TICKER_LOG")
        t_avg="n/a"
        if [[ "$t_active_days" -gt 0 ]]; then
            t_avg=$(echo "scale=2; $t_total_pnl / $t_active_days" | bc -l)
            t_avg_sign="+"; if (( $(echo "$t_avg < 0" | bc -l) )); then t_avg_sign="-"; fi
            t_avg="${t_avg_sign}\$$(echo "$t_avg" | tr -d '-')"
        fi
        t_sign="+"; if (( $(echo "$t_total_pnl < 0" | bc -l) )); then t_sign="-"; fi
        printf "  %-6s  %s\$%-8.2f  %6d  %8d  %8s\n" "$t" "$t_sign" "$(echo "$t_total_pnl" | tr -d '-')" "$t_total_trades" "$t_active_days" "$t_avg"
    fi
done

# === monthly breakdown ===
echo ""
echo "=== monthly breakdown ==="
printf "  %-10s  %10s  %6s  %4s  %4s  %4s\n" "month" "P&L" "trades" "win" "loss" "flat"
current_month=""
m_pnl=0; m_trades=0; m_win=0; m_loss=0; m_flat=0
while IFS=' ' read -r month pnl trades; do
    if [[ "$current_month" != "$month" ]]; then
        if [[ -n "$current_month" ]]; then
            m_sign="+"; if (( $(echo "$m_pnl < 0" | bc -l) )); then m_sign="-"; fi
            printf "  %-10s  %s\$%-8.2f  %6d  %4d  %4d  %4d\n" "$current_month" "$m_sign" "$(echo "$m_pnl" | tr -d '-')" "$m_trades" "$m_win" "$m_loss" "$m_flat"
        fi
        current_month="$month"
        m_pnl="$pnl"; m_trades=$((trades)); m_win=0; m_loss=0; m_flat=0
        if (( $(echo "$pnl > 0.001" | bc -l) )); then m_win=1
        elif (( $(echo "$pnl < -0.001" | bc -l) )); then m_loss=1
        else m_flat=1; fi
    else
        m_pnl=$(echo "$m_pnl + $pnl" | bc -l)
        m_trades=$(( m_trades + trades ))
        if (( $(echo "$pnl > 0.001" | bc -l) )); then m_win=$(( m_win + 1 ))
        elif (( $(echo "$pnl < -0.001" | bc -l) )); then m_loss=$(( m_loss + 1 ))
        else m_flat=$(( m_flat + 1 )); fi
    fi
done < "$MONTHLY_LOG"
# print last month
if [[ -n "$current_month" ]]; then
    m_sign="+"; if (( $(echo "$m_pnl < 0" | bc -l) )); then m_sign="-"; fi
    printf "  %-10s  %s\$%-8.2f  %6d  %4d  %4d  %4d\n" "$current_month" "$m_sign" "$(echo "$m_pnl" | tr -d '-')" "$m_trades" "$m_win" "$m_loss" "$m_flat"
fi

# === rubric ===
echo ""
echo "=== rubric ==="
# thresholds calibrated for correct position sizing ($10k capital, ~3% max fraction)

score_total=0
metric_count=0

# 1. total P&L score (weight 3x)
if (( $(echo "$total_pnl >= 100" | bc -l) )); then pnl_score=5
elif (( $(echo "$total_pnl >= 25" | bc -l) )); then pnl_score=4
elif (( $(echo "$total_pnl >= 0" | bc -l) )); then pnl_score=3
elif (( $(echo "$total_pnl >= -100" | bc -l) )); then pnl_score=2
else pnl_score=1; fi
score_total=$(( score_total + pnl_score * 3 ))
metric_count=$(( metric_count + 3 ))
printf "  total P&L:          %d/5  (>\$100=5, >\$25=4, >\$0=3, >-\$100=2, else=1)\n" "$pnl_score"

# 2. win day rate (weight 2x) — of active days only
if [[ "$active_days" -gt 0 ]]; then
    wr=$(echo "scale=1; $win_days * 100 / $active_days" | bc -l)
    if (( $(echo "$wr >= 65" | bc -l) )); then wr_score=5
    elif (( $(echo "$wr >= 55" | bc -l) )); then wr_score=4
    elif (( $(echo "$wr >= 45" | bc -l) )); then wr_score=3
    elif (( $(echo "$wr >= 35" | bc -l) )); then wr_score=2
    else wr_score=1; fi
else wr_score=1; fi
score_total=$(( score_total + wr_score * 2 ))
metric_count=$(( metric_count + 2 ))
printf "  win day rate:       %d/5  (>65%%=5, >55%%=4, >45%%=3, >35%%=2, else=1) [of active days]\n" "$wr_score"

# 3. profit factor (weight 2x)
if (( $(echo "$gross_losses > 0" | bc -l) )); then
    pf=$(echo "scale=4; $gross_wins / $gross_losses" | bc -l)
    if (( $(echo "$pf >= 2.0" | bc -l) )); then pf_score=5
    elif (( $(echo "$pf >= 1.5" | bc -l) )); then pf_score=4
    elif (( $(echo "$pf >= 1.0" | bc -l) )); then pf_score=3
    elif (( $(echo "$pf >= 0.5" | bc -l) )); then pf_score=2
    else pf_score=1; fi
elif (( $(echo "$gross_wins > 0" | bc -l) )); then
    pf_score=5
else
    pf_score=1
fi
score_total=$(( score_total + pf_score * 2 ))
metric_count=$(( metric_count + 2 ))
printf "  profit factor:      %d/5  (>2.0=5, >1.5=4, >1.0=3, >0.5=2, else=1)\n" "$pf_score"

# 4. trade frequency (weight 1x) — avg trades per active day
if [[ "$active_days" -gt 0 ]]; then
    at=$(echo "scale=1; $total_trades / $active_days" | bc -l)
    if (( $(echo "$at >= 2 && $at <= 10" | bc -l) )); then tf_score=5
    elif (( $(echo "$at >= 1 && $at <= 15" | bc -l) )); then tf_score=4
    elif (( $(echo "$at >= 0.5 && $at <= 20" | bc -l) )); then tf_score=3
    elif (( $(echo "$at <= 30" | bc -l) )); then tf_score=2
    else tf_score=1; fi
else tf_score=1; fi
score_total=$(( score_total + tf_score ))
metric_count=$(( metric_count + 1 ))
printf "  trade frequency:    %d/5  (2-10=5, 1-15=4, 0.5-20=3, <30=2, else=1) [per active day]\n" "$tf_score"

# 5. tail risk (weight 2x) — biggest single-day loss as % of capital
if [[ -n "$biggest_loss_date" ]]; then
    abs_bl=$(echo "$biggest_loss" | tr -d '-')
    bl_pct=$(echo "scale=2; $abs_bl * 100 / $CAPITAL" | bc -l)
    if (( $(echo "$bl_pct <= 0.1" | bc -l) )); then tr_score=5
    elif (( $(echo "$bl_pct <= 0.2" | bc -l) )); then tr_score=4
    elif (( $(echo "$bl_pct <= 0.5" | bc -l) )); then tr_score=3
    elif (( $(echo "$bl_pct <= 1.0" | bc -l) )); then tr_score=2
    else tr_score=1; fi
else tr_score=5; fi
score_total=$(( score_total + tr_score * 2 ))
metric_count=$(( metric_count + 2 ))
printf "  tail risk:          %d/5  (max loss <0.1%%cap=5, <0.2%%=4, <0.5%%=3, <1%%=2, else=1)\n" "$tr_score"

# 6. win/loss asymmetry (weight 1x)
if [[ "$win_days" -gt 0 ]] && [[ "$loss_days" -gt 0 ]]; then
    avg_win_val=$(echo "scale=4; $sum_win_pnl / $win_days" | bc -l)
    avg_loss_val=$(echo "scale=4; ($sum_loss_pnl * -1) / $loss_days" | bc -l)
    if (( $(echo "$avg_loss_val > 0" | bc -l) )); then
        wlr=$(echo "scale=4; $avg_win_val / $avg_loss_val" | bc -l)
        if (( $(echo "$wlr >= 2.0" | bc -l) )); then wl_score=5
        elif (( $(echo "$wlr >= 1.5" | bc -l) )); then wl_score=4
        elif (( $(echo "$wlr >= 1.0" | bc -l) )); then wl_score=3
        elif (( $(echo "$wlr >= 0.5" | bc -l) )); then wl_score=2
        else wl_score=1; fi
    else wl_score=5; fi
else wl_score=3; fi
score_total=$(( score_total + wl_score ))
metric_count=$(( metric_count + 1 ))
printf "  win/loss asymmetry: %d/5  (avg_win/avg_loss >2=5, >1.5=4, >1=3, >0.5=2, else=1)\n" "$wl_score"

# 7. max drawdown (weight 2x)
if (( $(echo "$max_drawdown > 0" | bc -l) )); then
    dd_pct=$(echo "scale=2; $max_drawdown * 100 / $CAPITAL" | bc -l)
    if (( $(echo "$dd_pct <= 0.5" | bc -l) )); then dd_score=5
    elif (( $(echo "$dd_pct <= 1" | bc -l) )); then dd_score=4
    elif (( $(echo "$dd_pct <= 2" | bc -l) )); then dd_score=3
    elif (( $(echo "$dd_pct <= 5" | bc -l) )); then dd_score=2
    else dd_score=1; fi
else dd_score=5; fi
score_total=$(( score_total + dd_score * 2 ))
metric_count=$(( metric_count + 2 ))
printf "  max drawdown:       %d/5  (<0.5%%cap=5, <1%%=4, <2%%=3, <5%%=2, else=1)\n" "$dd_score"

# composite score
if [[ "$metric_count" -gt 0 ]]; then
    max_score=$(( metric_count * 5 ))
    composite=$(echo "scale=1; $score_total * 10 / $max_score" | bc -l)
    printf "\n  COMPOSITE SCORE:    %s / 10.0\n" "$composite"
fi

# data quality warnings
if [[ "$rate_limit_warnings" -gt 0 ]] || [[ "$data_quality_warnings" -gt 0 ]] || [[ "$error_days" -gt 0 ]]; then
    echo ""
    echo "=== DATA QUALITY ==="
    if [[ "$rate_limit_warnings" -gt 0 ]]; then
        printf "  rate limit retries detected on %d/%d days — results may be unreliable\n" "$rate_limit_warnings" "$day_count"
    fi
    if [[ "$data_quality_warnings" -gt 0 ]]; then
        printf "  low candle counts on %d/%d days — possible data gaps\n" "$data_quality_warnings" "$day_count"
    fi
    if [[ "$error_days" -gt 0 ]]; then
        printf "  %d/%d days returned no result (holidays? data gaps?)\n" "$error_days" "$day_count"
    fi
fi

echo ""
printf "completed: %s\n" "$ENDED_AT"
printf "full log:  %s\n" "$LOG_FILE"

# append summary to log file
{
    echo ""
    echo "=== run complete ==="
    echo "ended: $ENDED_AT"
    echo "total P&L: ${sign}\$$(echo "$abs_pnl" | xargs printf '%.2f')"
    echo "days: $day_count (win: $win_days  loss: $loss_days  flat: $flat_days  error: $error_days)"
    echo "trades: $total_trades"
} >> "$LOG_FILE"
