#!/usr/bin/env bash
# ARCHIVED: pre-bug-fix script. "v89" was the config version before sizing fix.
# see docs/pre-bug-fix.md for context. current tuning uses entry windows.
#
# phase 4 optimization: systematic micro-tuning of v89
# tests each parameter variant independently against the 20-day set
# usage: ./scripts/test_optimization.sh
#
# phases:
#   A: low-hanging parameter refinements (session, thresholds, adaptive hold)
#   B: indicator weight micro-tuning (5min, 1hr, timescale allocation)
#   C: structural enhancements (agreement, OFI, dynamic fusion)

set -euo pipefail

DATES=(
    2025-03-17 2025-04-07 2025-04-28 2025-05-19 2025-06-09
    2025-06-30 2025-07-21 2025-08-11 2025-08-25 2025-09-15
    2025-10-06 2025-10-27 2025-11-17 2025-12-08 2025-12-29
    2026-01-12 2026-01-27 2026-02-10 2026-02-24 2026-03-02
)

CAPITAL=10000
LOOKBACK=3
COST_ARGS="--slippage-bps 2.0 --half-spread 0.005"

# build once in release mode
echo "building backtest binary..."
cargo build -p backtest --release 2>&1 | tail -1
BACKTEST="./target/release/backtest"
echo ""

run_sweep() {
    local label="$1"
    shift
    local extra_args="$*"

    local total_pnl=0
    local total_trades=0
    local win_days=0
    local loss_days=0
    local day_count=0
    local gross_wins=0
    local gross_losses=0
    local biggest_loss=0

    for date in "${DATES[@]}"; do
        output=$($BACKTEST --date "$date" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS $extra_args 2>&1) || true

        total_line=$(echo "$output" | grep -E "^\s+total\s" || true)

        if [[ -z "$total_line" ]]; then
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
        elif (( $(echo "$day_pnl < -0.001" | bc -l) )); then
            loss_days=$(( loss_days + 1 ))
            abs_loss=$(echo "$day_pnl" | tr -d '-')
            gross_losses=$(echo "$gross_losses + $abs_loss" | bc -l)
            if (( $(echo "$day_pnl < $biggest_loss" | bc -l) )); then
                biggest_loss="$day_pnl"
            fi
        fi
    done

    # compute metrics
    local pf="n/a"
    if (( $(echo "$gross_losses > 0" | bc -l) )); then
        pf=$(echo "scale=2; $gross_wins / $gross_losses" | bc -l)
    elif (( $(echo "$gross_wins > 0" | bc -l) )); then
        pf="inf"
    fi

    local win_rate="0"
    local active_days=$(( win_days + loss_days ))
    if [[ "$active_days" -gt 0 ]]; then
        win_rate=$(echo "scale=1; $win_days * 100 / $active_days" | bc -l)
    fi

    local avg_trades="0"
    if [[ "$day_count" -gt 0 ]]; then
        avg_trades=$(echo "scale=1; $total_trades / $day_count" | bc -l)
    fi

    local sign="+"; local abs_pnl="$total_pnl"
    if (( $(echo "$total_pnl < 0" | bc -l) )); then sign="-"; abs_pnl=$(echo "$total_pnl" | tr -d '-'); fi

    local abs_bl="0"
    if (( $(echo "$biggest_loss < 0" | bc -l) )); then
        abs_bl=$(echo "$biggest_loss" | tr -d '-')
    fi

    printf "  %-48s  P&L: %s\$%-10s  trades: %-4d  PF: %-5s  win%%: %5s%%  avg/day: %4s  worst: -\$%s\n" \
        "$label" "$sign" "$(printf '%.2f' "$abs_pnl")" "$total_trades" "$pf" "$win_rate" "$avg_trades" "$(printf '%.2f' "$abs_bl")"
}

echo "=== phase 4 optimization A/B testing (20-day sweep) ==="
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days"
echo ""

echo "--- baseline (v89 current config) ---"
run_sweep "baseline (v89)"
echo ""

echo "========================================"
echo "=== PHASE A: parameter refinements ==="
echo "========================================"
echo ""

echo "--- A1: avoid_first_minutes (currently 60) ---"
run_sweep "avoid_first_minutes=30"  --avoid-first-minutes 30
run_sweep "avoid_first_minutes=45"  --avoid-first-minutes 45
run_sweep "avoid_first_minutes=15"  --avoid-first-minutes 15
echo ""

echo "--- A2: max_concurrent_positions (currently 1) ---"
run_sweep "max_concurrent_positions=2"  --max-concurrent-positions 2
echo ""

