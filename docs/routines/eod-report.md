# End-of-day report routine

You are writing the daily end-of-day report for an automated intraday paper-trading bot. You run
in the cloud on a git checkout of this repository; you have **no access to the trading machine**
(no postgres, no systemd, no broker). Everything you need is exported by the desktop after the
close into `data/live/<YYYY-MM-DD>/` and committed. Read those files; do not guess at anything
they do not contain.

Your output is one markdown report, committed to `docs/reports/<YYYY-MM-DD>.md`, plus a
one-line summary at the end of your reply. You never change the strategy config, never edit
code, and never open a pull request. If the day's export directory does not exist, say so in
one line and stop.

## 1. What the system is (read once)

- Four US mega-caps (AMZN, AAPL, NVDA, MSFT), 1-minute bars, SPY as market context. Paper
  account, $10,000 budget, positions sized at 30 % each, up to three concurrent.
- **Short-only morning book.** Two momentum-thrust windows fire 09:30–11:30 ET when the
  5-minute score is strongly negative, filtered to bars where SPY's session return is within
  ±0.2 % (`cross_1m`) and VPIN is in its top quintile (`vpin_1m.raw_vpin ≥ 0.217`). Exits:
  40-minute losing-side time limit, 90-minute winning limit, 0.5 % breakeven stop, 2.5 % hard
  stop, flat by 11:55 ET. The promoted config is v18 (`config_versions` row 12).
- **What the backtest says to expect** (five years, honest costs 3 bps + $0.005 per leg):
  about 0.4 trades per day (≈ 2 per week), win rate ≈ 38 %, profit factor 1.64, average
  +$6 per trade, max drawdown ≈ $400. **About 60 % of days have no trade**; a full blank week
  happens roughly one week in eight even when the strategy is working. Exit mix ≈ 62 %
  max-hold, 23 % breakeven, 8 % session close, 6 % score exit, 2 % hard stop.
- **What actually matters in the first weeks** is plumbing and cost, not P&L: whether live
  fills match the same-day replay, whether realized slippage stays near the 6.5 bps round-trip
  assumption (the edge is ~7 bps deep: PF 1.44 at 5 bps, 1.06 at 10 bps), and whether the
  feeds (bars, SPY context, VPIN) were populated. P&L over a few weeks is noise: ~130 trades
  are needed to separate the edge from zero.
- Background and history: `docs/paper_trading_plan_2026-09.md` (§10 onward), the check-in
  skill `.claude/skills/eod-review/SKILL.md`, and previous reports in `docs/reports/`.

## 2. Files in `data/live/<date>/`

| file | what it is |
|---|---|
| `system.json` | service active/inactive at export time, start timestamp, restart count, next timer firings |
| `engine_state.json` | one row per ticker: heartbeat (`updated_at`), last bar time, `feed_stale`, `config_version_id`, open position fields, `entry_blocked_by`, `near_miss`, `daily_pnl` |
| `trades.json` | today's paper trades: fills (`entry_price`/`exit_price` are the engine's, `broker_*` the broker's), P&L, hold, exit reason, entry window |
| `block_events.json` | every gate/near-miss event today with the reason text |
| `recent_days.json` | last 30 days: trades and P&L per day |
| `config_versions.json` | last five config rows (which is promoted, when, why) |
| `memos.json` | agent memos from the last week |
| `notifications.log` | ntfy alerts sent today (empty = no alerts) |
| `preopen-check.jsonl` | the morning check-in's JSON output (`result` field has its verdict) |
| `journal_excerpt.log` | WARN/ERROR/config/start/stop lines from the trader's log |
| `replay_trades.csv` | the same day replayed from cached bars with the promoted config (rows starting `trade,`) |
| `near_miss_replay.txt` | each near-miss bar replayed as a hypothetical short |

Timestamps in JSON are UTC. Convert to US/Eastern for the report (EDT = UTC−4 until early
November, EST = UTC−5 after). Market hours 09:30–16:00 ET; entries allowed until 11:30 ET.

## 3. What to check, in order

### 3.1 Health (facts, no judgement)

- Did the service start on time (06:10 PT) and run without restarts? (`system.json`, journal
  excerpt.) Was it stopped by the 13:10 PT timer, not a crash?
- Per ticker: `feed_stale` false; last bar ≈ the close; `config_version_id` = the promoted row;
  no open position at export time.
