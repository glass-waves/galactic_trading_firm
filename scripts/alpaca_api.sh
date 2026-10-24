#!/usr/bin/env bash
# read-only GET against the Alpaca paper trading API, keys from .env. one leading token so the
# scheduled check-ins can call it under their allow-list instead of inlining `source .env && curl`.
# usage: ./scripts/alpaca_api.sh /v2/positions   |   ./scripts/alpaca_api.sh "/v2/orders?status=open"
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
P="${1:-}"
if [[ -z "$P" || "$P" != /v2/* ]]; then echo "usage: $0 /v2/<path>  (GET only)" >&2; exit 2; fi
set -a; [[ -f "$ROOT/.env" ]] && source "$ROOT/.env"; set +a
: "${APCA_API_KEY_ID:?missing in .env}" "${APCA_API_SECRET_KEY:?missing in .env}"
curl -sS --max-time 20 -H "APCA-API-KEY-ID: $APCA_API_KEY_ID" -H "APCA-API-SECRET-KEY: $APCA_API_SECRET_KEY" \
  "https://paper-api.alpaca.markets$P"
echo
