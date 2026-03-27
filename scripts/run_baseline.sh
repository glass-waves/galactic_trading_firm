#!/usr/bin/env bash
# gold standard 4-year baseline test (2022-2025)
#
# runs backtest_year.sh on all 4 years with pessimistic realistic costs.
# produces per-trade CSV files in data/ and prints a summary comparison table.
# this is THE standard test for validating any config change.
#
# usage:
#   ./scripts/run_baseline.sh                           # run all 4 years
#   ./scripts/run_baseline.sh --tag v2                  # tag output files (data/v2_2022_trades.csv)
#   ./scripts/run_baseline.sh --extra "--entry-threshold 0.45"  # forward args to backtest
#   ./scripts/run_baseline.sh --parallel                # run 2 years at a time (faster, more API pressure)
#
# output:
#   data/<tag>_<year>_trades.csv   — per-trade CSV (primary data)
#   logs/<tag>_<year>_csv.log      — progress log (stderr)

set -euo pipefail

# defaults
TAG="baseline"
SLIPPAGE="3.0"
HALF_SPREAD="0.005"
CAPITAL="10000"
PARALLEL=false
EXTRA_ARGS=""
TICKER_OVERRIDE="AAPL:entry_threshold=0.35"

# parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --tag) TAG="$2"; shift 2 ;;
        --slippage-bps) SLIPPAGE="$2"; shift 2 ;;
        --half-spread) HALF_SPREAD="$2"; shift 2 ;;
        --capital) CAPITAL="$2"; shift 2 ;;
        --parallel) PARALLEL=true; shift ;;
        --no-ticker-override) TICKER_OVERRIDE=""; shift ;;
        --extra) EXTRA_ARGS="$2"; shift 2 ;;
        *) echo "unknown arg: $1"; exit 1 ;;
    esac
done

YEARS=(2022 2023 2024 2025)
COST_ARGS="--slippage-bps $SLIPPAGE --half-spread $HALF_SPREAD"

mkdir -p data logs

echo "=== 4-year baseline test ==="
echo "tag: $TAG"
echo "capital: \$$CAPITAL  costs: $COST_ARGS"
if [[ -n "$TICKER_OVERRIDE" ]]; then
    echo "ticker override: $TICKER_OVERRIDE"
fi
if [[ -n "$EXTRA_ARGS" ]]; then
    echo "extra args: $EXTRA_ARGS"
fi
echo "parallel: $PARALLEL"
echo ""

STARTED=$(date "+%Y-%m-%d %H:%M:%S")

run_year() {
    local year=$1
    local outfile="data/${TAG}_${year}_trades.csv"
    local logfile="logs/${TAG}_${year}_csv.log"

    local args="$COST_ARGS"
    if [[ -n "$TICKER_OVERRIDE" ]]; then
        args="$args --ticker-override $TICKER_OVERRIDE"
    fi
    if [[ -n "$EXTRA_ARGS" ]]; then
        args="$args $EXTRA_ARGS"
    fi

    ./scripts/backtest_year.sh "$year" $args > "$outfile" 2> "$logfile"
    echo "  $year done → $outfile"
}

if $PARALLEL; then
    # wave 1: 2022 + 2023
    echo "wave 1: 2022 + 2023..."
    run_year 2022 &
    run_year 2023 &
    wait
    # wave 2: 2024 + 2025
    echo "wave 2: 2024 + 2025..."
    run_year 2024 &
    run_year 2025 &
    wait
else
    for year in "${YEARS[@]}"; do
        echo "running $year..."
        run_year "$year"
    done
fi

ENDED=$(date "+%Y-%m-%d %H:%M:%S")

# === summary table ===
echo ""
echo "=== 4-year summary ($TAG) ==="
printf "%-6s  %8s  %5s  %6s  %6s  %6s  %6s\n" "year" "P&L" "PF" "trades" "win%" "W/L" "avg\$/t"

grand_pnl=0
grand_trades=0
grand_wins=0
grand_losses=0
grand_gross_win=0
grand_gross_loss=0

