# ANT (Anthropic, PBC — NASDAQ) as a potential fifth name — 2026-11-27

ANT listed 2026-11-19 (first IEX bar 11:48 ET at 112). five sessions exist (11-19 → 11-25, close 82.85):
too few to backtest. this is a mechanics check of row 25 on the name, not a verdict. the window is after
the registered holdout end (2026-10-30); one pre-declared cell, run with `LAB_REVEAL=1`.

specs: `specs/r3/ant_only.json` (row 25, tickers ANT), `specs/r3/ant_add.json` (row 25 + ANT), vs `specs/base/row25.json`,
window `2026-11-19:2026-11-25`.

| cell | P&L | n | PF | dd | note |
|---|---:|---:|---:|---:|---|
| base/row25 | +156 | 6 | 3.06 | −67 | equals the live primary's six trades (+155.84) |
| r3/ant_only | −133 | 8 | 0.17 | −144 | 7 of 8 losers; one hard stop −91 (11-20 stress window) |
| r3/ant_add | +22 | 14 | 1.09 | −56 | base + ANT; ANT's trades are additive (no position-cap interaction seen) |

ANT trades: the stress window (SPY ≤ −1.5 %) fired four times on 11-19/11-20 while SPY fell ~3 % each day;
ANT did not follow the index down intraday — it was in IPO price discovery, bouncing off its lows while the
mega-caps slid. three breakeven stops and one hard stop. 11-24: two breakeven stops and one +27 session-close winner.

liquidity: IEX 1m volume median ~31k shares at ~$83 (NVDA ~28k at ~$149 on the same feed); shortable, easy to borrow.

decision: no candidate proposed (the additive-ticker gate needs a multi-year sweep). proposed instead: a human
shadow book `shadow:ant` (follows the promoted row, tickers {ANT}, simulated fills) so the name accumulates
forward data without touching the primary. revisit after ~60 sessions, or sooner if the IPO regime (no index
beta) persists — a new listing is not what row 25's SPY-relative gates were fitted on.

create (human):
`INSERT INTO books (name, role, config_version_id, tickers, capital, enabled, purpose, candidate_id, created_by)
 VALUES ('shadow:ant', 'shadow', NULL, ARRAY['ANT'], NULL, true, 'human shadow 2026-11-27: promoted config on ANT alone; research/lab/r3_ant.md', NULL, 'human');`
retire: `UPDATE books SET enabled=false, retired_at=now(), retire_reason='...' WHERE name='shadow:ant';`
