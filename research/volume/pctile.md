# Study A — self-calibrating VPIN floor (rolling percentile instead of raw ≥ 0.217)

Run 2026-09-24/25 on the IEX 1-minute cache (2022-01-01..2026-09-10), honest costs
(3 bps slippage + $0.005 half-spread per leg), research sizing 0.36 / 0.36, `--cross-index SPY`.

## Question and design

The v18 short windows gate on `vpin_1m.raw_vpin ≥ 0.217`. That number was fitted to one feed;
IEX volume is ~3 % of consolidated, so a raw floor is not portable and might also let the trade
rate drift with the volume regime. This study replaces it with a rolling percentile:
a second VPIN instance `vpin_p` (`pctile_window` = W readings, 1-minute bars) emits `pctile` =
share of the last W readings below the current raw VPIN, and both short windows require
`vpin_p.pctile ≥ P`. Everything else in the two windows (composite ≤ −0.35, 5m/1h timescale
conditions, SPY session return within ±0.2 %) is unchanged from v18.

Grid: W ∈ {390, 1170, 1950} (1, 3, 5 sessions) × P ∈ {0.4, 0.5, 0.6, 0.7}. Follow-ups on the best W
(1950): P = 0.55, the SPY band widened to ±0.3 %, and one extra cell P = 0.8 because no grid
cell came close to the baseline's trade count (needed for the like-for-like test).

Bar: pooled PF ≥ 1.3, ≥ 4 of 5 years positive, trades ≥ 407.

## Grid

Per-year cells are P&L (PF). Δ columns are vs `iex_v18`.

| tag | W | P | SPY band | 5y P&L | trades | win % | PF | max DD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | Δ trades | Δ P&L |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| iex_v18 (baseline) | raw ≥0.217 | — | ±0.2% | +2154 | 407 | 39 | 1.48 | -526 | +1565 (2.98) | -181 (0.83) | -93 (0.92) | +451 (1.57) | +413 (1.60) | 3/5 | +0 | +0 |
| vp_w390_p0.4 | 390 | 0.4 | ±0.2% | +2132 | 683 | 36 | 1.28 | -630 | +1556 (1.84) | -279 (0.81) | +31 (1.02) | +359 (1.25) | +464 (1.40) | 4/5 | +276 | -22 |
| vp_w390_p0.5 | 390 | 0.5 | ±0.2% | +2127 | 642 | 36 | 1.30 | -632 | +1632 (1.98) | -271 (0.81) | -42 (0.97) | +428 (1.32) | +381 (1.33) | 3/5 | +235 | -27 |
| vp_w390_p0.6 | 390 | 0.6 | ±0.2% | +2031 | 585 | 36 | 1.31 | -662 | +1655 (2.08) | -298 (0.78) | -62 (0.96) | +276 (1.22) | +460 (1.48) | 3/5 | +178 | -123 |
| vp_w390_p0.7 | 390 | 0.7 | ±0.2% | +1920 | 525 | 37 | 1.31 | -595 | +1660 (2.28) | -183 (0.86) | -133 (0.91) | +276 (1.24) | +301 (1.31) | 3/5 | +118 | -234 |
| vp_w1170_p0.4 | 1170 | 0.4 | ±0.2% | +1819 | 689 | 35 | 1.24 | -684 | +1460 (1.77) | -289 (0.81) | -66 (0.96) | +336 (1.23) | +379 (1.31) | 3/5 | +282 | -335 |
| vp_w1170_p0.5 | 1170 | 0.5 | ±0.2% | +2274 | 640 | 36 | 1.32 | -664 | +1684 (1.99) | -311 (0.78) | -28 (0.98) | +461 (1.36) | +469 (1.43) | 3/5 | +233 | +120 |
| vp_w1170_p0.6 | 1170 | 0.6 | ±0.2% | +2152 | 584 | 36 | 1.33 | -604 | +1702 (2.17) | -296 (0.79) | +9 (1.01) | +303 (1.23) | +434 (1.46) | 4/5 | +177 | -2 |
| vp_w1170_p0.7 | 1170 | 0.7 | ±0.2% | +1967 | 524 | 37 | 1.33 | -547 | +1635 (2.23) | -154 (0.87) | -113 (0.92) | +273 (1.24) | +325 (1.33) | 3/5 | +117 | -187 |
| vp_w1950_p0.4 | 1950 | 0.4 | ±0.2% | +1906 | 682 | 35 | 1.25 | -694 | +1505 (1.79) | -322 (0.79) | -71 (0.96) | +348 (1.24) | +445 (1.37) | 3/5 | +275 | -248 |
| vp_w1950_p0.5 | 1950 | 0.5 | ±0.2% | +2279 | 638 | 36 | 1.33 | -630 | +1723 (2.03) | -294 (0.79) | +4 (1.00) | +461 (1.36) | +386 (1.35) | 4/5 | +231 | +125 |
| vp_w1950_p0.6 | 1950 | 0.6 | ±0.2% | +2239 | 586 | 37 | 1.34 | -528 | +1681 (2.13) | -222 (0.83) | -10 (0.99) | +367 (1.30) | +422 (1.44) | 3/5 | +179 | +85 |
| vp_w1950_p0.7 | 1950 | 0.7 | ±0.2% | +1832 | 521 | 37 | 1.30 | -567 | +1542 (2.13) | -159 (0.87) | -142 (0.90) | +269 (1.24) | +322 (1.33) | 3/5 | +114 | -322 |
| *follow-ups on W = 1950* | | | | | | | | | | | | | | | | |
| vp_w1950_p0.55 | 1950 | 0.55 | ±0.2% | +2176 | 616 | 36 | 1.32 | -571 | +1707 (2.08) | -288 (0.79) | -32 (0.98) | +415 (1.32) | +374 (1.36) | 3/5 | +209 | +22 |
| vp_w1950_p0.5_b0.6 | 1950 | 0.5 | ±0.3% | +1942 | 746 | 36 | 1.23 | -683 | +1494 (1.64) | -179 (0.87) | +159 (1.10) | +150 (1.09) | +318 (1.25) | 4/5 | +339 | -212 |
| vp_w1950_p0.8 (extra) | 1950 | 0.8 | ±0.2% | +2216 | 440 | 39 | 1.44 | -400 | +1711 (2.71) | -141 (0.86) | +33 (1.03) | +341 (1.33) | +272 (1.34) | 4/5 | +33 | +62 |

