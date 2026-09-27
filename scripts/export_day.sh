#!/usr/bin/env bash
# export one trading day's live state to data/live/<YYYY-MM-DD>/ as plain JSON/CSV so an
# off-machine agent (a claude routine on a git checkout) can write the end-of-day report.
# usage: scripts/export_day.sh [YYYY-MM-DD]   (default: today, America/New_York)
# env (testing only): LIVE_DIR=<dir> writes under <dir>/<date> instead of data/live/<date>;
#                     EXPORT_NO_GIT=1 skips the commit/push.
# books (since 2026-09-26): the per-ticker files describe the primary book; books.json,
# shadow_trades.json and replay_<book>.csv cover every enabled book (docs/pipeline.md).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
set -a; source .env; set +a
D="${1:-$(TZ=America/New_York date +%F)}"
LIVE_DIR="${LIVE_DIR:-data/live}"
OUT="$LIVE_DIR/$D"; mkdir -p "$OUT"
Q() { ./scripts/psql.sh -At -c "$1" </dev/null; }

Q "select json_agg(row_to_json(t)) from (select ticker, updated_at, last_bar_at, feed_stale, loss_breaker_active, config_version_id, pending_config_version_id, position_direction, position_entry_price, position_hold_ms, entry_blocked_by, near_miss, daily_pnl, process_started_at from engine_state where book='primary' order by ticker) t" > "$OUT/engine_state.json"
Q "select json_agg(row_to_json(t)) from (select ticker, direction, entry_reason, exit_reason, entry_fill_at, exit_fill_at, entry_price, exit_price, broker_entry_price, broker_exit_price, position_size, pnl_dollars, pnl_percent, hold_duration_ms, entry_score_composite, exit_score_composite, config_version_id from trades where source='paper' and (exit_fill_at at time zone 'America/New_York')::date = '$D' order by exit_fill_at) t" > "$OUT/trades.json"
Q "select json_agg(row_to_json(t)) from (select ts, ticker, kind, reason, composite from entry_block_events where book='primary' and (ts at time zone 'America/New_York')::date = '$D' order by ts) t" > "$OUT/block_events.json"
Q "select json_agg(row_to_json(t)) from (select (exit_fill_at at time zone 'America/New_York')::date as day, count(*) as trades, round(sum(pnl_dollars)::numeric,2) as pnl from trades where source='paper' and exit_fill_at > now() - interval '30 days' group by 1 order by 1) t" > "$OUT/recent_days.json"
Q "select json_agg(row_to_json(t)) from (select id, status, promoted_at, mutation_reason, config_blob->>'config_id' as config_id from config_versions order by id desc limit 5) t" > "$OUT/config_versions.json"
Q "select json_agg(row_to_json(t)) from (select created_at, agent, memo_type, reasoning, flags from agent_memos where created_at > now() - interval '7 days' order by created_at) t" > "$OUT/memos.json"

# books: every book hosted (or retired today), the day's trades / P&L / open positions at close,
# and the trial-to-date tally since the book was created. shadow trades carry source='shadow'.
Q "select json_agg(row_to_json(t)) from (
  select b.name as book, b.role, b.enabled, b.config_version_id, b.tickers, b.capital, b.purpose, b.candidate_id,
         b.created_at, b.retired_at, b.retire_reason,
         (select max(e.config_version_id) from engine_state e where e.book=b.name) as running_config_version_id,
         (select count(*) from engine_state e where e.book=b.name) as hosted_tickers,
         (select count(*) from trades tr where tr.book=b.name and tr.source in ('paper','shadow') and (tr.exit_fill_at at time zone 'America/New_York')::date = '$D') as trades,
         (select round(coalesce(sum(pnl_dollars),0)::numeric,2) from trades tr where tr.book=b.name and tr.source in ('paper','shadow') and (tr.exit_fill_at at time zone 'America/New_York')::date = '$D') as pnl,
         (select json_agg(json_build_object('ticker', e.ticker, 'direction', e.position_direction, 'entry_price', e.position_entry_price, 'unrealized_pnl', e.position_unrealized_pnl, 'opened_at', e.position_opened_at))
            from engine_state e where e.book=b.name and e.position_direction is not null) as open_positions,
         (select count(*) from book_sessions s where s.book=b.name) as sessions_to_date,
         (select count(*) from trades tr where tr.book=b.name and tr.source in ('paper','shadow') and tr.exit_fill_at >= b.created_at) as trades_to_date,
         (select round(coalesce(sum(pnl_dollars),0)::numeric,2) from trades tr where tr.book=b.name and tr.source in ('paper','shadow') and tr.exit_fill_at >= b.created_at) as pnl_to_date
  from books b where b.enabled or (b.retired_at at time zone 'America/New_York')::date = '$D'
  order by b.role, b.name) t" > "$OUT/books.json"
