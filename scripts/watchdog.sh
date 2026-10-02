#!/usr/bin/env bash
# no-LLM safety watchdog for the paper trader. runs every few minutes from a systemd timer
# during the session; reads engine_state + systemd + trades + one Alpaca snapshot and pushes
# ntfy alerts via scripts/notify.sh. each distinct alert is sent once per day (state file),
# then re-sent only if it escalates. exit 0 always (the timer must not go into failed state).
#
# books (since 2026-09-26): the trader hosts a primary book plus shadow books. position /
# P&L / late-position checks look at book='primary' only; heartbeat and feed staleness use any
# row (all books share the feed); an enabled shadow with no engine_state rows after 09:40 ET
# is a warning ("not hosted"). abnormal-move alert: one snapshot request per run for the
# primary's tickers + SPY (docs/plans/2026-09-26_pipeline_and_books.md §6).
#
# env: MOVE_ALERT_NAME_PCT (2.0) / MOVE_ALERT_SPY_PCT (1.0) — % vs today's open;
#      MOVE_ALERT_NAME_HIGH_PCT (2.0) / MOVE_ALERT_SPY_HIGH_PCT (1.5) — % off the intraday high;
#      DRY_RUN=1 — ignore the session clock and the service check, print alerts instead of
#      sending them, use a throw-away state file, accept a stale snapshot (weekend testing).
#
# thresholds mirror .claude/skills/intraday-review/SKILL.md §1.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
set -a; [[ -f .env ]] && source .env; set +a
CAPITAL="${INITIAL_CAPITAL:-10000}"
DRY_RUN="${DRY_RUN:-0}"
STATE_DIR="$ROOT/logs/watchdog"; mkdir -p "$STATE_DIR"
TODAY="$(TZ=America/New_York date +%F)"
STATE="$STATE_DIR/$TODAY.sent"
if [[ "$DRY_RUN" == "1" ]]; then STATE="$(mktemp)"; trap 'rm -f "$STATE"' EXIT; fi
touch "$STATE"
HM="$(TZ=America/New_York date +%H%M)"; DOW="$(TZ=America/New_York date +%u)"
PSQL="$ROOT/scripts/psql.sh"

