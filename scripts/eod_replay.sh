#!/usr/bin/env bash
# same-day backtest of the promoted config on the IEX cache, exactly as the eod-review check-in
# needs it — one leading token, no command substitution, so it runs under the check-in allow-list.
# usage: ./scripts/eod_replay.sh [YYYY-MM-DD]   (default: today in US/Eastern)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
D="${1:-$(TZ=America/New_York date +%F)}"
[[ "$D" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || { echo "usage: $0 [YYYY-MM-DD]" >&2; exit 2; }
exec ./target/release/backtest --date "$D" --lookback-days 8 --capital "${INITIAL_CAPITAL:-10000}" \
  --slippage-bps 3.0 --half-spread 0.005 --verbose \
  --bars-dir data/bars_iex --cross-index SPY --cross-lag 1
