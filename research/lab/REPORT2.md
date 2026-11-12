# lab round 2 (2026-11-11 → 11-12): permute from row 25

Goal (user): start from the promoted row 25 (`lab-na-bundle`) and permute — brainstorm changes, additions and tweaks,
narrow by backtest, run the top few as shadows. Plan, protocol and pre-registered tests: `ROUND2.md`. Four Opus desks
(exits, entries, sizing-universe, second-books), 105 ledger runs on this round (programme ledger 352 at the reveal),
desk reports in `desks/r2-*.md`. Judge output: `REPORT2_judge.md`.

## verdict

**Row 25 is at a local optimum of its configuration surface. No tweak beats it on the holdout; the only finalist that
passes the pre-registered tests is leverage.** Five finalists were fixed before the reveal (ROUND2.md "finalists"):

| finalist | kind | train (vs row 25 +2610 / 320 / 1.75) | holdout (vs +899 / 111 / 2.00) | paired vs row 25, holdout | verdict |
|---|---|---|---|---|---|
| `r2/sizing/tier_x150` — ×1.5 at VPIN ≥ 0.26 | T (leverage) | +2984 / 320 / 1.77, dd −266 | +1047 / 111 / 2.05, dd −141 | +148, P 0.004 | **passes T1–T4** |
| `r2/entries/pdl15` — prior-day-low ≥ −1.5 % on am/pm | T (quality) | +2634 / 256 / 2.16, dd −174 | +891 / 78 / 2.50, dd −134 | −7, P 0.50 | fails count and paired |
| `r2/entries/peersnotall_thr` — no am thrust when all peers red | T | +2839 / 282 / 1.98 | +678 / 105 / 1.77 | −221, P 0.97 | fails; worse on holdout |
| row 25 + `r2/books/m5_r3_ls5_dm175` — afternoon long book | A | book +1071 / 97 / 1.69, corr 0.00 | book −82 / 9 / 0.34 | — | fails A4 |
| row 25 + `r2/books/m1_r3_ls5` — same book, round-1 gate | A (control) | book +935 / 153 / 1.42 | book −91 / 21 / 0.66 | — | fails A2, A4 |

Reading, cell by cell:
- **tier ×1.5** is the same 111 holdout trades at larger size on the VPIN ≥ 0.26 half. It passes because that half was net
  positive on the holdout as on train; at matched notional the gain is ≈ 0 (sizing desk: +75, P 0.20 on train). It adds
  ≈ 12 % P&L for ≈ 12 % drawdown. The live form is base 0.30, ×1.5, cap 0.45 (`patches/r2/sizing/tier_x150.pipeline.json`).
- **pdl15** makes the same money on 30 % fewer trades with a shallower drawdown and a PF of 2.5 on the holdout, i.e. the
  excluded trades are net zero. It is a risk-preference variant, not an improvement in dollars, and the entries desk showed
  the dial is not monotone (−2.5 % cuts winners, −1.5 % cuts losers, −1.0 % cuts winners). It fails the 150-trade goal.
- **peers-not-all-red** was a delay effect carried by 19 re-entries on the screen window; the holdout reverses it (−221).
- **the afternoon long book** was +757 of +1071 from one day (2025-04-09) on train; on the holdout it lost on 9 trades.
  Round 1's version needed a 22-day warm-up that the primary cannot share; at warm-up 8 the mechanism is gone.

## what the desks established (negatives are results)

- **exits** (23 specs): every hold, stop, breakeven and VWAP-stop variant on row 25 is flat or negative; the ATR trail
  never fires; the stress window's 2 h / −0.6 are its best values; the RQ-14 max-hold clock offset is ±1 bar = −57 / −27
  on train (close that item). Engine note: only the replay calls `set_max_hold_ms`; harmless for row 25 (verify before
  any window relies on the position-level default).
- **entries** (25 specs): window clocks are exhausted (10:10 cut-over and 10:00 stress start never bind; pm to 11:15
  loses on train); quality metadata conditions (relative volume, VWAP stretch, red-day, SPY 15 m, relative weakness)
  mostly remove winners — climax bars are the winners.
- **sizing-universe** (15 runs): row 25's own ×1.25 tier is leverage (matched-notional +60, P 0.26); VPIN bands are not
  monotone; AAPL cut/down-weight is in-sample only (round 1's holdout already reversed it); QQQ/SPY under row 25 fail the
  60-trade bar; the 3-position cap binds twice; same-day re-entries are the book's best trades, so cooldowns cut winners.
  **Engine findings (live too):** `ticker_overrides.sizing_fraction` is ignored under `indicator_tiered`; per-ticker
  `entry_threshold` does nothing on a window-only config; sizing actions do not stack; `max_capital_deployed_pct` is dead
  (`total_deployed_capital` is never set) — live exposure is bounded only by 3 × 0.40; `volatility_scaled` is price-level
  dependent; the replay does not enforce `max_concurrent_positions`.
- **second-books** (19 specs): the QQQ afternoon noise book has no edge at warm-up 8 (ex-2022 PF 0.86); afternoon shorts
  inside row 25 lose and can pre-empt the stress window; stress variants (1.0 %, 2.0 %, longer hold, score exit off) are
  ≤ 0; a stress-reversal long is dead and a "day +1" trigger cannot be written (no prior-session index return key).

## shadows

1. **`lab-tier-x150`** proposed to the pipeline (gate `sizing-config`, cells_scanned 15, base row 25) — the nightly gate
   sweeps it against `live_25` and opens `shadow:lab-tier-x150`. Expect a pass (same trade set, +12 % P&L, drawdown per
   dollar unchanged). The shadow is a parity check of the tier at ×1.5; it cannot show edge.
2. **`shadow:pdl15`** created by hand (config row materialized from row 25 + `patches/r2/entries/pdl15.json`, status
   `validated`, book `created_by = 'human'`): no pipeline gate fits a same-dollars-fewer-trades variant, and the quality
   gate would fail it on the paired bound by construction. It runs for information: live parity of `prior_day_levels`
   (a new indicator in the live path) and the realised trade count. It is not a promotion candidate under the 150-trade goal.
3. Nothing else. The afternoon long and the peer filter failed the holdout; exits had nothing.

## engine work this round surfaced (not implemented; binary frozen)

1. `max_capital_deployed_pct` is dead code — either wire `total_deployed_capital` or delete the knob (live safety claim!).
2. `TickerOverrides::apply` should cover `indicator_tiered` (or reject the override) — silent no-op today.
3. per-book history depth (each book's candle windows cut to its own warm-up), so a 22-day-warm-up afternoon book could run
   beside row 25 without moving its trades; or a `noise_area` sigma from a daily table.
4. a prior-session index return key on `cross_context`; a peer-timing parity flag (`--cross-lag 1` for peers).
5. multiplicative sizing on a second key (the prior-day-low clause as a size dial, not a cut).
6. `--patch-json "disable"` covering indicators (round 1, still open); `vpin.bucket_divisor` cleanup (round 1).

## next

The configuration surface around row 25 is mapped: clocks, exits, filters, sizing, universe and second books. Further
config-only rounds on the four names are unlikely to pay. What would change the picture: (a) the engine items above,
especially per-book warm-up (re-opens the afternoon books) and a real deployed-capital cap; (b) more data (the holdout is
111 trades); (c) a different market — a long book or a different instrument class needs its own programme, not a patch.
Live evidence accrues at ~7 trades/month; ≈ 200 trades (two years) before live PF means anything.
