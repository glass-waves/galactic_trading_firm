#!/usr/bin/env bash
# psql against the trading database, whichever way works on this host:
# a native psql with DATABASE_URL, or psql inside the postgres container (podman/docker).
# usage: ./scripts/psql.sh [psql args...]      e.g. ./scripts/psql.sh -tA -c "select 1"
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
set -a; [[ -f "$ROOT/.env" ]] && source "$ROOT/.env"; set +a
if command -v psql >/dev/null 2>&1 && [[ -n "${DATABASE_URL:-}" ]]; then
    exec psql "$DATABASE_URL" -v ON_ERROR_STOP=1 "$@"
fi
CONTAINER="${TRADING_PG_CONTAINER:-galactic_trading_firm-postgres-1}"
# podman's docker shim prints an "Emulate Docker CLI" banner on stderr; drop it, keep real errors
docker exec -i "$CONTAINER" psql -U postgres -d adaptive_trading -v ON_ERROR_STOP=1 "$@" \
    2> >(grep -v "Emulate Docker CLI" >&2)
