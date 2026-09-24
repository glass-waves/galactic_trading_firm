#!/usr/bin/env bash
# export one trading day's live state to data/live/<YYYY-MM-DD>/ as plain JSON/CSV so an
# off-machine agent (a claude routine on a git checkout) can write the end-of-day report.
# usage: scripts/export_day.sh [YYYY-MM-DD]   (default: today, America/New_York)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
set -a; source .env; set +a
D="${1:-$(TZ=America/New_York date +%F)}"
OUT="data/live/$D"; mkdir -p "$OUT"
Q() { ./scripts/psql.sh -At -c "$1"; }

Q "select json_agg(row_to_json(t)) from (select ticker, updated_at, last_bar_at, feed_stale, loss_breaker_active, config_version_id, pending_config_version_id, position_direction, position_entry_price, position_hold_ms, entry_blocked_by, near_miss, daily_pnl, process_started_at from engine_state order by ticker) t" > "$OUT/engine_state.json"
Q "select json_agg(row_to_json(t)) from (select ticker, direction, entry_reason, exit_reason, entry_fill_at, exit_fill_at, entry_price, exit_price, broker_entry_price, broker_exit_price, position_size, pnl_dollars, pnl_percent, hold_duration_ms, entry_score_composite, exit_score_composite, config_version_id from trades where source='paper' and (exit_fill_at at time zone 'America/New_York')::date = '$D' order by exit_fill_at) t" > "$OUT/trades.json"
Q "select json_agg(row_to_json(t)) from (select ts, ticker, kind, reason, composite from entry_block_events where (ts at time zone 'America/New_York')::date = '$D' order by ts) t" > "$OUT/block_events.json"
Q "select json_agg(row_to_json(t)) from (select (exit_fill_at at time zone 'America/New_York')::date as day, count(*) as trades, round(sum(pnl_dollars)::numeric,2) as pnl from trades where source='paper' and exit_fill_at > now() - interval '30 days' group by 1 order by 1) t" > "$OUT/recent_days.json"
Q "select json_agg(row_to_json(t)) from (select id, status, promoted_at, mutation_reason, config_blob->>'config_id' as config_id from config_versions order by id desc limit 5) t" > "$OUT/config_versions.json"
Q "select json_agg(row_to_json(t)) from (select created_at, agent, memo_type, reasoning, flags from agent_memos where created_at > now() - interval '7 days' order by created_at) t" > "$OUT/memos.json"

# systemd + alerts + check-in outputs
{
  echo "{\"exported_at\":\"$(date -Is)\",\"service_active\":\"$(systemctl --user is-active paper-trader.service)\","
  echo " \"service\":$(systemctl --user show paper-trader.service -p ActiveEnterTimestamp -p NRestarts -p Result | python3 -c 'import sys,json; print(json.dumps(dict(l.split("=",1) for l in sys.stdin.read().splitlines() if "=" in l)))'),"
  echo " \"timers\":$(systemctl --user list-timers --no-pager | grep -E 'paper-trader|preopen|eod|watchdog' | awk '{print $1" "$2" "$3" "$NF}' | python3 -c 'import sys,json; print(json.dumps(sys.stdin.read().strip().split("\n")))')}"
} > "$OUT/system.json"
grep "^$(date -d "$D" +%Y-%m-%d)" logs/notifications.log > "$OUT/notifications.log" 2>/dev/null || true
for j in preopen-check watchdog; do [[ -f logs/checkins/$j.jsonl ]] && tail -1 logs/checkins/$j.jsonl > "$OUT/$j.jsonl" || true; done
journalctl --user -u paper-trader.service --since "$D 00:00" --until "$D 23:59" --no-pager -o cat 2>/dev/null | grep -iE 'WARN|ERROR|reconnect|shutdown|starting|config' | grep -v sqlx | tail -40 > "$OUT/journal_excerpt.log" || true

# same-day replay from the bar cache (free plan: bars available ~16 min after the close)
./target/release/backtest --fetch-bars data/bars --start "$D" --end "$D" --tickers AAPL,AMZN,NVDA,MSFT,SPY >/dev/null 2>&1 || true
./target/release/backtest --date "$D" --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars --cross-index SPY 2>/dev/null > "$OUT/replay_trades.csv" || true
python3 scripts/analysis/near_miss_replay.py "$D" > "$OUT/near_miss_replay.txt" 2>/dev/null || true
echo "exported $OUT: $(ls "$OUT" | wc -l) files"
