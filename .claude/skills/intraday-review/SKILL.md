---
name: intraday-review
description: One tick of the market-hours watchdog + review loop for the paper trader. Reads engine_state / trades / entry_block_events from postgres, checks health, and proposes or applies a config change only under strict guardrails. Run repeatedly with `/loop 15m /intraday-review` while paper_trader is running.
---

# intraday review — one tick

you are the intraday watchdog for the paper trader described in `CLAUDE.md` and
`docs/paper_trading_plan_2026-09.md`. this skill is ONE tick of a loop that runs every
15 minutes during US market hours. be fast, be quiet when nothing is wrong, and
default to **hold steady**. a bad config change is worse than no change.

all times below are US/Eastern. market hours are 09:30–16:00 ET; the promoted config
takes no new entries after `session.no_new_entries_after` and is flat by
`session.force_exit_by` (11:30 / 11:55 ET in v16). v16 is SHORT-ONLY (long windows disabled): every position is a short,
and a short with a very negative composite is healthy, not a warning. ~1 trade/day is normal.

## 0. environment

- postgres: `./scripts/psql.sh -c "<sql>"` (picks native psql or the container; there is
  no native psql on this host). read-only unless step 4 says otherwise. the container's
  tz database lacks the `US/Eastern` alias — always write `America/New_York` in SQL.
- the trader writes `engine_state` on every bar and every 30 s. `updated_at` is the heartbeat.
- if it is a weekend, a market holiday, or outside 09:15–16:15 ET: say so in one line and stop.
- if the ET time is past `session.force_exit_by` + 5 min AND every `engine_state` row has
  `position_direction` null AND no row has `feed_stale = true`: the book is closed for the
  day. do the health query once (step 1, heartbeat + positions only), reply
  `hold — flat after close, heartbeat ok`, and stop. this keeps the afternoon ticks cheap.

## 1. health (always; no judgement needed)

run these and classify. any CRITICAL ⇒ `./scripts/notify.sh critical "<one line>"` and
write a memo (step 5). WARNING ⇒ memo plus `./scripts/notify.sh warning "<one line>"`.

```sql
-- heartbeat + feed + positions
SELECT ticker, now() - updated_at AS heartbeat_age, now() - last_bar_at AS bar_age,
       feed_stale, loss_breaker_active, entry_blocked_by,
       position_direction, position_entry_price, position_unrealized_pnl_pct,
       position_hold_ms / 60000 AS hold_min, position_entry_reason,
       config_version_id, pending_config_version_id, daily_pnl
FROM engine_state ORDER BY ticker;

-- today's trades
SELECT ticker, entry_reason, exit_reason, entry_fill_at AT TIME ZONE 'America/New_York' AS entry_et,
       round(pnl_dollars::numeric,2) AS pnl, round((pnl_percent*100)::numeric,2) AS pnl_pct,
       hold_duration_ms/60000 AS hold_min, config_version_id, broker_entry_price, entry_price
FROM trades
WHERE source='paper' AND (exit_fill_at AT TIME ZONE 'America/New_York')::date = (now() AT TIME ZONE 'America/New_York')::date
ORDER BY exit_fill_at;
```

| condition | level |
|---|---|
| any `heartbeat_age` > 3 min during hours | WARNING; > 6 min CRITICAL (process dead?) |
| `feed_stale = true` on any ticker | WARNING; all tickers CRITICAL |
| open position and ET time > `force_exit_by` + 3 min | CRITICAL (the clock-net should have closed it) |
| `daily_pnl` < −3 % of INITIAL_CAPITAL | WARNING; < −5 % CRITICAL |
| `loss_breaker_active` on any ticker | WARNING (expected behaviour, but the user should know) |
| trades today > 3 × the 20-day backtest mean | WARNING (churn) |
| a trade with `broker_entry_price` null in `alpaca_paper` mode | WARNING (order never filled but engine kept the trade?) |

do **not** place or cancel broker orders yourself in week 1. the human is present.

## 2. read the day so far (only if health is OK or WARNING)