## Extra breakdown — trade rate steadiness and the like-for-like test

Yearly trade counts, 2022–2025 only (2026 is partial):

| cell | 2022 | 2023 | 2024 | 2025 | mean | sd | CV |
|---|---|---|---|---|---|---|---|
| iex_v18 (raw ≥ 0.217) | 91 | 80 | 90 | 77 | 84.5 | 6.1 | **0.072** |
| vp_w1950_p0.8 (like-for-like) | 105 | 81 | 91 | 87 | 91.0 | 8.8 | 0.097 |
| vp_w1950_p0.6 | 147 | 111 | 122 | 102 | 120.5 | 16.9 | 0.140 |
| vp_w1950_p0.5 (best grid P&L) | 166 | 120 | 128 | 111 | 131.2 | 20.9 | 0.160 |
| vp_w1950_p0.4 | 182 | 126 | 133 | 118 | 139.8 | 25.0 | 0.179 |

W barely matters: at fixed P the three windows agree on trade count within ±5 (W=390 CV
0.119–0.172, W=1170 0.124–0.177, W=1950 0.123–0.179). CV rises monotonically as P falls — the
looser the floor, the more 2022 (the high-volatility year) dominates. No percentile cell is
steadier than the raw floor; the hypothesis that a percentile floor evens out the trade rate
across years is falsified on this data.

Closest trade count to iex_v18's 407 is `vp_w1950_p0.8` at 440 (+33). Like-for-like: PF 1.44 vs
1.48, 5y +2216 vs +2154, max DD −400 vs −526, win 39 % vs 39 %, 2024 +33 vs −93 (both are
noise), CV 0.097 vs 0.072. Statistically indistinguishable from the fixed floor. Interpolating
trade counts, raw 0.217 on IEX corresponds to roughly the 82nd–85th percentile of the 5-session
VPIN distribution; every grid P (≤ 0.7) is a *looser* filter than v18 runs today.

Per-window split (baseline → p0.5 → p0.8): the extra trades come almost entirely from the
5m-thrust window (360 → 565 → 389). Strong-core stays small (47 → 73 → 51) and keeps its PF > 5 in
2022 but is negative in 2023–24 in every cell. 2022 is 73–76 % of 5y P&L in every cell, exactly
as in the baseline.

