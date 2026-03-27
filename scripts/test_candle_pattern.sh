#!/usr/bin/env bash
# candle pattern indicator A/B testing
# tests engulfing pattern with confluence/reversion modes
#
# phases:
#   1: diagnostic — single-date verbose comparison (does the pattern change trades?)
#   2: feature isolation — reversion × confluence factorial at weight 0.25
#   3: weight sweep — 0.05, 0.15, 0.25, 0.40 on best variant from phase 2
#
# usage: ./scripts/test_candle_pattern.sh [extra-args...]

set -euo pipefail

DATES=(
    2025-03-17 2025-04-07 2025-04-28 2025-05-19 2025-06-09
    2025-06-30 2025-07-21 2025-08-11 2025-08-25 2025-09-15
    2025-10-06 2025-10-27 2025-11-17 2025-12-08 2025-12-29
    2026-01-12 2026-01-27 2026-02-10 2026-02-24 2026-03-02
)

DIAGNOSTIC_DATE="2025-10-15"
CAPITAL=10000
LOOKBACK=3
COST_ARGS="--slippage-bps 2.0 --half-spread 0.005"
EXTRA_ARGS=("$@")

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
        output=$($BACKTEST --date "$date" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS $extra_args ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1) || true

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

echo "=============================================="
echo "=== candle pattern A/B testing (20-day IS) ==="
echo "=============================================="
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days"
echo ""

echo "--- PHASE 1: diagnostic (single date ${DIAGNOSTIC_DATE}) ---"
echo ""
echo "  baseline trades:"
$BACKTEST --date "$DIAGNOSTIC_DATE" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS --verbose ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1 | grep -E "(#[0-9]|total)" | head -20
echo ""
echo "  with candle pattern (w=0.25, full model):"
$BACKTEST --date "$DIAGNOSTIC_DATE" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS --verbose --add-candle-pattern 0.25 ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1 | grep -E "(#[0-9]|total)" | head -20
echo ""
echo "  with candle pattern (w=0.40, full model):"
$BACKTEST --date "$DIAGNOSTIC_DATE" --capital "$CAPITAL" --lookback-days "$LOOKBACK" $COST_ARGS --verbose --add-candle-pattern 0.40 ${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"} 2>&1 | grep -E "(#[0-9]|total)" | head -20
echo ""

echo "======================================================="
echo "=== PHASE 2: feature isolation (reversion x confluence) ==="
echo "======================================================="
echo ""

echo "--- baseline ---"
run_sweep "baseline (no candle pattern)"
echo ""

echo "--- engulfing variants at weight 0.25 ---"
run_sweep "A: raw (no confluence, no reversion)"       --add-candle-pattern 0.25 --candle-pattern-no-confluence --candle-pattern-no-reversion
run_sweep "B: +confluence (no reversion)"               --add-candle-pattern 0.25 --candle-pattern-no-reversion
run_sweep "C: +reversion (no confluence)"               --add-candle-pattern 0.25 --candle-pattern-no-confluence
run_sweep "D: full model (confluence + reversion)"      --add-candle-pattern 0.25
echo ""

echo "================================="
echo "=== PHASE 3: weight sweep ==="
echo "================================="
echo ""

echo "--- full model at different weights ---"
run_sweep "full model w=0.05"  --add-candle-pattern 0.05
run_sweep "full model w=0.10"  --add-candle-pattern 0.10
run_sweep "full model w=0.15"  --add-candle-pattern 0.15
run_sweep "full model w=0.25"  --add-candle-pattern 0.25
run_sweep "full model w=0.40"  --add-candle-pattern 0.40
echo ""

echo "--- +confluence only (no reversion) at different weights ---"
run_sweep "+confluence w=0.05"  --add-candle-pattern 0.05 --candle-pattern-no-reversion
run_sweep "+confluence w=0.10"  --add-candle-pattern 0.10 --candle-pattern-no-reversion
run_sweep "+confluence w=0.15"  --add-candle-pattern 0.15 --candle-pattern-no-reversion
run_sweep "+confluence w=0.25"  --add-candle-pattern 0.25 --candle-pattern-no-reversion
run_sweep "+confluence w=0.40"  --add-candle-pattern 0.40 --candle-pattern-no-reversion
echo ""

echo "done. review phase 2 to identify best mode, phase 3 for best weight."
echo "if winner found, validate on 100-day and 2022 bear:"
echo "  ./scripts/backtest_100days.sh --add-candle-pattern <weight> [mode flags]"
echo "  ./scripts/backtest_2022bear.sh --add-candle-pattern <weight> [mode flags]"
