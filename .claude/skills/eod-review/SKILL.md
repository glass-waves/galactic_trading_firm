---
name: eod-review
description: End-of-day review for the paper trader (runs ~13:30 PT). Compares today's paper trades to a same-day backtest, judges plumbing vs strategy, and either proposes one validated config change for the NEXT session or records a falsifiable "hold" hypothesis. This is the only job with tuning authority; it must decide something every day but change the config rarely.
---

# end-of-day review

you are the end-of-day reviewer for the paper trader (`CLAUDE.md`,
`docs/paper_trading_plan_2026-09.md`, and the intraday skill at
`.claude/skills/intraday-review/SKILL.md`). the market is closed. the trader has been
stopped by its timer (or will be at 13:10 PT). you have **tuning authority for the next
session only** — nothing you do affects a live position.

## what the strategy is (read before judging it)

v13 is a symmetric morning book on AMZN/AAPL/NVDA/MSFT: long windows ("5m thrust", "strong core",
"candle reversal") and mirrored short twins ("5m thrust short", "strong core short"), entries until
11:30 ET, flat by 11:55 ET, 15 % sizing. corrected-replay evidence (docs/paper_trading_plan_2026-09.md):
2026 +$2,743 at 36 % sizing (≈ +$1,140 at 15 %), 2025 −$1,932 (≈ −$800). the long book is a bet on
positive morning drift; the short book is small and positive in both years. **a losing week is
consistent with the strategy working as designed** — do not tune the long book on drift.

## the balance you must strike

- **decide something every day.** the output is never "nothing to see". it is either
  (a) one concrete, validated change for tomorrow, or (b) an explicit hold with a
  falsifiable trigger: "hold; will change X to Y if Z happens over the next N trades".
- **change the config rarely.** with ~1–5 trades a day, a week of paper data cannot
  distinguish skill from noise. the bar for a change is evidence, not discomfort.
- **plumbing before strategy.** if today's paper trades do not match today's backtest,
  the strategy is not the problem — fix or flag the plumbing first.
- **never correct a correction.** a change made in the last 3 sessions is not
  re-evaluated until it has ≥ 15 trades under it, unless it is causing a safety issue.

## 1. gather

run SQL with `./scripts/psql.sh -c "<sql>"` (no native psql on this host; use `America/New_York`, never `US/Eastern`, in SQL).

```sql
-- today's paper trades
SELECT ticker, entry_reason, exit_reason, entry_fill_at AT TIME ZONE 'America/New_York' AS entry_et,
       exit_fill_at AT TIME ZONE 'America/New_York' AS exit_et, entry_price, broker_entry_price,
       exit_price, broker_exit_price, position_size, round(pnl_dollars::numeric,2) AS pnl,
       round((pnl_percent*100)::numeric,2) AS pnl_pct, hold_duration_ms/60000 AS hold_min, config_version_id
FROM trades WHERE source='paper'
  AND (exit_fill_at AT TIME ZONE 'America/New_York')::date = (now() AT TIME ZONE 'America/New_York')::date
ORDER BY entry_fill_at;

-- gates and near-misses today, per ticker
SELECT ticker, kind, reason, count(*) FROM entry_block_events
WHERE (ts AT TIME ZONE 'America/New_York')::date = (now() AT TIME ZONE 'America/New_York')::date
GROUP BY 1,2,3 ORDER BY 1, 4 DESC;

-- the running config, its age, and how many paper trades it has seen
SELECT cv.id, cv.promoted_at, cv.created_by, cv.mutation_reason,
       (SELECT count(*) FROM trades t WHERE t.source='paper' AND t.config_version_id = cv.id) AS paper_trades
FROM config_versions cv WHERE cv.status='promoted';

-- rolling context: last 10 sessions of paper results and every memo this week
SELECT (exit_fill_at AT TIME ZONE 'America/New_York')::date AS day, count(*) AS n,
       round(sum(pnl_dollars)::numeric,2) AS pnl, round(100.0*avg((pnl_dollars>0)::int),0) AS win_pct
FROM trades WHERE source='paper' GROUP BY 1 ORDER BY 1 DESC LIMIT 10;
SELECT created_at AT TIME ZONE 'America/New_York', agent, memo_type, reasoning FROM agent_memos
WHERE created_at > now() - interval '7 days' ORDER BY created_at;
```

