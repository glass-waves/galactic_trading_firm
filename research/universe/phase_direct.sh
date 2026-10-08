#!/usr/bin/env bash
set -uo pipefail
cd /home/dylmet/Projects/galactic_trading_firm
set -a; source .env; set +a
LOG=logs/universe_direct.log
: > "$LOG"
BIN=target/release/backtest
BARS=data/bars_iex
FETCH_END=2026-10-07

declare -A IDS=(
  [LLY]=12 [COST]=13 [HD]=14 [XOM]=15 [CVX]=16 [ADBE]=17 [CRM]=18 [ORCL]=19
  [QCOM]=20 [CAT]=21 [GS]=22 [PG]=23 [ABBV]=24 [MRK]=25 [TXN]=26 [AMAT]=27
  [QQQ]=30 [SMH]=31 [XLK]=32 [XLF]=33 [IWM]=34 [XLE]=35 [TSLA]=36 [AVGO]=37
  [NFLX]=38 [COIN]=39 [PLTR]=40 [MU]=41 [SHOP]=42 [UBER]=43
)
ORDER=(LLY COST HD XOM CVX ADBE CRM ORCL QCOM CAT GS PG ABBV MRK TXN AMAT QQQ SMH XLK XLF IWM XLE TSLA AVGO NFLX COIN PLTR MU SHOP UBER)

for T in "${ORDER[@]}"; do
  ID=${IDS[$T]}
  TAG="cand_${ID}"
  echo "=== $T (id $ID) $(date) ===" >> "$LOG"

  if [[ ! -s "$BARS/$T.csv" ]]; then
    echo "-- fetching $T bars" >> "$LOG"
    (
      exec 9>logs/.research.lock
      flock -w 14400 9 || { echo "lock timeout fetching $T" >> "$LOG"; exit 1; }
      "$BIN" --fetch-bars "$BARS" --start 2022-01-01 --end "$FETCH_END" --tickers "$T" --feed iex
    ) >> "$LOG" 2>&1
  else
    echo "-- $T bars already cached" >> "$LOG"
  fi

  if [[ ! -s "data/${TAG}_2026_trades.csv" ]]; then
    echo "-- sweeping $TAG" >> "$LOG"
    (
      exec 9>logs/.research.lock
      flock -w 14400 9 || { echo "lock timeout sweeping $T" >> "$LOG"; exit 1; }
      BARS_DIR="$BARS" scripts/run_cached_sweep.sh "$TAG" --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --tickers "$T"
    ) >> "$LOG" 2>&1
  else
    echo "-- $TAG sweep already on disk" >> "$LOG"
  fi

  scripts/psql.sh -q -c "UPDATE pipeline_candidates SET backtest_tag='${TAG}' WHERE id=${ID} AND backtest_tag IS DISTINCT FROM '${TAG}';" >> "$LOG" 2>&1

  tries=0
  while :; do
    tries=$((tries+1))
    out=$(python3 scripts/pipeline/pipeline.py backtest "ticker:$T" 2>&1)
    echo "$out" >> "$LOG"
    if echo "$out" | grep -q "could not take the pipeline lock"; then
      if (( tries > 40 )); then echo "!!! giving up on pipeline lock for $T" >> "$LOG"; break; fi
      sleep 5; continue
    fi
    break
  done
  echo "--- done $T $(date) tries=$tries ---" >> "$LOG"
done
echo "=== DIRECT DRIVER ALL DONE $(date) ===" >> "$LOG"