- Were the SPY and VPIN feeds live? Near-miss reasons containing `cross_1m none` or
  `vpin_1m.raw_vpin n/a` on bars **after 09:35 ET** mean the cross feed was not working — that
  is a plumbing WARNING, not a strategy signal. `cross_1m none` on the 09:30 bar alone is normal.
- Alerts: list anything in `notifications.log`. Pre-open verdict: quote its last line.

### 3.2 Trades

For each trade: ticker, direction, window, entry and exit times (ET), hold minutes, exit reason,
P&L. Then **realized cost**: for a short, entry slip = (broker_entry − engine_entry) / engine_entry
and exit slip = (engine_exit − broker_exit) / engine_exit, both in bps, negative = cost. Report
the round-trip cost per trade and the running mean over all trades in `recent_days` you can see
(previous reports carry the earlier values). Flag if the running mean over ≥ 10 trades exceeds
10 bps: that is a strategy-level problem, never something to tune around.

### 3.3 Live vs replay

Compare `trades.json` with the `trade,` rows of `replay_trades.csv` on ticker, entry minute and
exit reason. **Match** = same trades ± one minute, P&L within a few dollars. Any trade in one but
not the other is a plumbing discrepancy and is the most important line in the report: say which
side has it and what the near-miss/gate events show for that ticker at that minute. (Known
benign cause: a live entry blocked by the concurrency cap while the replay, which runs tickers
independently, took it.)

### 3.4 Why there were no trades (blank days only)

Group `block_events.json` near-misses by ticker and reason. For each episode (consecutive bars
on one ticker), say which condition was binding: the base window (5-minute score not lagging,
or above −0.50), the SPY band (`cross_1m` outside ±0.40), or VPIN (`raw_vpin` below 0.22).
Then read `near_miss_replay.txt`: what would the filtered bars have made as shorts? Summarize
as one sentence per episode and one for the day ("the filters saved/cost about X"). Do not
recommend loosening anything on one day's evidence.

### 3.5 Running tally

From `recent_days.json`: trades and P&L for the current week and since v18 was promoted
(2026-09-14). Compare the trade rate to the expected 0.4/day and say plainly whether the
shortfall or excess is inside normal variation (a Poisson rate of 0.4/day gives 0 trades in
5 days about 13 % of the time).

## 4. Decide

The default is **hold**. This routine does not change config. It may *recommend* one, and only
on these patterns:

- **Plumbing** (recommend a fix, not a tune): live/replay mismatch; cross or VPIN feed missing
  after 09:35; restarts; a position held past 11:58 ET; realized cost mean > 10 bps over ≥ 10
  trades; a check-in or alert failure.
- **Trade count** (recommend widening, already validated): if the running rate since
  2026-09-14 is below 0.2/day over ≥ 10 sessions, point to the pre-validated plateau cell in
  `docs/paper_trading_plan_2026-09.md` §11.6/§13.5 (SPY ±0.3 %, VPIN ≥ 0.18: 652 trades / PF
  1.47 / every year positive) as the one change with evidence.
- Never recommend a threshold change based on today's near-misses, a win rate, or a P&L
  figure. Lots of small losses and breakeven exits is the design.

## 5. Write the report

`docs/reports/<date>.md`, in this shape, concise, numbers over adjectives:

```
# EOD <date> — <one-line verdict: "plumbing ok · hold" | "plumbing WARNING: … · hold" | "… · recommend …">

## Health
- service …, restarts …, feeds …, config row …, alerts …, pre-open …

## Trades
| ticker | side | window | entry | exit | hold | reason | P&L | cost bps |
(or "none — see Why no trades")

## Live vs replay
…

## Why no trades  (blank days only)
…

## Tally
week: N trades, $X · since v18: N trades, $X over D sessions (rate r/day vs 0.4 expected) · running cost mean … bps

## Notes
anything unusual, and what (if anything) the human should look at
```

Commit it with the message `report: eod <date>` and end your reply with the verdict line only.
Do not paste the whole report into the reply.

## 6. Things that have bitten before

- SQL against this database must use `America/New_York`, never `US/Eastern` — irrelevant here,
  but the exported timestamps are UTC, so convert before printing times.
- The replay's cost model is direction-aware since 2026-09-12; older numbers in the docs that
  say "positive five of five years" for v15 are from before that fix and are invalid.
- A `breakeven_stop` exit at −$2 to −$8 is normal (next-bar fill plus spread).
- The free data plan lags bars by 16 minutes; if `replay_trades.csv` is empty on a day that
  had live trades, the export probably ran too early — say so rather than calling it a mismatch.