```sql
-- gates: what is stopping entries, per ticker, last 90 minutes
SELECT ticker, kind, reason, count(*), min(ts AT TIME ZONE 'America/New_York') AS first, max(ts AT TIME ZONE 'America/New_York') AS last,
       round(avg(composite)::numeric,2) AS avg_composite
FROM entry_block_events
WHERE ts > now() - interval '90 minutes'
GROUP BY ticker, kind, reason ORDER BY ticker, count(*) DESC;

-- memos already written today (so you do not repeat yourself)
SELECT created_at AT TIME ZONE 'America/New_York' AS at_et, agent, memo_type, left(reasoning, 200)
FROM agent_memos
WHERE created_at > (now() AT TIME ZONE 'America/New_York')::date ORDER BY created_at;

-- promoted config
SELECT id, promoted_at, mutation_reason FROM config_versions WHERE status='promoted';
```

read the promoted `config_blob` only if you are considering a change.

## 3. decide — default is HOLD

with 1–5 trades a day, today's P&L is noise. do not tune on it. a change is
warranted only on one of these evidence patterns:

- **a gate is vetoing everything**: e.g. `reject_gate:1m noise filter` on ≥ 90 % of
  block events across ≥ 3 tickers for ≥ 90 minutes while near-miss rows show windows
  otherwise passing.
- **an exit is systematically misfiring**: ≥ 3 `hard_stop` exits today that were up
  > 1 % at some point (compare `position_unrealized_pnl_pct` history in memos / TUI).
- **a ticker is broken**: halted, `feed_stale` for that symbol only, or 2+ broker
  rejections ⇒ remove the ticker from `tickers` rather than tune around it.
- **safety**: drawdown WARNING ⇒ tighten `stop_loss_pct` or lower
  `max_concurrent_positions`. never loosen anything on a losing day.

if none apply: write nothing, say "hold — <one line>", and stop.

## 4. change protocol (only when step 3 fires)

hard rules — do not bend them:

1. **max 1 parameter per tick, max 3 per day.** none in the first 30 min after the
   open or the last 30 min before `no_new_entries_after`.
2. **no stacking**: if the current promoted row has < 10 paper trades under it, do not
   touch the same parameter area again.
3. **allowed knobs**: entry-window `conditions` thresholds; `enabled` on a window or
   the reject gate; `stop_loss_pct`; ATR `multiplier`; `max_hold_ms` /
   `profit_extension_ms`; `scoring.exit_threshold`; `entry_cooldown_ms`;
   `no_new_entries_after` (earlier only); removing a ticker; `max_concurrent_positions`
   **downward only**; `ticker_overrides` for `stop_loss_pct` / `atr_multiplier` /
   `max_hold_ms`.
4. **forbidden intraday**: sizing `fraction` (any direction); `max_position_pct`;
   adding tickers (needs a restart); adding/removing indicators or changing an
   indicator's `timescale`/`period`; `hard_gate_timescales`; `force_exit_by`;
   `max_concurrent_positions` upward; `scoring.timescale_weights`; anything that makes
   the blob fail to deserialise.
5. **validate non-safety changes** before promoting: run the backtest for the last
   5 trading days with the equivalent CLI override (max 2 concurrent, ~1 min/day) and
   require P&L ≥ the current config's on those days and no rise in `hard_stop` share.
   safety changes (rule 3, last bullet of step 3) skip validation.
6. **apply only when flat**: the trader swaps engines per ticker as each goes flat,
   so a change made mid-position takes effect on that ticker's next entry. that is
   acceptable; but do not change *exit* parameters while a position is open (the
   engine holding it keeps the old ones — confusing to review).
7. **how to apply** (one command; creates a new promoted row with parent + reason):
   ```bash
   ./scripts/update_config.sh '<jq expression>' '<reason with the evidence>' claude_intraday
   ```
   then confirm within 2 minutes that `engine_state.config_version_id` moved (or
   `pending_config_version_id` is set) for the affected tickers.
8. **rollback** = `update_config.sh` with the *previous* blob as a new row (the
   watcher only sees higher ids). never `UPDATE ... SET status='promoted'` on an old row.

## 5. memo (only when something happened)

one row per tick that had a WARNING/CRITICAL, a proposal, or a change:

```sql
INSERT INTO agent_memos (agent, memo_type, reasoning, flags, proposed_config_version_id, trades_reviewed, period_pnl)
VALUES ('claude_intraday', '<watchdog_warning|watchdog_critical|intraday_review>',
        '<3–6 lines: what you saw, what you did or did not do, and why>',
        '{"level": "<ok|warning|critical>", "tickers": [...]}'::jsonb,
        <new config id or NULL>, <trades today>, <daily_pnl>);
```

silent ticks write nothing. end your reply with one line: `hold` / `warning: …` /
`critical: …` / `changed: <row id> <what>`.

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
