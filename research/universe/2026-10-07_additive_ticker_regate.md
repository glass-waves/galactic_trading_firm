# additive-ticker: regating the 36 universe candidates on marginal effect — 2026-10-07

`default-ticker` judges a candidate ticker on its own stand-alone 5y record (PF ≥ 1.3, ≥ 4/5
years, ≥ 100 trades, no year < −300). Two of the four live names fail that exact bar on today's
IEX replay (AAPL PF 1.26/3 yrs, AMZN PF 0.74/3 yrs) — the gate is stricter for a newcomer than it
ever was for the incumbents. The right question for a candidate is its **marginal** effect on the
live book, not its own isolated number.

## gate: additive-ticker (`scripts/pipeline/gates.py`, wired in `pipeline.py` like additive-config)

Candidate's own sweep (`--tickers T`) is concatenated with baseline tag `iex_v18` (`metrics.marginal_metrics()`,
new `combined`/`concurrency` fields). Every candidate trade is `added` (a different ticker never
matches a baseline `trade_key()`), so `base` is empty and unchecked. `combined` = `iex_v18`'s own
trades + the candidate's. Checks: added trades ≥ 40 (5y); added P&L > 0; added PF ≥ 1.15; combined
PF ≥ baseline PF − 0.02 (not dilutive); `combined_years_not_worse` — no year of the combined book
> 50 worse than baseline's same year; added worst year ≥ −150. Bar-cache coverage is enforced the
same way as `default-ticker` (pipeline's kind-level check, before the gate runs). The live
3-position cap is **info only** (`concurrency_info` check, always `ok: true`) — the replay runs
each ticker independently, so concatenation is exact except for that cap. 7 unit tests added
(`AdditiveTicker` in `test_gates.py`) plus `ConcurrencyExceedance`/combined-field tests in
`test_metrics.py`; all 106 pipeline tests pass.

## regate: all 36 ticker candidates (ids 2, 3, 8–43)

No new sweeps — `pipeline.py regate ticker:<T> --gate additive-ticker` reused every existing
`cand_<id>` sweep. No sweep tag was missing; all 36 regated. **0 of 36 pass.** Sorted by added P&L:

| ticker | n | add P&L | add PF | comb PF | worst yr | margin | fails |
|---|---|---|---|---|---|---|---|
| SHOP | 113 | +622 | 1.30 | 1.345 | 23 | -445 | c_pf,c_years_not_worse,a_worst_year |
| CAT | 92 | +267 | 1.26 | 1.348 | 24 | -128 | c_years_not_worse |
| LLY | 147 | +218 | 1.14 | 1.310 | 22 | -105 | a_pf,c_pf,c_years_not_worse |
| CVX | 56 | +213 | 1.43 | 1.372 | 24 | -67 | c_years_not_worse |
| PLTR | 160 | +204 | 1.06 | 1.245 | 26 | -235 | a_pf,c_pf,c_years_not_worse,a_worst_year |
| XOM | 51 | +131 | 1.28 | 1.358 | 26 | -112 | c_years_not_worse |
| MRK | 25 | +90 | 1.26 | 1.359 | 25 | -189 | a_trades,c_years_not_worse,a_worst_year |
| QQQ | 67 | +53 | 1.10 | 1.340 | 26 | -102 | a_pf,c_pf,c_years_not_worse |
| ORCL | 77 | +44 | 1.04 | 1.301 | 26 | -187 | a_pf,c_pf,c_years_not_worse,a_worst_year |
| NFLX | 128 | +24 | 1.01 | 1.271 | 24 | -70 | a_pf,c_pf,c_years_not_worse |
| JPM | 51 | +23 | 1.03 | 1.325 | 26 | -217 | a_pf,c_pf,c_years_not_worse,a_worst_year |
| AMD | 158 | -18 | 0.99 | 1.229 | 26 | -463 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| META | 160 | -19 | 0.99 | 1.259 | 23 | -223 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| ABBV | 41 | -44 | 0.90 | 1.325 | 23 | -48 | a_pnl,a_pf,c_pf |
| XLF | 2 | -46 | 0.26 | 1.352 | 22 | -62 | a_trades,a_pnl,a_pf,c_years_not_worse |
| COST | 68 | -56 | 0.92 | 1.311 | 24 | -123 | a_pnl,a_pf,c_pf,c_years_not_worse |
| ADBE | 122 | -70 | 0.96 | 1.260 | 23 | -204 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| TXN | 66 | -95 | 0.87 | 1.300 | 26 | -114 | a_pnl,a_pf,c_pf,c_years_not_worse |
| UNH | 119 | -130 | 0.90 | 1.268 | 23 | -144 | a_pnl,a_pf,c_pf,c_years_not_worse |
| SMH | 90 | -141 | 0.86 | 1.277 | 26 | -234 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| PG | 21 | -151 | 0.40 | 1.317 | 26 | -126 | a_trades,a_pnl,a_pf,c_pf,c_years_not_worse |
| XLK | 38 | -178 | 0.65 | 1.296 | 26 | -193 | a_trades,a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| CRM | 106 | -182 | 0.87 | 1.252 | 26 | -98 | a_pnl,a_pf,c_pf,c_years_not_worse |
| XLE | 43 | -204 | 0.57 | 1.293 | 26 | -97 | a_pnl,a_pf,c_pf,c_years_not_worse |
| MA | 85 | -234 | 0.80 | 1.254 | 22 | -204 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| IWM | 27 | -241 | 0.40 | 1.291 | 25 | -109 | a_trades,a_pnl,a_pf,c_pf,c_years_not_worse |
| V | 67 | -245 | 0.64 | 1.275 | 26 | -128 | a_pnl,a_pf,c_pf,c_years_not_worse |
| UBER | 84 | -305 | 0.79 | 1.230 | 26 | -211 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| MU | 139 | -317 | 0.87 | 1.196 | 26 | -284 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| AMAT | 112 | -326 | 0.82 | 1.214 | 23 | -252 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| AVGO | 143 | -455 | 0.73 | 1.199 | 26 | -351 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| QCOM | 98 | -534 | 0.66 | 1.190 | 24 | -329 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| GS | 93 | -564 | 0.57 | 1.193 | 26 | -390 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| HD | 92 | -568 | 0.52 | 1.197 | 25 | -186 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| TSLA | 223 | -569 | 0.85 | 1.134 | 22 | -379 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |
| COIN | 217 | -1307 | 0.74 | 1.043 | 23 | -504 | a_pnl,a_pf,c_pf,c_years_not_worse,a_worst_year |

`c_pf` never falls far short (worst passing-direction value 1.043, most ≥ 1.2 — the book absorbs a
weak name's PF easily). The binding constraint everywhere is **`combined_years_not_worse`**: three
names (CVX, CAT, XOM) fail on *that check alone* — their own PF/trades/P&L are fine — because the
baseline's own 2023 (−3) and 2024 (−20) are so thin that even a mildly bad year from the candidate
(CVX −67 margin, CAT −128, XOM −112 in a strong 2026) blows through the 50-point tolerance. This is
not "no good candidates exist"; it's "the live book's weak years have no room to absorb anything".

## stack / greedy vs v18

0 passers at 5y → nothing to add. Combined (base 4 + all passers) = **unchanged**: +1,728 / 436 /
PF 1.366, years +991 −3 −20 +332 +429 (same as `iex_v18`). Greedy (by added P&L/trade, non-dilutive
+ years-not-worse gating as each is added) also selects nothing — same reason.

## concurrency (info only, base-4 book; unaffected since nothing was added)

3 of 436 trading days exceed the live 3-position cap: 2023-04-10, 2024-01-02, 2024-07-17 (max
4 concurrent). Pre-existing property of the current book, confirmed by `metrics.concurrency_exceedance()`.

## LOYO on the greedy selection (train on 4y, evaluate the held-out 5th)

Training on 4 years *does* pick up candidates the full 5y bar correctly rejects — and every one
hurts the held-out year:

| held out | training passers (greedy) | held-out combined P&L | held-out baseline P&L |
|---|---|---|---|
| 2022 | none | +991 | +991 |
| 2023 | SHOP | **−449** | −3 |
| 2024 | CVX, CAT | **−215** | −20 |
| 2025 | none | +332 | +332 |
| 2026 | ORCL, QQQ | **+139** | +429 |

3 of 5 folds select a candidate that then loses badly (2023, 2024) or costs ~290 of upside (2026)
out-of-sample — classic overfitting on a single lucky year. This confirms the full-5y zero-passer
result is the right answer, not bad luck.

## verdict

**No expansion.** 0 of 36 pass `additive-ticker` at the full 5-year bar, the stacked/greedy book is
identical to `iex_v18` (0% more trades, same PF), and LOYO shows the only way to get a passer is to
overfit a single training split — which then loses on the held-out year in 3 of 5 folds. The
proposal condition (PF ≥ 1.37, ≥ 15% more trades, ≥ 4/5 years positive) is not met because there is
nothing to add. No `universe_additive_v1.patch.json` was written; no `pipeline.py propose` call made.

Report only (no gating), `uq_<T>` vs quality baseline `cand_29` (+1,714/278/PF 1.63): the picture
inverts — AMD (+452, comb PF 1.50) and META (+401, comb PF 1.54) are the two best additions, both
individually failing their own thrust-1h15 bar in the prior universe screen but adding real value
once blended into a higher base PF. COIN (−1,369) and AVGO (−819) remain clearly bad on either
variant. This is a lead for a future round (AMD/META additive-ticker regate against `cand_29`
instead of `iex_v18`), not acted on here.