for year in "${YEARS[@]}"; do
    f="data/${TAG}_${year}_trades.csv"
    if [[ ! -f "$f" ]] || ! grep -q "^trade," "$f" 2>/dev/null; then
        printf "%-6s  %8s\n" "$year" "NO DATA"
        continue
    fi

    total=$(grep -c "^trade," "$f")
    wins=$(grep "^trade," "$f" | awk -F',' '$10+0>0.001{n++} END {print n+0}')
    losses=$(grep "^trade," "$f" | awk -F',' '$10+0<-0.001{n++} END {print n+0}')
    cum_pnl=$(grep "^trade," "$f" | awk -F',' '{sum+=$10} END {printf "%.2f", sum}')
    gw=$(grep "^trade," "$f" | awk -F',' '$10+0>0{sum+=$10} END {printf "%.2f", sum}')
    gl=$(grep "^trade," "$f" | awk -F',' '$10+0<0{sum+=(-$10)} END {printf "%.2f", sum}')

    pf="n/a"
    if (( $(echo "$gl > 0" | bc -l) )); then
        pf=$(echo "scale=2; $gw / $gl" | bc -l)
    elif (( $(echo "$gw > 0" | bc -l) )); then
        pf="inf"
    fi

    wr="0.0"
    if [[ "$total" -gt 0 ]]; then
        wr=$(echo "scale=1; $wins * 100 / $total" | bc -l)
    fi

    wl_ratio="n/a"
    if [[ "$wins" -gt 0 ]] && [[ "$losses" -gt 0 ]]; then
        avg_w=$(echo "scale=4; $gw / $wins" | bc -l)
        avg_l=$(echo "scale=4; $gl / $losses" | bc -l)
        if (( $(echo "$avg_l > 0" | bc -l) )); then
            wl_ratio=$(echo "scale=2; $avg_w / $avg_l" | bc -l)
        fi
    fi

    avg_per_trade="0.00"
    if [[ "$total" -gt 0 ]]; then
        avg_per_trade=$(echo "scale=2; $cum_pnl / $total" | bc -l)
    fi

    sign="+"; if (( $(echo "$cum_pnl < 0" | bc -l) )); then sign=""; fi
    printf "%-6s  %s\$%-6s  %5s  %6d  %5s%%  %5s  \$%s\n" "$year" "$sign" "$cum_pnl" "$pf" "$total" "$wr" "$wl_ratio" "$avg_per_trade"

    grand_pnl=$(echo "$grand_pnl + $cum_pnl" | bc -l)
    grand_trades=$(( grand_trades + total ))
    grand_wins=$(( grand_wins + wins ))
    grand_losses=$(( grand_losses + losses ))
    grand_gross_win=$(echo "$grand_gross_win + $gw" | bc -l)
    grand_gross_loss=$(echo "$grand_gross_loss + $gl" | bc -l)
done

# grand total row
echo "------  --------  -----  ------  ------  ------  ------"
grand_pf="n/a"
if (( $(echo "$grand_gross_loss > 0" | bc -l) )); then
    grand_pf=$(echo "scale=2; $grand_gross_win / $grand_gross_loss" | bc -l)
fi
grand_wr="0.0"
if [[ "$grand_trades" -gt 0 ]]; then
    grand_wr=$(echo "scale=1; $grand_wins * 100 / $grand_trades" | bc -l)
fi
grand_wl="n/a"
if [[ "$grand_wins" -gt 0 ]] && [[ "$grand_losses" -gt 0 ]]; then
    gaw=$(echo "scale=4; $grand_gross_win / $grand_wins" | bc -l)
    gal=$(echo "scale=4; $grand_gross_loss / $grand_losses" | bc -l)
    if (( $(echo "$gal > 0" | bc -l) )); then
        grand_wl=$(echo "scale=2; $gaw / $gal" | bc -l)
    fi
fi
grand_avg="0.00"
if [[ "$grand_trades" -gt 0 ]]; then
    grand_avg=$(echo "scale=2; $grand_pnl / $grand_trades" | bc -l)
fi
sign="+"; if (( $(echo "$grand_pnl < 0" | bc -l) )); then sign=""; fi
printf "%-6s  %s\$%-6s  %5s  %6d  %5s%%  %5s  \$%s\n" "TOTAL" "$sign" "$(printf '%.2f' "$grand_pnl")" "$grand_pf" "$grand_trades" "$grand_wr" "$grand_wl" "$grand_avg"

# per-ticker breakdown
echo ""
echo "=== per-ticker ($TAG) ==="
printf "%-6s  %8s  %6s  %6s\n" "ticker" "P&L" "trades" "win%"
for t in SPY QQQ AAPL MSFT NVDA; do
    t_pnl=0; t_trades=0; t_wins=0
    for year in "${YEARS[@]}"; do
        f="data/${TAG}_${year}_trades.csv"
        [[ -f "$f" ]] || continue
        t_pnl=$(grep "^trade," "$f" | awk -F',' -v tk="$t" '$3==tk{sum+=$10} END {printf "%.2f", sum+'"$t_pnl"'}')
        t_trades=$(( t_trades + $(grep "^trade," "$f" | awk -F',' -v tk="$t" '$3==tk{n++} END {print n+0}') ))
        t_wins=$(( t_wins + $(grep "^trade," "$f" | awk -F',' -v tk="$t" '$3==tk && $10+0>0.001{n++} END {print n+0}') ))
    done
    t_wr="0.0"
    if [[ "$t_trades" -gt 0 ]]; then
        t_wr=$(echo "scale=1; $t_wins * 100 / $t_trades" | bc -l)
    fi
    sign="+"; if (( $(echo "$t_pnl < 0" | bc -l) )); then sign=""; fi
    printf "%-6s  %s\$%-6s  %6d  %5s%%\n" "$t" "$sign" "$t_pnl" "$t_trades" "$t_wr"
done

echo ""
echo "started: $STARTED"
echo "completed: $ENDED"
echo "CSV files: data/${TAG}_202{2,3,4,5}_trades.csv"
