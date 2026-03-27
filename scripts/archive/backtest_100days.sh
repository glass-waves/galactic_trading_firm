#!/usr/bin/env bash
# run backtest on a fixed set of ~100 dates and report aggregate results with rubric
# usage: ./scripts/backtest_100days.sh

set -euo pipefail

DATES=(
    2025-03-10
    2025-03-12
    2025-03-17
    2025-03-19
    2025-03-24
    2025-03-26
    2025-03-31
    2025-04-02
    2025-04-07
    2025-04-09
    2025-04-14
    2025-04-16
    2025-04-21
    2025-04-23
    2025-04-28
    2025-04-30
    2025-05-05
    2025-05-07
    2025-05-12
    2025-05-14
    2025-05-19
    2025-05-21
    2025-05-28
    2025-06-02
    2025-06-04
    2025-06-09
    2025-06-11
    2025-06-16
    2025-06-18
    2025-06-23
    2025-06-25
    2025-06-30
    2025-07-02
    2025-07-07
    2025-07-09
    2025-07-14
    2025-07-16
    2025-07-21
    2025-07-23
    2025-07-28
    2025-07-30
    2025-08-04
    2025-08-06
    2025-08-11
    2025-08-13
    2025-08-18
    2025-08-20
    2025-08-25
    2025-08-27
    2025-09-03
    2025-09-08
    2025-09-10
    2025-09-15
    2025-09-17
    2025-09-22
    2025-09-24
    2025-09-29
    2025-10-01
    2025-10-06
    2025-10-08
    2025-10-13
    2025-10-15
    2025-10-20
    2025-10-22
    2025-10-27
    2025-10-29
    2025-11-03
    2025-11-05
    2025-11-10
    2025-11-12
    2025-11-17
    2025-11-19
    2025-11-24
    2025-11-26
    2025-12-01
    2025-12-03
    2025-12-08
    2025-12-10
    2025-12-15
    2025-12-17
    2025-12-22
    2025-12-24
    2025-12-29
    2025-12-31
    2026-01-05
    2026-01-07
    2026-01-12
    2026-01-14
    2026-01-21
    2026-01-26
    2026-01-28
    2026-02-02
    2026-02-04
    2026-02-09
    2026-02-11
    2026-02-18
    2026-02-23
    2026-02-25
    2026-03-02
)

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

echo "=== ${#DATES[@]}-day backtest sweep ==="
if $COMPOUND; then
    echo "capital: \$${CAPITAL} (compounding)  lookback: ${LOOKBACK} days  costs: ${COST_ARGS}"
else
    echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days  costs: ${COST_ARGS}"
fi
echo ""

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

current_capital=$CAPITAL
for date in "${DATES[@]}"; do
    equity_args=""
    if $COMPOUND; then
        equity_args="--output-equity"
    fi
    output=$($BACKTEST --date "$date" --capital "$current_capital" --lookback-days "$LOOKBACK" $COST_ARGS $equity_args ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1) || true

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

    total_line=$(echo "$output" | grep -E "^\s+total\s" || true)

    if [[ -z "$total_line" ]]; then
        printf "  %s  ERROR (no result)\n" "$date"
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

    printf "  %s  %s\$%-10s  %2d trades\n" "$date" "$day_sign" "$(echo "$day_pnl" | tr -d '-')" "$day_trades"
done

echo ""
echo "=== summary ==="
printf "days:           %d (win: %d  loss: %d  flat: %d)\n" "$day_count" "$win_days" "$loss_days" "$flat_days"
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

# return on capital
if [[ "$day_count" -gt 0 ]]; then
    roc=$(echo "scale=2; $total_pnl * 100 / $CAPITAL" | bc -l)
    roc_sign="+"; abs_roc=$(echo "$roc" | tr -d '-')
    if (( $(echo "$roc < 0" | bc -l) )); then roc_sign="-"; fi
    printf "return on cap:  %s%s%%\n" "$roc_sign" "$abs_roc"
fi

echo ""
echo "=== rubric ==="
# score each metric on a 1-5 scale
# P&L thresholds scaled ~5x vs 20-day script (99 days vs 20)

score_total=0
metric_count=0

