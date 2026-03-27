#!/usr/bin/env bash
# ARCHIVED: pre-bug-fix script. "v89" references are pre-sizing-fix config versions.
# see docs/pre-bug-fix.md for context. current tuning uses entry windows.
#
# phase D: combine phase A/C winners and validate
# tests combinations on 20-day set first, then validates on 100-day + 2022 bear
# usage: ./scripts/test_combinations.sh

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

    printf "  %-52s  P&L: %s\$%-10s  trades: %-4d  PF: %-5s  win%%: %5s%%  avg/day: %4s  worst: -\$%s\n" \
        "$label" "$sign" "$(printf '%.2f' "$abs_pnl")" "$total_trades" "$pf" "$win_rate" "$avg_trades" "$(printf '%.2f' "$abs_bl")"
}

echo "=== phase D: combination testing (20-day sweep) ==="
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days"
echo ""

echo "--- reference ---"
run_sweep "baseline (v89)"
echo ""

echo "--- individual winners (reference) ---"
run_sweep "A1: avoid_first_minutes=30"  --avoid-first-minutes 30
run_sweep "A6: exit_threshold=-0.05"    --exit-threshold -0.05
run_sweep "C2: OFI weight=0.05"        --add-ofi 0.05
run_sweep "C2: OFI weight=0.10"        --add-ofi 0.10
echo ""

echo "--- two-way combinations ---"
run_sweep "A1+A6: avoid30 + exit-0.05"  --avoid-first-minutes 30 --exit-threshold -0.05
run_sweep "A1+C2: avoid30 + OFI=0.05"   --avoid-first-minutes 30 --add-ofi 0.05
run_sweep "A1+C2: avoid30 + OFI=0.10"   --avoid-first-minutes 30 --add-ofi 0.10
run_sweep "A6+C2: exit-0.05 + OFI=0.05" --exit-threshold -0.05 --add-ofi 0.05
run_sweep "A6+C2: exit-0.05 + OFI=0.10" --exit-threshold -0.05 --add-ofi 0.10
echo ""

echo "--- three-way combinations ---"
run_sweep "A1+A6+C2(0.05): all three"   --avoid-first-minutes 30 --exit-threshold -0.05 --add-ofi 0.05
run_sweep "A1+A6+C2(0.10): all three"   --avoid-first-minutes 30 --exit-threshold -0.05 --add-ofi 0.10
echo ""

echo "--- three-way + profit extension ---"
run_sweep "all + profit_ext 45m (0.05)"  --avoid-first-minutes 30 --exit-threshold -0.05 --add-ofi 0.05 --profit-extension-ms 2700000
run_sweep "all + profit_ext 45m (0.10)"  --avoid-first-minutes 30 --exit-threshold -0.05 --add-ofi 0.10 --profit-extension-ms 2700000
echo ""

echo "done."
