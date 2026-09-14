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

## v16 notes (promoted 2026-09-12 ~11:00 PT, config_versions row 10)

- v16 = v15 (short-only, AMZN/AAPL/NVDA/MSFT, 30 % sizing) plus three exit/entry changes:
  losing-side hold limit 40 min (`max_hold.loss_reduction_ms` 3,000,000) and winning limit 90 min
  (`profit_extension_ms` 0); `window_strong_core_short` hourly condition ≤ −0.40; and a *working*
  breakeven stop at 0.5 % (`exit_reason = 'breakeven_stop'`, new enum value). the breakeven
  monitor was a silent no-op before 2026-09-12.
- **expected shape**: win rate ~34 % (many trades cut at 40 min or at breakeven for a few dollars),
  exit mix ≈ 62 % max_hold_timeout / 23 % breakeven_stop / 8 % session_close / 6 % score_exit /
  2 % hard_stop. lots of small losses and breakevens is the design, NOT a malfunction. do not
  "fix" the win rate. a `breakeven_stop` exit at −$2…−$8 is normal (next-bar fill + spread).
- **honest expectations**: the replay cost model was direction-blind until 2026-09-12 and credited
  shorts with favourable slippage; under the corrected model v16 is +$1,684 over 2022–2026 at
  36 % sizing (PF 1.12, max drawdown $702) vs +$112 for v15. the edge is thin. the point of week 1
  is whether live fills, latency and the exit plumbing match the replay — compare
  `broker_entry_price`/`broker_exit_price` against the bar opens, and the exit-reason mix against
  the numbers above. do not tune entry thresholds on a week of data.
- rollback = re-insert the v15 blob (row 9) as a new promoted row via `update_config.sh`, never
  `UPDATE … SET status`.

## v17 notes (promoted 2026-09-12 ~13:30 PT, config_versions row 11)

- v17 = v16 plus two **entry filters** on both short windows: SPY session return within ±0.2 %
  at the entry bar (`cross_1m` score in [−0.4, 0.4]) and VPIN raw ≥ 0.217 (`vpin_1m.raw_vpin`).
  honest five-year replay +$2,785 / 469 trades / PF 1.64 / maxDD $401, every year positive
  (v16: PF 1.12). expect **~0.4 trades/day** — several no-trade days a week is normal.
- the SPY filter depends on the live feed populating `MarketState.cross` from a SPY bar
  subscription (`data_feed/src/cross_tracker.rs`). if `entry_block_events.near_miss` rows show
  `cross_1m n/a` (or `vpin_1m.raw_vpin n/a`) all morning, the cross feed is not working — that is
  a WARNING (plumbing), not a strategy signal; do not loosen the window to compensate.
- `near_miss` rows mentioning `cross_1m … not ≥ / not ≤` mean SPY was moving more than 0.2 %
  either way: the filter doing its job.
- rollback = re-insert the v16 blob (row 10) as a new promoted row via `update_config.sh`.

## v18 note (promoted 2026-09-14 ~10:30 PT, row 12)

v18 = v17 + `max_concurrent_positions` 3 (was 1) and `max_capital_deployed_pct` 0.95. up to three
tickers may be short at once (≈ 90 % of the $10k budget); that is intended — the replay evidence
was produced without a cross-ticker cap. do not lower it on a losing day unless the daily-loss
WARNING fires. `entries_blocked` gate events should now be rare.