# 1. total P&L score (weight 3x) — scaled for ~100 days
if (( $(echo "$total_pnl >= 10000" | bc -l) )); then pnl_score=5
elif (( $(echo "$total_pnl >= 2500" | bc -l) )); then pnl_score=4
elif (( $(echo "$total_pnl >= 0" | bc -l) )); then pnl_score=3
elif (( $(echo "$total_pnl >= -10000" | bc -l) )); then pnl_score=2
else pnl_score=1; fi
score_total=$(( score_total + pnl_score * 3 ))
metric_count=$(( metric_count + 3 ))
printf "  total P&L:          %d/5  (>10k=5, >2.5k=4, >0=3, >-10k=2, else=1)\n" "$pnl_score"

# 2. win day rate (weight 2x)
if [[ "$day_count" -gt 0 ]]; then
    wr=$(echo "scale=1; $win_days * 100 / $day_count" | bc -l)
    if (( $(echo "$wr >= 60" | bc -l) )); then wr_score=5
    elif (( $(echo "$wr >= 50" | bc -l) )); then wr_score=4
    elif (( $(echo "$wr >= 40" | bc -l) )); then wr_score=3
    elif (( $(echo "$wr >= 30" | bc -l) )); then wr_score=2
    else wr_score=1; fi
else wr_score=1; fi
score_total=$(( score_total + wr_score * 2 ))
metric_count=$(( metric_count + 2 ))
printf "  win day rate:       %d/5  (>60%%=5, >50%%=4, >40%%=3, >30%%=2, else=1)\n" "$wr_score"

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

# 4. trade frequency (weight 1x) — avg trades/day
if [[ "$day_count" -gt 0 ]]; then
    at=$(echo "scale=1; $total_trades / $day_count" | bc -l)
    if (( $(echo "$at >= 2 && $at <= 10" | bc -l) )); then tf_score=5
    elif (( $(echo "$at >= 1 && $at <= 15" | bc -l) )); then tf_score=4
    elif (( $(echo "$at >= 0.5 && $at <= 20" | bc -l) )); then tf_score=3
    elif (( $(echo "$at <= 30" | bc -l) )); then tf_score=2
    else tf_score=1; fi
else tf_score=1; fi
score_total=$(( score_total + tf_score ))
metric_count=$(( metric_count + 1 ))
printf "  trade frequency:    %d/5  (2-10=5, 1-15=4, 0.5-20=3, <30=2, else=1)\n" "$tf_score"

# 5. tail risk (weight 2x) — biggest single-day loss as %% of capital
if [[ -n "$biggest_loss_date" ]]; then
    abs_bl=$(echo "$biggest_loss" | tr -d '-')
    bl_pct=$(echo "scale=1; $abs_bl * 100 / $CAPITAL" | bc -l)
    if (( $(echo "$bl_pct <= 5" | bc -l) )); then tr_score=5
    elif (( $(echo "$bl_pct <= 10" | bc -l) )); then tr_score=4
    elif (( $(echo "$bl_pct <= 20" | bc -l) )); then tr_score=3
    elif (( $(echo "$bl_pct <= 40" | bc -l) )); then tr_score=2
    else tr_score=1; fi
else tr_score=5; fi
score_total=$(( score_total + tr_score * 2 ))
metric_count=$(( metric_count + 2 ))
printf "  tail risk:          %d/5  (max loss <5%%cap=5, <10%%=4, <20%%=3, <40%%=2, else=1)\n" "$tr_score"

# 6. win/loss asymmetry (weight 1x) — avg win day / avg loss day
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

# composite score
if [[ "$metric_count" -gt 0 ]]; then
    max_score=$(( metric_count * 5 ))
    composite=$(echo "scale=1; $score_total * 10 / $max_score" | bc -l)
    printf "\n  COMPOSITE SCORE:    %s / 10.0\n" "$composite"
fi

# data quality warnings
if [[ "$rate_limit_warnings" -gt 0 ]] || [[ "$data_quality_warnings" -gt 0 ]]; then
    echo ""
    echo "=== DATA QUALITY WARNING ==="
    if [[ "$rate_limit_warnings" -gt 0 ]]; then
        printf "  rate limit retries detected on %d/%d days — results may be unreliable\n" "$rate_limit_warnings" "$day_count"
    fi
    if [[ "$data_quality_warnings" -gt 0 ]]; then
        printf "  low candle counts on %d/%d days — possible data gaps or rate limiting\n" "$data_quality_warnings" "$day_count"
    fi
    echo "  consider re-running with delays between dates or reducing parallelism"
fi