## Sanity checks

- Every trade in every cell carries `entry_reason` = `window:5m thrust short pct` or
  `window:strong core short pct` (the patch's names); the promoted short windows are disabled
  (7,299 rows across the 12 grid cells; follow-ups the same).
- All 15 sweeps report 260/260/262/261/181 days, 0 skipped, no `skipped:` lines in any log.
- `pctile` needs 195 readings; with an 8-day lookback (~3,100 bars) the key exists from the first
  RTH bar of every replay day, including for W = 1950.
- Machine was under load (load 19 on 12 cores); sweeps took 8–12 min each, no effect on results.
- Cost model untouched (script defaults).

## Interpretation

- The percentile floor is a *portability* device, not an alpha source. At matched selectivity
  (P = 0.8 ≈ raw 0.217 on IEX) it reproduces the fixed floor: same PF, same P&L, same year
  pattern. That is the useful result — a P value would transfer between IEX and SIP where 0.217 will not.
- It does not steady the trade rate. VPIN is not the binding condition; the year-to-year swing in
  trade count comes from how often composite/5m/1h thrust conditions fire (2022 fires ~40 % more
  often than 2025 at every P). Loosening VPIN amplifies that swing rather than damping it.
- Below P ≈ 0.8 the marginal trades are roughly breakeven: p0.5 adds 198 trades over p0.8 for
  +$63. They add drawdown (−630 vs −400) and cut PF (1.33 vs 1.44) without adding money.
- W is irrelevant in the 1–5 session range: the VPIN distribution is stationary enough over days
  that a 1-session and 5-session percentile pick the same bars. Prefer the longest (1950) for
  fewer wobbles at the open; it costs nothing.
- Widening SPY to ±0.3 % fails as it did in the raw-floor study (§14.1): +108 trades, −$337 vs the
  same P at ±0.2 %, PF 1.23. The flat-SPY band remains a real edge on IEX.
- 2023 is negative in every cell (PF 0.78–0.87) and 2024 hovers at zero (±$160). Neither knob
  studied here touches that; the 4/5-positive-years flags in the table all rest on a 2024
  within ±$35 of zero and should be read as 3/5 with a coin flip.
- Cells at P ≤ 0.6 (584–689 trades) are the only ones that would meaningfully raise the live trade
  rate (~1 extra trade per 2 days) and they do so at PF ~1.3 — acceptable but not better.

## Verdict

Best cell: **vp_w1950_p0.8** (W = 1950, P = 0.8, SPY ±0.2 %): 5y +2216, 440 trades, PF 1.44,
max DD −400, years +1711 / −141 / +33 / +341 / +272. It nominally passes the bar (PF 1.44 ≥ 1.3,
4/5 years positive, 440 ≥ 407), but the fourth positive year is 2024 at +$33 and the cell is a
re-expression of the fixed floor, not an improvement on it (Δ P&L +62 on 440 trades).
Best cell inside the requested grid: vp_w1950_p0.5 (+2279, 638 trades, PF 1.33, 2024 +4) — also a
nominal pass, on the same 2024 coin flip, with worse PF and drawdown than the baseline.

Recommendation: if the floor must be portable across feeds (it must — the live/replay VPIN
mismatch of 2026-09-24 is exactly this problem), adopt `vpin_p.pctile ≥ 0.8` with W = 1950 as a
feed-independent restatement of raw ≥ 0.217, expecting parity, not gain. Do not adopt any P < 0.7
for the trade-rate argument; that argument is falsified here.

## Commands

```bash
# grid (12 cells): W ∈ {390,1170,1950} × P ∈ {0.4,0.5,0.6,0.7}; patch = smoke_pctile.json with W/P substituted
for W in 390 1170 1950; do for P in 0.4 0.5 0.6 0.7; do
  BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh vp_w${W}_p${P} --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/volume/vp_w${W}_p${P}.json
done; done
# follow-ups on W=1950
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh vp_w1950_p0.55    --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/volume/vp_w1950_p0.55.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh vp_w1950_p0.5_b0.6 --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/volume/vp_w1950_p0.5_b0.6.json   # cross_1m in [-0.6, 0.6]
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh vp_w1950_p0.8     --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/volume/vp_w1950_p0.8.json
# summaries
python3 research/entries/summarize.py iex_v18 vp_w1950_p0.5 vp_w1950_p0.8 --by-window
```