Q "select json_agg(row_to_json(t)) from (select book, ticker, direction, entry_reason, exit_reason, entry_fill_at, exit_fill_at, entry_price, exit_price, broker_entry_price, broker_exit_price, position_size, pnl_dollars, pnl_percent, hold_duration_ms, entry_score_composite, exit_score_composite, config_version_id from trades where source='shadow' and (exit_fill_at at time zone 'America/New_York')::date = '$D' order by book, exit_fill_at) t" > "$OUT/shadow_trades.json"

# systemd + alerts + check-in outputs
{
  echo "{\"exported_at\":\"$(date -Is)\",\"service_active\":\"$(systemctl --user is-active paper-trader.service)\","
  echo " \"service\":$(systemctl --user show paper-trader.service -p ActiveEnterTimestamp -p NRestarts -p Result | python3 -c 'import sys,json; print(json.dumps(dict(l.split("=",1) for l in sys.stdin.read().splitlines() if "=" in l)))'),"
  echo " \"timers\":$(systemctl --user list-timers --no-pager | grep -E 'paper-trader|preopen|eod|watchdog' | awk '{print $1" "$2" "$3" "$NF}' | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read().strip().split("\n")))')}"
} > "$OUT/system.json"
grep "^$(date -d "$D" +%Y-%m-%d)" logs/notifications.log > "$OUT/notifications.log" 2>/dev/null || true
for j in preopen-check watchdog; do [[ -f logs/checkins/$j.jsonl ]] && tail -1 logs/checkins/$j.jsonl > "$OUT/$j.jsonl" || true; done
journalctl --user -u paper-trader.service --since "$D 00:00" --until "$D 23:59" --no-pager -o cat 2>/dev/null | grep -iE 'WARN|ERROR|reconnect|shutdown|starting|config' | grep -v sqlx | tail -40 > "$OUT/journal_excerpt.log" || true

# same-day bars from the bar cache (free plan: bars available ~16 min after the close).
# IEX: the union of every enabled book's tickers (+ SPY), so trial tickers accumulate history.
UNION="$(Q "select string_agg(t, ',' order by t) from (
  select distinct unnest(coalesce(b.tickers,
           (select array(select jsonb_array_elements_text(cv.config_blob->'tickers')) from config_versions cv
             where cv.id = coalesce(b.config_version_id, (select max(id) from config_versions where status='promoted'))))) as t
  from books b where b.enabled
  union select 'SPY') s" || true)"
UNION="${UNION:-AAPL,AMZN,NVDA,MSFT,SPY}"
./target/release/backtest --fetch-bars data/bars --start "$D" --end "$D" --tickers AAPL,AMZN,NVDA,MSFT,SPY >/dev/null 2>&1 || true
./target/release/backtest --fetch-bars data/bars_iex --start "$D" --end "$D" --tickers "$UNION" --feed iex >/dev/null 2>&1 || true
# live streams and warms up on IEX (since 2026-09-24), so the like-for-like replay is on the IEX cache
./target/release/backtest --date "$D" --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars_iex --cross-index SPY 2>/dev/null > "$OUT/replay_trades.csv" || true
./target/release/backtest --date "$D" --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars --cross-index SPY 2>/dev/null > "$OUT/replay_trades_sip.csv" || true
# one replay per enabled book (its own config row / ticker set, no sizing override) -> replay_<book>.csv
: > "$OUT/replay_books.log"
while read -r b; do
    [[ -z "$b" ]] && continue
    LIVE_DIR="$LIVE_DIR" ./scripts/replay_book.sh "$b" "$D" >>"$OUT/replay_books.log" 2>&1 </dev/null || echo "replay_book $b exit $?" >> "$OUT/replay_books.log"
done <<< "$(Q "select name from books where enabled order by role, name" || true)"
python3 scripts/analysis/near_miss_replay.py "$D" > "$OUT/near_miss_replay.txt" 2>/dev/null || true
echo "exported $OUT: $(ls "$OUT" | wc -l) files"
# commit and push so the routine (and history) can see it; never fail the timer on push
[[ "${EXPORT_NO_GIT:-0}" == "1" ]] && exit 0
git add "$OUT" >/dev/null 2>&1 && git commit -q -m "live: export $D" >/dev/null 2>&1 && (git push -q origin HEAD 2>/dev/null || echo "export: push failed (offline?)" >&2) || true
