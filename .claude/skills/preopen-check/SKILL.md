---
name: preopen-check
description: Pre-open readiness check for the paper trader (runs ~06:15 PT on trading days). Verifies the service is up, warmed, on the expected config, and that nothing is orphaned at the broker. Notifies on anything that would make the day's data untrustworthy.
---

# pre-open readiness check

you are the pre-open checker for the paper trader (`CLAUDE.md`, `docs/paper_trading_plan_2026-09.md`).
it is ~06:15 PT / 09:15 ET on a trading day. the systemd timer started `paper-trader.service`
at 06:10 PT. your job: confirm the day's data will be trustworthy, and say so in one line.
you make NO config changes at pre-open.

## checks (run all; then classify)

```bash
systemctl --user is-active paper-trader.service
journalctl --user -u paper-trader.service --since "10 minutes ago" --no-pager -o cat | tail -60
```

```sql
-- via: ./scripts/psql.sh -c "<sql>"   (no native psql on this host; the wrapper uses the container)
SELECT ticker, now() - updated_at AS heartbeat_age, config_version_id, broker_mode, feed_stale,
       position_direction, process_started_at
FROM engine_state ORDER BY ticker;
SELECT id, promoted_at, created_by, left(mutation_reason, 80) FROM config_versions WHERE status = 'promoted';
-- yesterday's memos, so you know what the EOD job decided
SELECT created_at AT TIME ZONE 'America/New_York', agent, memo_type, left(reasoning, 300)
FROM agent_memos WHERE created_at > now() - interval '20 hours' ORDER BY created_at;
```

| condition | level |
|---|---|
| service not active, or no `engine_state` rows updated in the last 2 min | CRITICAL |
| journal shows `starting cold`, `fewer than 3 trading days`, or `FEED STALE` | WARNING (day's hourly scores suspect) |
| journal shows `BROKER HOLDS A POSITION THE ENGINE DOES NOT KNOW ABOUT` | CRITICAL (orphan from a previous day — human must close it) |
| `broker_mode` ≠ what `.env` says, or `config_version_id` ≠ the promoted row | WARNING |
| journal shows `config blob's config_id differs` only | ignore (expected) |
| any `position_direction` non-null this early | CRITICAL (nothing should be open before 09:30 ET) |
| yesterday's EOD memo proposed a config for today but the promoted row didn't change | WARNING (mention it; do not apply it yourself) |

## output

- CRITICAL → `./scripts/notify.sh critical "<one line>"` and a memo (`memo_type='watchdog_critical'`, `agent='claude_intraday'`).
- WARNING → memo (`watchdog_warning`) and `./scripts/notify.sh warning "<one line>"`.
- all clear → no memo, no notification. reply with exactly one line: `preopen ok — <config row> · <n tickers warm> · <broker_mode>`.
