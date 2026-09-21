#!/usr/bin/env bash
# no-LLM safety watchdog for the paper trader. runs every few minutes from a systemd timer
# during the session; reads engine_state + systemd + trades and pushes ntfy alerts via
# scripts/notify.sh. each distinct alert is sent once per day (state file), then re-sent only
# if it escalates. exit 0 always (the timer must not go into failed state).
#
# thresholds mirror .claude/skills/intraday-review/SKILL.md §1.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
set -a; [[ -f .env ]] && source .env; set +a
CAPITAL="${INITIAL_CAPITAL:-10000}"
STATE_DIR="$ROOT/logs/watchdog"; mkdir -p "$STATE_DIR"
TODAY="$(TZ=America/New_York date +%F)"
STATE="$STATE_DIR/$TODAY.sent"; touch "$STATE"
HM="$(TZ=America/New_York date +%H%M)"; DOW="$(TZ=America/New_York date +%u)"

# outside weekdays 09:25–13:15 ET there is nothing to watch
if (( DOW > 5 )) || (( 10#$HM < 925 )) || (( 10#$HM > 1615 )); then exit 0; fi

alert() {  # level key message
    local level="$1" key="$2" msg="$3"
    if ! grep -qx "$level:$key" "$STATE"; then
        echo "$level:$key" >> "$STATE"
        "$ROOT/scripts/notify.sh" "$level" "$msg" >/dev/null
    fi
}

# 1. service alive?
if ! systemctl --user is-active --quiet paper-trader.service; then
    alert critical service_down "paper-trader.service is NOT active at $HM ET (systemctl --user status paper-trader)"
    exit 0
fi

# 2. engine_state per ticker
ROWS="$("$ROOT/scripts/psql.sh" -At -F '|' -c "
SELECT ticker,
       EXTRACT(EPOCH FROM now()-updated_at)::int,
       COALESCE(EXTRACT(EPOCH FROM now()-last_bar_at)::int, 99999),
       feed_stale, loss_breaker_active,
       COALESCE(position_direction::text,''), COALESCE(position_hold_ms,0)/60000,
       COALESCE(daily_pnl,0)
FROM engine_state ORDER BY ticker" 2>/dev/null)"
if [[ -z "$ROWS" ]]; then
    alert critical no_state "engine_state is empty or postgres unreachable at $HM ET"
    exit 0
fi

STALE_ALL=1
while IFS='|' read -r t hb bar stale brk pos hold pnl; do
    [[ -z "$t" ]] && continue
    if (( hb > 360 )); then alert critical "hb_$t" "$t heartbeat ${hb}s old at $HM ET — trader hung?"
    elif (( hb > 180 )); then alert warning "hb_$t" "$t heartbeat ${hb}s old at $HM ET"; fi
    if [[ "$stale" == "t" ]]; then alert warning "stale_$t" "$t feed_stale at $HM ET (last bar ${bar}s ago)"; else STALE_ALL=0; fi
    if [[ "$brk" == "t" ]]; then alert warning "breaker_$t" "$t daily loss breaker active at $HM ET"; fi
    if [[ -n "$pos" ]] && (( 10#$HM > 1601 )); then alert critical "late_$t" "$t still holds a $pos position at $HM ET (force-exit missed)"; fi
    # daily pnl is the same on every row; evaluate once
    if [[ "$t" == "AAPL" || "$LAST_PNL_CHECKED" != "1" ]]; then
        LAST_PNL_CHECKED=1
        LIM3=$(python3 -c "print(-0.03*$CAPITAL)"); LIM5=$(python3 -c "print(-0.05*$CAPITAL)")
        if python3 -c "import sys; sys.exit(0 if $pnl < $LIM5 else 1)"; then alert critical dd5 "daily P&L $pnl < 5% of capital at $HM ET"
        elif python3 -c "import sys; sys.exit(0 if $pnl < $LIM3 else 1)"; then alert warning dd3 "daily P&L $pnl < 3% of capital at $HM ET"; fi
    fi
done <<< "$ROWS"
if (( STALE_ALL == 1 )); then alert critical stale_all "ALL tickers feed_stale at $HM ET"; fi

# 3. churn
N="$("$ROOT/scripts/psql.sh" -At -c "SELECT count(*) FROM trades WHERE source='paper' AND (exit_fill_at AT TIME ZONE 'America/New_York')::date = (now() AT TIME ZONE 'America/New_York')::date" 2>/dev/null)"
if [[ "${N:-0}" =~ ^[0-9]+$ ]] && (( N > 4 )); then alert warning churn "$N paper trades already today at $HM ET (v17 averages 0.4/day)"; fi

# 4. did the last LLM check-in fail? (json line with is_error true or an auth/billing message)
for job in preopen-check eod-review intraday-review; do
    f="$ROOT/logs/checkins/$job.jsonl"
    [[ -f "$f" ]] || continue
    if [[ "$(find "$f" -mmin -30 2>/dev/null)" ]] && tail -1 "$f" | grep -qiE '"is_error":true|credit balance|not logged in|rate limit'; then
        alert warning "checkin_$job" "$job check-in failed: $(tail -1 "$f" | grep -oiE 'credit balance[^"]*|not logged in|rate limit[^"]*|is_error":true' | head -1)"
    fi
done
exit 0
