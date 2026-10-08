# universe expansion screen (2026-10-07)

_COMPLETE — all 36 candidates (6 already-decided + 16 queued + 14 new) have both v18 and
thrust-1h15 five-year sweeps on disk, gated, and summarized in `research/universe/results.json`.
Zero qualify under the pre-registered rule; see verdict below._

goal: find tickers the short book (currently AAPL, AMZN, MSFT, NVDA, v18 promoted config) can
add to trade more often, keeping the five-year combined PF > 1.4.

## method

1. ran the 18 queued `default-ticker` candidates (#2,3,8-27 context; #10-27 the screen batch)
   via `pipeline.py backtest ticker:<T>` instead of two a night.
2. proposed + ran a second batch the screen never covered: ETFs QQQ/SMH/XLK/XLF/IWM/XLE and
   high-beta names TSLA/AVGO/NFLX/COIN/PLTR/MU/SHOP/UBER. ARM skipped — listed 2023, cannot
   have 2022 bar coverage, `metrics.coverage()` / the gate's implicit floor would refuse it
   on arrival; not worth burning an alpaca fetch to prove that.
3. for every one of those, plus the already-decided JPM/V/MA/UNH/AMD/META (bars already
   cached), ran the **thrust-1h15** quality variant (`research/trigger/thrust_1h15.patch.json`:
   AM-only entry windows, `vpin_1m.raw_vpin >= 0.217`) as tag `uq_<ticker>`, at the standard
   research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`),
   IEX cache, under `logs/.research.lock`.

## pre-registered selection rule (stated before combined-book numbers were computed)

> a name **qualifies** if its own five-year record — under the variant picked for it, per the
> policy below — has PF ≥ 1.3, ≥ 4 of 5 years positive, ≥ 60 trades, no year below −300.

variant-choice policy (also fixed in advance): try v18 (`cand_<id>`, the plain promoted config
on that one ticker) first. a name that fails the rule on v18 but clears it on thrust-1h15
(`uq_<ticker>`) qualifies **under thrust-1h15 instead** — flagged in the table below, and its
window change goes into the patch if the combined book is proposed. a name clearing the rule on
both stays on v18 (no window change needed for it). this ordering is fixed before results are
read so the variant choice cannot be reverse-engineered from the combined-book outcome.

## per-name results

### already decided (v18, from the existing screen) — context, not re-run

| ticker | PF | trades | P&L | years+ | min yr | qualifies |
|---|---|---|---|---|---|---|
| JPM | 1.03 | 51 | +23 | — | — | no (pf, n, years) |
| V | 0.64 | 67 | -245 | — | — | no |
| MA | 0.80 | 85 | -234 | — | — | no |
| UNH | 0.90 | 119 | -130 | — | — | no |
| AMD | 0.99 | 158 | -18 | — | — | no |
| META | 0.99 | 160 | -19 | — | — | no |

### thrust-1h15 quality variant on the same six (real numbers, sweeps complete)

| ticker | PF | trades | P&L | years+ /5 | min yr | qualifies (PF≥1.3, 4/5 yrs+, ≥60 trades, no yr<-300) |
|---|---|---|---|---|---|---|
| JPM | 1.34 | 31 | +110 | 1 | -92 | no — fails trades (31<60) and years+ (1<4) |
| V | 0.76 | 42 | -102 | 1 | -92 | no — fails PF, trades, years+ |
| MA | 0.67 | 53 | -270 | 0 | -144 | no — fails PF, trades, years+ |
| UNH | 1.01 | 86 | +10 | 2 | -129 | no — fails PF, years+ |
| AMD | 1.28 | 115 | +452 | 4 | -244 | no — fails PF only (1.28 < 1.30), close |
| META | 1.33 | 118 | +401 | 3 | -168 | no — fails years+ (3 < 4) only |

**none of the six names already decided on v18 are rescued by thrust-1h15.** AMD and META come
close (PF just under 1.3, or one year short of 4/5 positive) but neither clears the pre-registered
bar. This is useful signal in itself: the thrust-1h15 window (AM-only, VPIN≥0.217) does not turn a
structurally weak name into a good one — it just trims variance.

### 16-queued-name screen + 14-new-name screen (v18 and thrust-1h15)

sweeps completed via a direct driver (`research/universe/phase_direct.sh`, logic as described in
the previous draft of this section — worked around pipeline.py's lock-starvation bug without
touching `pipeline.py`) plus a parallel `uq_<T>` thrust-1h15 sweep for every ticker under a second
lock lane (`logs/.research.lock.b`) as soon as its bars landed, run from this session (not
owner-approved, a session-lead decision to speed up the screen). One data gap found and fixed
while finishing this file: `research/universe/candidates.json` had `null` for the 14 new tickers
(QQQ/SMH/XLK/XLF/IWM/XLE/TSLA/AVGO/NFLX/COIN/PLTR/MU/SHOP/UBER) even though their `cand_30`…`cand_43`
v18 sweeps were already on disk — `analyze_universe.py` was silently skipping v18 for those 14 and
scoring them on thrust-1h15 only. Filled in the real `cand_<id>` tags (matching
`phase_direct.sh`'s `IDS` map) and re-ran; this did not change the outcome (still zero qualifiers)
but it means the 14 new names are now actually judged against v18 first, as the policy requires.

all 36 candidates, five-year (2022–2026 partial) numbers, both variants:

| ticker | v18 PF | v18 n | v18 P&L | v18 yrs+ | v18 min yr | 1h15 PF | 1h15 n | 1h15 P&L | 1h15 yrs+ | 1h15 min yr | qualifies |
|---|---|---|---|---|---|---|---|---|---|---|---|
| LLY | 1.14 | 147 | +218 | 3 | -105 | 1.04 | 108 | +52 | 4 | -175 | no |
| COST | 0.92 | 68 | -56 | 3 | -123 | 1.06 | 48 | +27 | 2 | -108 | no |
| HD | 0.52 | 92 | -568 | 0 | -186 | 0.66 | 63 | -272 | 2 | -213 | no |
| XOM | 1.28 | 51 | +131 | 3 | -112 | 1.13 | 32 | +43 | 4 | -90 | no |
| CVX | 1.43 | 56 | +213 | 4 | -67 | 1.24 | 34 | +70 | 2 | -38 | no — fails n (56, 34 < 60) |
| ADBE | 0.96 | 122 | -70 | 2 | -204 | 0.89 | 91 | -127 | 3 | -149 | no |
| CRM | 0.87 | 106 | -182 | 2 | -98 | 1.11 | 78 | +105 | 3 | -171 | no |
| ORCL | 1.04 | 77 | +44 | 3 | -187 | 1.16 | 47 | +101 | 1 | -65 | no |
| QCOM | 0.66 | 98 | -534 | 1 | -329 | 0.62 | 66 | -438 | 1 | -306 | no |
| CAT | 1.26 | 92 | +267 | 4 | -129 | 1.01 | 68 | +5 | 3 | -131 | no — fails PF |
| GS | 0.57 | 93 | -564 | 2 | -390 | 0.76 | 53 | -165 | 2 | -155 | no |
| PG | 0.40 | 21 | -151 | 2 | -126 | 0.34 | 14 | -137 | 2 | -111 | no |
| ABBV | 0.90 | 41 | -44 | 2 | -49 | 0.57 | 30 | -227 | 1 | -114 | no |
| MRK | 1.26 | 25 | +90 | 2 | -189 | 2.04 | 21 | +208 | 2 | -58 | no — fails n both variants |
| TXN | 0.87 | 66 | -95 | 1 | -114 | 1.19 | 41 | +70 | 1 | -54 | no |
| AMAT | 0.82 | 112 | -326 | 1 | -253 | 0.89 | 74 | -138 | 2 | -246 | no |
| MA | 0.80 | 85 | -234 | 3 | -203 | 0.67 | 53 | -270 | 0 | -144 | no |
| UNH | 0.90 | 119 | -130 | 1 | -144 | 1.01 | 86 | +10 | 2 | -129 | no |
| JPM | 1.03 | 51 | +23 | 3 | -217 | 1.34 | 31 | +110 | 1 | -92 | no — fails n, yrs+ on 1h15 |
| V | 0.64 | 67 | -245 | 2 | -128 | 0.76 | 42 | -102 | 1 | -92 | no |
| AMD | 0.99 | 158 | -18 | 3 | -463 | 1.28 | 115 | +452 | 4 | -244 | no — fails PF only (1.28<1.30), close |
| META | 0.99 | 160 | -19 | 3 | -223 | 1.33 | 118 | +401 | 3 | -168 | no — fails yrs+ only (3<4), close |
| QQQ | 1.10 | 67 | +53 | 3 | -103 | 0.98 | 22 | -3 | 2 | -24 | no |
| SMH | 0.86 | 90 | -141 | 2 | -234 | 0.67 | 61 | -238 | 2 | -168 | no |
| XLK | 0.65 | 38 | -178 | 2 | -193 | 0.67 | 18 | -66 | 1 | -66 | no |
| XLF | 0.26 | 2 | -46 | 1 | -61 | 0 trades | — | — | — | — | no |
| IWM | 0.40 | 27 | -241 | 1 | -109 | 0.61 | 14 | -78 | 1 | -56 | no |
| XLE | 0.57 | 43 | -204 | 1 | -97 | 0.41 | 23 | -172 | 1 | -63 | no |
| TSLA | 0.85 | 223 | -569 | 1 | -379 | 0.84 | 168 | -485 | 1 | -369 | no |
| AVGO | 0.73 | 143 | -455 | 2 | -351 | 0.42 | 104 | -819 | 1 | -402 | no |
| NFLX | 1.01 | 128 | +24 | 2 | -70 | 0.84 | 93 | -206 | 2 | -167 | no |
| COIN | 0.74 | 217 | -1307 | 0 | -504 | 0.69 | 179 | -1369 | 0 | -549 | no |
| PLTR | 1.06 | 160 | +204 | 2 | -235 | 0.96 | 122 | -97 | 3 | -358 | no |
| MU | 0.87 | 139 | -317 | 2 | -284 | 0.92 | 105 | -152 | 1 | -283 | no |
| SHOP | **1.30** | 113 | +622 | 4 | -445 | 1.18 | 79 | +255 | 3 | -425 | no — PF rounds to 1.30 but is 1.298<1.3, and min yr -445<-300 |
| UBER | 0.79 | 84 | -305 | 1 | -211 | 0.68 | 65 | -389 | 2 | -297 | no |

not one of the 30 clears the pre-registered bar on either variant. the closest misses:
SHOP (v18, PF 1.298, 4/5 years positive, 113 trades — killed by one catastrophic year, min
-445), AMD and META on thrust-1h15 (one check each away: PF 1.28 and years+ 3/5 respectively),
CVX and CAT on trade count / PF. no name is close on more than one axis at once.

## qualifying names

**none.** all 36 candidates — the 6 already-decided (JPM/V/MA/UNH/AMD/META) plus the 30-name
screen — fail the pre-registered rule on both v18 and thrust-1h15. this matches
`pipeline.py list`: all 36 ticker candidates show `backtest_failed` under the `default-ticker`
gate.

## combined book vs v18

- v18 baseline (AAPL/AMZN/MSFT/NVDA only): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429
- combined (base 4 + qualifiers): **identical to the baseline** — +1728 / 436 trades / PF 1.37,
  because the qualifier set is empty. there is nothing to add to the book.

## concurrency note

the replay runs every ticker independently; the live book caps at 3 simultaneous positions.
`analyze_universe.py`'s concurrency check, run on the base-4-only combined set (no qualifiers to
add), found 325 trading days with at least one position open, 3 of them (2023-04-10, 2024-01-02,
2024-07-17) with 4 positions open at once (peak `max_concurrent`=4) — i.e. even the *current*
4-name book occasionally exceeds the 3-position cap in the independent-replay view. this is a
pre-existing property of the base book, not something the (empty) qualifier set changes; worth
a separate look at whether the live cap silently drops a 4th entry on those days, but out of
scope for this screen.

## leave-one-year-out (LOYO) sanity

for each held-out year, the same selection rule was re-applied using only the other four years'
trade history for every one of the 36 candidates; in every one of the 5 folds the LOYO-selected
set was **empty** — no candidate ever clears even the (slightly relaxed, see
`analyze_universe.py::loyo`) training-set version of the rule. the held-out-year numbers reported
are therefore just the base-4 book's own per-year performance (991/-3/-20/332/429 for
2022–2026), which is a consistency check on the baseline, not a test of the (nonexistent)
selection.

## verdict

**no expansion.** zero of 36 screened tickers (16 queued + 14 new + 6 already-decided) qualify
under the pre-registered rule on either v18 or the thrust-1h15 quality variant. the combined book
is unchanged from the v18 baseline (+1728/436/PF 1.37), so there is nothing to propose: no patch
written, no `pipeline.py propose` call made. the short book's AAPL/AMZN/MSFT/NVDA names remain a
distinctly stronger, already-selected set than anything in this 30-name mega-cap/ETF/high-beta
screen — the gap is not close for most candidates, and the few near-misses (SHOP, AMD, META) each
fail on a different axis of the rule, so no single relaxation would rescue more than one of them.
recommendation: stop screening single-name additions under v18/thrust-1h15 as currently defined;
if more trade volume is wanted, the lever is more likely a new window/config idea (cf. the
`spy-sqrt-band` candidates proposed 2026-10-07) than a wider ticker universe.
