#!/usr/bin/env bash
# sweep a single parameter across values, backtest each on one year
#
# usage:
#   ./scripts/sweep_params.sh --param "--w4-1h-min" --values "0.20 0.25 0.30 0.35 0.40" --year 2024
#   ./scripts/sweep_params.sh --param "--exit-threshold" --values "-0.05 -0.10 -0.15 -0.20" --year 2024
#
# prerequisites: entry windows flags are always included automatically.
# output: comparison table + individual CSV files in data/

set -euo pipefail

PARAM=""
VALUES=""
YEAR="2024"
BASE_EXTRA="--use-entry-windows --add-momentum-persistence 0.05 --add-candle-pattern 0.05"
TICKER_OVERRIDE="AAPL:entry_threshold=0.35"
SLIPPAGE="3.0"
HALF_SPREAD="0.005"
CAPITAL="10000"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --param) PARAM="$2"; shift 2 ;;
        --values) VALUES="$2"; shift 2 ;;
        --year) YEAR="$2"; shift 2 ;;
        --extra) BASE_EXTRA="$BASE_EXTRA $2"; shift 2 ;;
        *) echo "unknown arg: $1"; exit 1 ;;
    esac
done

if [[ -z "$PARAM" ]] || [[ -z "$VALUES" ]]; then
    echo "usage: $0 --param \"--w4-1h-min\" --values \"0.20 0.25 0.30 0.35 0.40\" [--year 2024]"
    exit 1
fi

# clean param name for file tags (e.g., "--w4-1h-min" → "w4_1h_min")
PARAM_TAG=$(echo "$PARAM" | sed 's/^--//; s/-/_/g')

mkdir -p data logs

echo "=== parameter sweep ==="
echo "param: $PARAM"
echo "values: $VALUES"
echo "year: $YEAR"
echo ""

RESULTS_FILE=$(mktemp)

for val in $VALUES; do
    tag="sweep_${PARAM_TAG}_${val}_${YEAR}"
    # clean tag (replace dots and negatives for filenames)
    tag=$(echo "$tag" | sed 's/\./_/g; s/-/n/g')

    outfile="data/${tag}_trades.csv"
    logfile="logs/${tag}_csv.log"

    echo -n "  $PARAM $val ... "

    ./scripts/backtest_year.sh "$YEAR" \
        --slippage-bps "$SLIPPAGE" --half-spread "$HALF_SPREAD" \
        --ticker-override "$TICKER_OVERRIDE" \
        $BASE_EXTRA \
        $PARAM "$val" \
        > "$outfile" 2> "$logfile"

    # extract stats
    read -r trades pnl wins losses gw gl <<< "$(awk -F, '
        $1=="trade" {
            t++; p+=$10
            if($10>0.001){w++; gw+=$10}
            if($10<-0.001){l++; gl+=(-$10)}
        } END {
            printf "%d %.2f %d %d %.2f %.2f", t, p, w, l, gw, gl
        }' "$outfile")"

    pf="—"
    if (( $(echo "$gl > 0" | bc -l) )); then
        pf=$(printf "%.2f" "$(echo "$gw / $gl" | bc -l)")
    fi
    wr="0.0"
    if [[ "$trades" -gt 0 ]]; then
        wr=$(printf "%.1f" "$(echo "$wins * 100 / $trades" | bc -l)")
    fi
    avg="0.00"
    if [[ "$trades" -gt 0 ]]; then
        avg=$(printf "%.2f" "$(echo "$pnl / $trades" | bc -l)")
    fi

    echo "$trades trades, \$$pnl, ${wr}% win, PF $pf, \$$avg/t"
    echo "$val $trades $pnl $wr $pf $avg" >> "$RESULTS_FILE"
done

# print comparison table
echo ""
echo "=== sweep results: $PARAM ($YEAR) ==="
printf "%-8s  %5s  %8s  %5s  %5s  %7s\n" "value" "trades" "P&L" "win%" "PF" "avg/t"
while read -r val trades pnl wr pf avg; do
    printf "%-8s  %5s  \$%7s  %4s%%  %5s  \$%5s\n" "$val" "$trades" "$pnl" "$wr" "$pf" "$avg"
done < "$RESULTS_FILE"

rm -f "$RESULTS_FILE"
echo ""
echo "CSV files: data/sweep_${PARAM_TAG}_*_${YEAR}_trades.csv"