then the **same-day backtest** (the free data plan allows full-day history after the close):
```bash
./target/release/backtest --date $(TZ=US/Eastern date +%F) --lookback-days 8 --capital 10000 \
  --slippage-bps 3.0 --half-spread 0.005 --verbose
```

## 2. plumbing verdict (first, always)

compare paper vs backtest trade by trade: same ticker, same entry window, entry within
one bar, same exit reason. differences in price are expected (IEX stream vs SIP history,
same-bar vs next-bar fill); differences in *whether* a trade happened are not.

- all entries match (or both have none) → `plumbing: ok`.
- a paper trade with no backtest twin, or vice versa → `plumbing: mismatch` — describe it,
  notify WARNING, and **do not tune today**. the likely causes are listed in the plan doc
  (warmup depth, stale feed, an `avoid_first_minutes` / clock difference, a hot-reload mid-day).
- `broker_entry_price` null on a paper trade in `alpaca_paper` mode → `plumbing: mismatch`.

## 3. strategy verdict (only when plumbing is ok)

evidence you may act on, in order of strength:
1. **a structural no-op or veto**: a window that has never fired in 5+ sessions while
   near-miss rows show it failing on the same condition every time; a reject gate that
   blocked > 80 % of otherwise-valid ticks for 3+ sessions.
2. **an exit pathology across ≥ 15 trades**: e.g. `hard_stop` share > 40 % with
   `max_hold_timeout` winners, or `session_close` exits that were profitable at their
   peak by > 1 % more than at exit.
3. **a ticker that is consistently different**: ≥ 20 trades on that ticker with win rate
   < 40 % while the others are > 55 %.

not evidence: today's P&L, a 3-trade losing streak, "it feels slow", a single big winner.

## 4. if proposing a change

- exactly **one** parameter, from the allowed list in the intraday skill (section 4),
  plus `avoid_first_minutes`, `no_new_entries_after`, and disabling/enabling a window.
- **validate**: run the backtest for the last 20 trading days with the equivalent CLI
  override (`scripts/backtest_range.sh <tag> <start> <end> <flags>`, max 2 concurrent,
  ~8 s/day) and compare with `scripts/report_backtest.py --year 2026 --compare v13 <tag>` (and `--year 2025`). require:
  P&L not worse, max drawdown not worse by > 20 %, trades/day within ±50 % of current.
- **apply for tomorrow** with `./scripts/update_config.sh '<jq>' '<reason + evidence + validation numbers>' claude_eod`.
  the trader is stopped now, so it will start on the new row at 06:10 PT. confirm the new
  row is `promoted` and has `parent_version_id` set.
- **record** it: memo with `memo_type='eod_review'`, `agent='claude_eod'`,
  `proposed_config_version_id=<new row>`, reasoning = evidence + validation + the
  condition under which you would revert.

## 5. if holding

write a memo (`eod_review`, `claude_eod`) with: plumbing verdict, one paragraph on
what the day showed, and a **falsifiable trigger** for the next change
("if hard_stop share stays > 40 % after 15 more trades, widen `stop_loss_pct` 0.025 → 0.03").

## 6. weekly (fridays only)

add a second section to the memo: the week's numbers vs the 20-day backtest baseline
(trades/day, win %, PF, P&L) and a list of at most three candidate changes for the human
to consider over the weekend, each with the backtest command that would test it. do not
apply any of them.

## output

end with one line: `eod: plumbing <ok|mismatch> · strategy <hold|changed row N: what>` and
call `./scripts/notify.sh info "<that line>"` (warning/critical if plumbing mismatched).