echo "--- A3: loss_reduction_ms (currently 900000 = 15m) ---"
run_sweep "loss_reduction 20m"  --loss-reduction-ms 1200000
run_sweep "loss_reduction 30m"  --loss-reduction-ms 1800000
echo ""

echo "--- A4: profit_extension_ms (currently 1800000 = 30m) ---"
run_sweep "profit_extension 45m"  --profit-extension-ms 2700000
run_sweep "profit_extension 60m"  --profit-extension-ms 3600000
echo ""

echo "--- A5: entry_threshold (currently 0.58) ---"
run_sweep "entry_threshold=0.55"  --entry-threshold 0.55
run_sweep "entry_threshold=0.56"  --entry-threshold 0.56
run_sweep "entry_threshold=0.57"  --entry-threshold 0.57
run_sweep "entry_threshold=0.60"  --entry-threshold 0.60
echo ""

echo "--- A6: exit_threshold (currently -0.15) ---"
run_sweep "exit_threshold=-0.10"  --exit-threshold -0.10
run_sweep "exit_threshold=-0.20"  --exit-threshold -0.20
run_sweep "exit_threshold=-0.25"  --exit-threshold -0.25
run_sweep "exit_threshold=-0.05"  --exit-threshold -0.05
echo ""

echo "========================================"
echo "=== PHASE B: indicator weight tuning ==="
echo "========================================"
echo ""

echo "--- B1: 5min MACD weight (currently 0.40) ---"
run_sweep "5min MACD=0.35 RSI=0.15"  --indicator-weight macd_5min=0.35 --indicator-weight rsi_14_5min=0.15
run_sweep "5min MACD=0.45 RSI=0.05"  --indicator-weight macd_5min=0.45 --indicator-weight rsi_14_5min=0.05
echo ""

echo "--- B2: 5min EMA weight (currently 0.30) ---"
run_sweep "5min EMA=0.25 StochRSI=0.20"  --indicator-weight ema_20_5min=0.25 --indicator-weight stoch_rsi_5min=0.20
run_sweep "5min EMA=0.35 StochRSI=0.10"  --indicator-weight ema_20_5min=0.35 --indicator-weight stoch_rsi_5min=0.10
echo ""

echo "--- B3: 1hr SuperTrend weight (currently 0.30) ---"
run_sweep "1hr ST=0.25 ADX=0.25"  --indicator-weight supertrend_1hr=0.25 --indicator-weight adx_14_1hr=0.25
run_sweep "1hr ST=0.35 ADX=0.15"  --indicator-weight supertrend_1hr=0.35 --indicator-weight adx_14_1hr=0.15
echo ""

echo "--- B4: timescale weights (currently 10/60/30) ---"
run_sweep "TS weights 10/55/35"  --ts-weight-1m 0.10 --ts-weight-5m 0.55 --ts-weight-1h 0.35
run_sweep "TS weights 15/55/30"  --ts-weight-1m 0.15 --ts-weight-5m 0.55 --ts-weight-1h 0.30
run_sweep "TS weights 5/65/30"   --ts-weight-1m 0.05 --ts-weight-5m 0.65 --ts-weight-1h 0.30
run_sweep "TS weights 10/50/40"  --ts-weight-1m 0.10 --ts-weight-5m 0.50 --ts-weight-1h 0.40
echo ""

echo "============================================"
echo "=== PHASE C: structural enhancements ==="
echo "============================================"
echo ""

echo "--- C1: cross-timescale agreement (currently disabled) ---"
run_sweep "agreement exp=0.5"  --enable-agreement --agreement-exponent 0.5
run_sweep "agreement exp=1.0"  --enable-agreement --agreement-exponent 1.0
run_sweep "agreement exp=0.3"  --enable-agreement --agreement-exponent 0.3
echo ""

echo "--- C2: OFI indicator on 5-min ---"
run_sweep "OFI weight=0.10"  --add-ofi 0.10
run_sweep "OFI weight=0.05"  --add-ofi 0.05
run_sweep "OFI weight=0.15"  --add-ofi 0.15
echo ""

echo "--- C3: dynamic fusion ---"
run_sweep "dynamic fusion (default)"  --enable-dynamic-fusion
run_sweep "dynamic fusion adj=0.10"   --enable-dynamic-fusion --fusion-adjustment 0.10
run_sweep "dynamic fusion adj=0.20"   --enable-dynamic-fusion --fusion-adjustment 0.20
echo ""

echo "done. review results above and combine winners for phase D validation."