# outside weekdays 09:25–16:08 ET there is nothing to watch
if [[ "$DRY_RUN" != "1" ]] && { (( DOW > 5 )) || (( 10#$HM < 925 )) || (( 10#$HM > 1608 )); }; then exit 0; fi

alert() {  # level key message
    local level="$1" key="$2" msg="$3"
    if ! grep -qx "$level:$key" "$STATE"; then
        echo "$level:$key" >> "$STATE"
        if [[ "$DRY_RUN" == "1" ]]; then echo "[dry-run] $level $key: $msg"
        else "$ROOT/scripts/notify.sh" "$level" "$msg" >/dev/null; fi
    fi
}

# 1. service alive?
if ! systemctl --user is-active --quiet paper-trader.service; then
    alert critical service_down "paper-trader.service is NOT active at $HM ET (systemctl --user status paper-trader)"
    [[ "$DRY_RUN" == "1" ]] || exit 0
fi

# 2a. heartbeat + feed staleness: per ticker, freshest row across books (all books share the feed)
HB_ROWS="$("$PSQL" -At -F '|' -c "
SELECT ticker,
       min(EXTRACT(EPOCH FROM now()-updated_at))::int,
       COALESCE(min(EXTRACT(EPOCH FROM now()-last_bar_at))::int, 99999),
       bool_and(feed_stale)
FROM engine_state GROUP BY ticker ORDER BY ticker" 2>/dev/null)"
if [[ -z "$HB_ROWS" ]]; then
    alert critical no_state "engine_state is empty or postgres unreachable at $HM ET"
    exit 0
fi
STALE_ALL=1
while IFS='|' read -r t hb bar stale; do
    [[ -z "$t" ]] && continue
    if (( hb > 360 )); then alert critical "hb_$t" "$t heartbeat ${hb}s old at $HM ET — trader hung?"
    elif (( hb > 180 )); then alert warning "hb_$t" "$t heartbeat ${hb}s old at $HM ET"; fi
    if [[ "$stale" == "t" ]]; then alert warning "stale_$t" "$t feed_stale at $HM ET (last bar ${bar}s ago)"; else STALE_ALL=0; fi
done <<< "$HB_ROWS"
if (( STALE_ALL == 1 )); then alert critical stale_all "ALL tickers feed_stale at $HM ET"; fi

# 2b. the primary book: breaker, late positions, daily P&L (book-level, same on every row)
ROWS="$("$PSQL" -At -F '|' -c "
SELECT ticker, loss_breaker_active, COALESCE(position_direction::text,''), COALESCE(position_hold_ms,0)/60000
FROM engine_state WHERE book='primary' ORDER BY ticker" 2>/dev/null)"
if [[ -z "$ROWS" ]]; then
    alert critical no_primary "no engine_state rows for book='primary' at $HM ET (primary not hosted?)"
fi
PRIMARY_TICKERS=""
while IFS='|' read -r t brk pos hold; do
    [[ -z "$t" ]] && continue
    PRIMARY_TICKERS="${PRIMARY_TICKERS:+$PRIMARY_TICKERS,}$t"
    if [[ "$brk" == "t" ]]; then alert warning "breaker_$t" "$t daily loss breaker active at $HM ET"; fi
    if [[ -n "$pos" ]] && (( 10#$HM > 1601 )); then alert critical "late_$t" "$t still holds a $pos position at $HM ET (force-exit missed)"; fi
done <<< "$ROWS"
PNL="$("$PSQL" -At -c "SELECT COALESCE(min(daily_pnl),0) FROM engine_state WHERE book='primary'" 2>/dev/null)"
if [[ "$PNL" =~ ^-?[0-9.]+$ ]]; then
    LIM3=$(python3 -c "print(-0.03*$CAPITAL)"); LIM5=$(python3 -c "print(-0.05*$CAPITAL)")
    if python3 -c "import sys; sys.exit(0 if $PNL < $LIM5 else 1)"; then alert critical dd5 "primary daily P&L $PNL < 5% of capital at $HM ET"
    elif python3 -c "import sys; sys.exit(0 if $PNL < $LIM3 else 1)"; then alert warning dd3 "primary daily P&L $PNL < 3% of capital at $HM ET"; fi
fi

# 2c. shadow books that the running trader is not hosting (10 min after the open)
if (( 10#$HM >= 940 )) || [[ "$DRY_RUN" == "1" ]]; then
    NOT_HOSTED="$("$PSQL" -At -c "
SELECT b.name FROM books b
WHERE b.enabled AND b.role='shadow'
  AND NOT EXISTS (SELECT 1 FROM engine_state e WHERE e.book=b.name) ORDER BY b.name" 2>/dev/null)"
    while read -r b; do
        [[ -z "$b" ]] && continue
        alert warning "nothosted_$b" "book $b not hosted (build failed or restart pending) at $HM ET"
    done <<< "$NOT_HOSTED"
fi

# 3. churn (primary only: shadows write source='shadow')
N="$("$PSQL" -At -c "SELECT count(*) FROM trades WHERE source='paper' AND (exit_fill_at AT TIME ZONE 'America/New_York')::date = (now() AT TIME ZONE 'America/New_York')::date" 2>/dev/null)"
if [[ "${N:-0}" =~ ^[0-9]+$ ]] && (( N > 4 )); then alert warning churn "$N paper trades already today at $HM ET (v17 averages 0.4/day)"; fi

# 4. did the last LLM check-in fail? (json line with is_error true or an auth/billing message)
for job in preopen-check eod-review intraday-review; do
    f="$ROOT/logs/checkins/$job.jsonl"
    [[ -f "$f" ]] || continue
    if [[ "$(find "$f" -mmin -30 2>/dev/null)" ]] && tail -1 "$f" | grep -qiE '"is_error":true|credit balance|not logged in|rate limit'; then
        alert warning "checkin_$job" "$job check-in failed: $(tail -1 "$f" | grep -oiE 'credit balance[^"]*|not logged in|rate limit[^"]*|is_error":true' | head -1)"
    fi
done

# 5. abnormal move: one snapshot request for the primary's tickers + SPY, keys from .env
book_context() {  # <sym> -> what the primary is doing on it and whether any shadow is in a trade there
    local sym="$1"
    if [[ "$sym" == "SPY" ]]; then
        "$PSQL" -At -c "
SELECT 'primary: ' || COALESCE((SELECT string_agg(ticker||' '||position_direction, ', ') FROM engine_state WHERE book='primary' AND position_direction IS NOT NULL), 'flat')
    || ', daily P&L ' || (SELECT round(COALESCE(min(daily_pnl),0)::numeric,2) FROM engine_state WHERE book='primary')
    || '; shadows in a trade: ' || COALESCE((SELECT string_agg(DISTINCT book, ', ') FROM engine_state WHERE book<>'primary' AND position_direction IS NOT NULL), 'none')" 2>/dev/null
    else
        "$PSQL" -At -c "
SELECT 'primary: ' || COALESCE((SELECT CASE WHEN position_direction IS NULL THEN 'flat'
                                            ELSE position_direction||' from '||round(position_entry_price::numeric,2)||' (unrealized '||round(COALESCE(position_unrealized_pnl,0)::numeric,2)||')' END
                               FROM engine_state WHERE book='primary' AND ticker='$sym'), 'does not trade it')
    || ', daily P&L ' || (SELECT round(COALESCE(min(daily_pnl),0)::numeric,2) FROM engine_state WHERE book='primary')
    || '; shadows in a trade: ' || COALESCE((SELECT string_agg(book||' '||position_direction, ', ') FROM engine_state WHERE book<>'primary' AND ticker='$sym' AND position_direction IS NOT NULL), 'none')" 2>/dev/null
    fi
}
if [[ -n "${APCA_API_KEY_ID:-}" && -n "${APCA_API_SECRET_KEY:-}" && -n "$PRIMARY_TICKERS" ]]; then
    SYMS="$PRIMARY_TICKERS,SPY"
    SNAP="$(curl -s -m 10 "https://data.alpaca.markets/v2/stocks/snapshots?symbols=$SYMS&feed=iex" \
        -H "APCA-API-KEY-ID: $APCA_API_KEY_ID" -H "APCA-API-SECRET-KEY: $APCA_API_SECRET_KEY" 2>/dev/null)"
    # the script comes in on stdin, so the JSON travels in the environment
    MOVES="$(SNAP="$SNAP" python3 - "$TODAY" "${MOVE_ALERT_NAME_PCT:-2.0}" "${MOVE_ALERT_SPY_PCT:-1.0}" \
            "${MOVE_ALERT_NAME_HIGH_PCT:-2.0}" "${MOVE_ALERT_SPY_HIGH_PCT:-1.5}" "$DRY_RUN" <<'PY'
import sys, json, os
today, npct, spct, nhigh, shigh, dry = sys.argv[1:7]
npct, spct, nhigh, shigh = map(float, (npct, spct, nhigh, shigh))
try:
    d = json.loads(os.environ.get("SNAP", ""))
except Exception as e:
    print(f"warning|snapshot_failed|-|abnormal-move check: snapshot not parseable ({e})"); sys.exit(0)
if not isinstance(d, dict) or "message" in d:
    print(f"warning|snapshot_failed|-|abnormal-move check: snapshot request failed ({str(d)[:80]})"); sys.exit(0)
for sym in sorted(d):
    s = d[sym]
    if not isinstance(s, dict): continue
    db = s.get("dailyBar") or {}
    o, h = db.get("o"), db.get("h")
    last = (s.get("latestTrade") or {}).get("p") or (s.get("minuteBar") or {}).get("c") or db.get("c")
    if not o or not h or not last: continue
    if not str(db.get("t", "")).startswith(today) and dry != "1": continue  # yesterday's bar: pre-open or stale
    po, ph = (last - o) / o * 100, (last - h) / h * 100
    if dry == "1": print(f"# {sym}: open {o} high {h} last {last} -> {po:+.2f}% vs open, {ph:+.2f}% off high", file=sys.stderr)
    if sym == "SPY":
        if po <= -spct: print(f"critical|move_SPY_open_down|SPY|SPY {po:+.2f}% vs today's open ({last} vs {o})")
        elif po >= spct: print(f"info|move_SPY_open_up|SPY|SPY {po:+.2f}% vs today's open ({last} vs {o})")
        if ph <= -shigh: print(f"critical|move_SPY_high_down|SPY|SPY {ph:+.2f}% off its intraday high ({last} vs {h})")
    else:
        if po <= -npct: print(f"warning|move_{sym}_open_down|{sym}|{sym} {po:+.2f}% vs today's open ({last} vs {o})")
        elif po >= npct: print(f"info|move_{sym}_open_up|{sym}|{sym} {po:+.2f}% vs today's open ({last} vs {o})")
        if ph <= -nhigh: print(f"warning|move_{sym}_high_down|{sym}|{sym} {ph:+.2f}% off its intraday high ({last} vs {h})")
PY
)"
    # fd 3: book_context runs psql (docker exec -i), which would eat the loop's stdin
    while IFS='|' read -r -u 3 level key sym text; do
        [[ -z "$level" ]] && continue
        if [[ "$sym" == "-" ]]; then alert "$level" "$key" "$text at $HM ET"; continue; fi
        alert "$level" "$key" "$text at $HM ET — $(book_context "$sym" </dev/null)"
    done 3<<< "$MOVES"
fi
exit 0
