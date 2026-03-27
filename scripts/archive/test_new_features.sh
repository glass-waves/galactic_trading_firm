#!/usr/bin/env bash
# test new features one at a time against the 20-day set
# compares each variant to the baseline (current config, no overrides)
# usage: ./scripts/test_new_features.sh

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

    printf "  %-40s  P&L: %s\$%-10s  trades: %-4d  PF: %-5s  win%%: %5s%%  avg/day: %4s  worst: -\$%s\n" \
        "$label" "$sign" "$(printf '%.2f' "$abs_pnl")" "$total_trades" "$pf" "$win_rate" "$avg_trades" "$(printf '%.2f' "$abs_bl")"
}

echo "=== new feature A/B testing (20-day sweep) ==="
echo "capital: \$${CAPITAL}  lookback: ${LOOKBACK} days"
echo ""

echo "--- baseline ---"
run_sweep "baseline (current config)"
echo ""

echo "--- test 1: entry cooldown ---"
run_sweep "cooldown 30s"  --entry-cooldown-ms 30000
run_sweep "cooldown 60s"  --entry-cooldown-ms 60000
run_sweep "cooldown 120s" --entry-cooldown-ms 120000
echo ""

echo "--- test 2: adaptive max hold ---"
run_sweep "adaptive hold (+30m win, -15m loss)" --profit-extension-ms 1800000 --loss-reduction-ms 900000
run_sweep "adaptive hold (+15m win, -15m loss)" --profit-extension-ms 900000 --loss-reduction-ms 900000
run_sweep "adaptive hold (+30m win, -30m loss)" --profit-extension-ms 1800000 --loss-reduction-ms 1800000
echo ""

echo "--- test 3: score-scaled sizing ---"
run_sweep "score-scaled sizing (2-8%)"  --score-scaled-sizing
echo ""

echo "--- test 4: relative volume indicator ---"
run_sweep "rvol 5min weight=0.10" --add-rvol 0.10
run_sweep "rvol 5min weight=0.15" --add-rvol 0.15
run_sweep "rvol 5min weight=0.20" --add-rvol 0.20
echo ""

echo "--- test 5: daily loss circuit breaker ---"
run_sweep "max daily loss 5%"  --max-daily-loss-pct 0.05
run_sweep "max daily loss 10%" --max-daily-loss-pct 0.10
run_sweep "max daily loss 15%" --max-daily-loss-pct 0.15
echo ""

echo "done."
