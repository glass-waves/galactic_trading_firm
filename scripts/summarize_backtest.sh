#!/usr/bin/env bash
# summarize backtest CSV results
#
# usage:
#   ./scripts/summarize_backtest.sh <tag>                    # summarize data/<tag>_202{2,3,4,5}_trades.csv
#   ./scripts/summarize_backtest.sh <tag> --years 2024 2025  # specific years only
#   ./scripts/summarize_backtest.sh --compare <tag1> <tag2>  # side-by-side comparison
#
# examples:
#   ./scripts/summarize_backtest.sh w1_5m_thrust
#   ./scripts/summarize_backtest.sh --compare baseline w1_5m_thrust
#   ./scripts/summarize_backtest.sh w2_aligned --years 2025

set -euo pipefail

# --- helpers ---

summarize_tag() {
    local tag="$1"
    shift
    local years=("$@")

    echo "=== $tag ==="
    printf "%-6s  %5s  %8s  %5s  %5s  %7s  %5s\n" "year" "trades" "P&L" "win%" "PF" "avg/t" "W/L"

    local grand_t=0 grand_p=0 grand_w=0 grand_l=0 grand_gw=0 grand_gl=0

    for year in "${years[@]}"; do
        local f="data/${tag}_${year}_trades.csv"
        if [[ ! -f "$f" ]] || ! grep -q "^trade," "$f" 2>/dev/null; then
            printf "%-6s  %5s\n" "$year" "—"
            continue
        fi

        read -r t p w l gw gl <<< "$(awk -F, '
            $1=="trade" {
                t++; p+=$10
                if($10>0.001){w++; gw+=$10}
                if($10<-0.001){l++; gl+=(-$10)}
            } END {
                printf "%d %.2f %d %d %.2f %.2f", t, p, w, l, gw, gl
            }' "$f")"

        local pf="—"
        if (( $(echo "$gl > 0" | bc -l) )); then
            pf=$(printf "%.2f" "$(echo "$gw / $gl" | bc -l)")
        elif (( $(echo "$gw > 0" | bc -l) )); then
            pf="inf"
        fi

        local wr="0.0"
        if [[ "$t" -gt 0 ]]; then
            wr=$(printf "%.1f" "$(echo "$w * 100 / $t" | bc -l)")
        fi

        local avg="0.00"
        if [[ "$t" -gt 0 ]]; then
            avg=$(printf "%.2f" "$(echo "$p / $t" | bc -l)")
        fi

        local wl="—"
        if [[ "$w" -gt 0 ]] && [[ "$l" -gt 0 ]]; then
            local aw al
            aw=$(echo "scale=4; $gw / $w" | bc -l)
            al=$(echo "scale=4; $gl / $l" | bc -l)
            if (( $(echo "$al > 0" | bc -l) )); then
                wl=$(printf "%.2f" "$(echo "$aw / $al" | bc -l)")
            fi
        fi

        printf "%-6s  %5d  \$%7s  %4s%%  %5s  \$%5s  %5s\n" "$year" "$t" "$p" "$wr" "$pf" "$avg" "$wl"

        grand_t=$(( grand_t + t ))
        grand_p=$(echo "$grand_p + $p" | bc -l)
        grand_w=$(( grand_w + w ))
        grand_l=$(( grand_l + l ))
        grand_gw=$(echo "$grand_gw + $gw" | bc -l)
        grand_gl=$(echo "$grand_gl + $gl" | bc -l)
    done

    # total row
    if [[ "${#years[@]}" -gt 1 ]]; then
        echo "------  -----  --------  -----  -----  -------  -----"
        local gpf="—"
        if (( $(echo "$grand_gl > 0" | bc -l) )); then
            gpf=$(printf "%.2f" "$(echo "$grand_gw / $grand_gl" | bc -l)")
        fi
        local gwr="0.0"
        if [[ "$grand_t" -gt 0 ]]; then
            gwr=$(printf "%.1f" "$(echo "$grand_w * 100 / $grand_t" | bc -l)")
        fi
        local gavg="0.00"
        if [[ "$grand_t" -gt 0 ]]; then
            gavg=$(printf "%.2f" "$(echo "$grand_p / $grand_t" | bc -l)")
        fi
        local gwl="—"
        if [[ "$grand_w" -gt 0 ]] && [[ "$grand_l" -gt 0 ]]; then
            local gaw gal
            gaw=$(echo "scale=4; $grand_gw / $grand_w" | bc -l)
            gal=$(echo "scale=4; $grand_gl / $grand_l" | bc -l)
            if (( $(echo "$gal > 0" | bc -l) )); then
                gwl=$(printf "%.2f" "$(echo "$gaw / $gal" | bc -l)")
            fi
        fi
        printf "%-6s  %5d  \$%7s  %4s%%  %5s  \$%5s  %5s\n" "TOTAL" "$grand_t" \
            "$(printf '%.2f' "$grand_p")" "$gwr" "$gpf" "$gavg" "$gwl"
    fi

    # per-ticker breakdown
    echo ""
    printf "%-6s  %8s  %5s  %5s\n" "ticker" "P&L" "trades" "win%"
    for tk in SPY QQQ AAPL MSFT NVDA; do
        local tk_p=0 tk_t=0 tk_w=0
        for year in "${years[@]}"; do
            local f="data/${tag}_${year}_trades.csv"
            [[ -f "$f" ]] || continue
            read -r yp yt yw <<< "$(awk -F, -v tk="$tk" '
                $1=="trade" && $3==tk {
                    t++; p+=$10
                    if($10>0.001) w++
                } END {
                    printf "%.2f %d %d", p, t, w
                }' "$f")"
            tk_p=$(echo "$tk_p + $yp" | bc -l)
            tk_t=$(( tk_t + yt ))
            tk_w=$(( tk_w + yw ))
        done
        local tkwr="0.0"
        if [[ "$tk_t" -gt 0 ]]; then
            tkwr=$(printf "%.1f" "$(echo "$tk_w * 100 / $tk_t" | bc -l)")
        fi
        printf "%-6s  \$%7s  %5d  %4s%%\n" "$tk" "$(printf '%.2f' "$tk_p")" "$tk_t" "$tkwr"
    done

    # exit reason breakdown
    echo ""
    printf "%-18s  %5s  %5s\n" "exit reason" "count" "pct"
    local tmpfile
    tmpfile=$(mktemp)
    for year in "${years[@]}"; do
        local f="data/${tag}_${year}_trades.csv"
        [[ -f "$f" ]] || continue
        awk -F, '$1=="trade" {print $13}' "$f" >> "$tmpfile"
    done
    local total_exits
    total_exits=$(wc -l < "$tmpfile" | tr -d ' ')
    if [[ "$total_exits" -gt 0 ]]; then
        sort "$tmpfile" | uniq -c | sort -rn | while read -r cnt reason; do
            local pct
            pct=$(printf "%.1f" "$(echo "$cnt * 100 / $total_exits" | bc -l)")
            printf "%-18s  %5d  %4s%%\n" "$reason" "$cnt" "$pct"
        done
    fi
    rm -f "$tmpfile"
}

# --- main ---

COMPARE=false
TAGS=()
YEARS=(2022 2023 2024 2025)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --compare) COMPARE=true; shift ;;
        --years) shift; YEARS=(); while [[ $# -gt 0 ]] && [[ "$1" != --* ]]; do YEARS+=("$1"); shift; done ;;
        *) TAGS+=("$1"); shift ;;
    esac
done

if [[ ${#TAGS[@]} -eq 0 ]]; then
    echo "usage: $0 <tag> [--years YYYY ...]"
    echo "       $0 --compare <tag1> <tag2> [--years YYYY ...]"
    exit 1
fi

for tag in "${TAGS[@]}"; do
    summarize_tag "$tag" "${YEARS[@]}"
    echo ""
done
