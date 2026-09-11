#!/usr/bin/env bash
# the pre-monday v12 validation sweep. sequential (alpaca rate limit). ~6h total.
# order is by value: the promoted config on 2026 (out-of-sample vs the v11 tuning
# years) first, then 2025, then the variants that answer the open questions.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
R="$ROOT/scripts/backtest_range.sh"
LOG="${SWEEP_LOG:-$ROOT/data/sweep_progress.log}"
Y26_START=2026-01-02; Y26_END=2026-09-10
Y25_START=2025-01-02; Y25_END=2025-12-31
{
  echo "sweep started $(date -Is)"
  "$R" v12          $Y26_START $Y26_END
  "$R" v12          $Y25_START $Y25_END
  "$R" v12_fullday  $Y26_START $Y26_END --no-new-entries-after 15:30 --force-exit-by 15:55
  "$R" v12_5pct     $Y26_START $Y26_END --sizing-fraction 0.05
  "$R" v12_avoid30  $Y26_START $Y26_END --avoid-first-minutes 30
  "$R" v12_core4    $Y26_START $Y26_END --tickers SPY,QQQ,AAPL,MSFT
  "$R" v12_fullday  $Y25_START $Y25_END --no-new-entries-after 15:30 --force-exit-by 15:55
  echo "sweep finished $(date -Is)"
} >> "$LOG" 2>&1
