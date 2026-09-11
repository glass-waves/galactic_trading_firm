#!/usr/bin/env bash
# notify a human. usage: ./scripts/notify.sh <level: info|warning|critical> "<message>"
# sends to ntfy.sh if NOTIFY_TOPIC is set in .env (install the ntfy app and subscribe to
# that topic), always appends to logs/notifications.log, and always prints.
set -uo pipefail
LEVEL="${1:-info}"; shift || true
MSG="${*:-}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
set -a; [[ -f "$ROOT/.env" ]] && source "$ROOT/.env"; set +a
mkdir -p "$ROOT/logs"
STAMP="$(date -Is)"
echo "$STAMP [$LEVEL] $MSG" | tee -a "$ROOT/logs/notifications.log"
if [[ -n "${NOTIFY_TOPIC:-}" ]]; then
    PRIO=3; [[ "$LEVEL" == "warning" ]] && PRIO=4; [[ "$LEVEL" == "critical" ]] && PRIO=5
    curl -s -o /dev/null -H "Title: paper trader [$LEVEL]" -H "Priority: $PRIO" \
        -d "$MSG" "https://ntfy.sh/$NOTIFY_TOPIC" || echo "ntfy send failed" >&2
fi
