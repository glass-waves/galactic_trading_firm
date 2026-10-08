# pretest round 1b — bigger neighbourhoods on round 1's five cells (2026-10-08)

Round 1 judged each of its five cells on the paper's rule plus ≤2 variations. This round scans a
**pre-registered neighbourhood** per cell before calling anything dead — see
`research/pretest/README.md`'s "verdict standard" section for the exact vocabulary
(`dead` / `sub-cost` / `decayed` / `fragile` / `pass`) and selection rule (prefer the best-by-P&L
point that clears the kill bar; otherwise report the best-by-P&L point that doesn't, with its gross
edge). Every grid cell's result is in `results/c1{1..5}_*.json`, not just the primary point.

Code: `research/pretest/cells_round1b.py`, run **standalone** through `harness.py`'s loader
functions (`bars`, `fill`, `metrics`, `loyo`, `kill`, `Order`, `execute`, `run_rule`) rather than
registered in `cells.py`'s `CELLS` list — a custom VWAP-cross stop executor (ORB) and a
cross-sectional rank-and-trade shape (HKS) don't fit `cells.py`'s one-symbol-at-a-time
`rule(b, i, p) -> [Order, ...]` interface, so reshaping the harness to fit two one-off cells wasn't
the "smallest possible" change. `harness.py` and `cells.py` are untouched.
`python3 research/pretest/cells_round1b.py` reproduces everything in ~25s.

---

## 1. #1 QQQ opening-range breakout — 81 cells scanned (QQQ), + 9 on SPY/SMH/IWM — **sub-cost**
Grid: range {5,15,30 min} x stop {other side of range, 1x range, VWAP cross} x exit {close, 2R,
5R} x vol filter {none, 14-session-avg-move >=0.7%, >=1.0%}, QQQ only (full grid); the top-3
(or, stop, exit, filter) points by net bps carried unchanged to SPY/SMH/IWM.

Best: `or=5, stop=other-side, exit=close, filt>=1.0%` — QQQ, n=355, **net +2.2 bps**, gross +8.5
bps, PF 1.07, 2/5 years. LOYO across the full 81-cell QQQ grid: P&L −627, 2/5 years (unstable —
different grid points win different years). Cross-instrument (same 3 params, unchanged): SPY n=138
net +0.7 bps PF 1.03 (1/5y); SMH n=559 net −0.7 bps PF 0.99 (2/5y); IWM n=333 net −5.1 bps PF 0.84
(0/5y) — the QQQ-tuned filter does not travel. Reverse-side control (same signal, opposite side):
+2.2 bps actual vs **−2.5 bps reversed** — the breakout direction is weakly informative, not
nothing, but nowhere near tradable.

**Near the bar?** Not close. The vol filter roughly doubles round 1's gross edge (+8.5 vs +3.3 bps)
but the round-trip cost (~6.3 bps realized here) still eats most of it; net PF 1.07 is well short
of 1.3, and 2/5 years is short of 4/5. The filter is also QQQ-specific — it fails on all three
other instruments outright.

## 2. #6 HKS half-hour periodicity, done cross-sectionally — 28 cells (34-name cross-section) — **sub-cost (mechanism marginal)**
Round 1 tested this as a single-name 4-stock sign-of-trailing-mean rule (underpowered — the paper's
test is cross-sectional). This round ranks the ~34 STOCKS names each half-hour by their lagged
same-clock half-hour return (k in {1,2,5,10,20,40,avg40} x bucket {decile, quintile} x halves
{all 13, first+last only}) and goes long the top decile/short the bottom, dollar-neutral.

**Mechanism** (Fama-MacBeth cross-sectional slope of today's half-hour return on the avg40 lagged
signal, by half-hour): only the **closing half-hour** clears |t|>=2 (t=2.21, gross +1.45 bps/leg) —
1 of 13 clears it, below the 2-half-hour bar for "mechanism present," but it *is* the half-hour the
paper calls out as strongest. The open (h=0) shows nothing (t=−0.07). **Tradability**: best grid
point (k=5, decile, open+close only) nets **−4.8 bps**, gross +2.2 bps, PF 0.88, 0/5 years. A
sign-flip permutation test on that cell's daily long-short return is significant (p=0.017 — the
tiny mean is reliably signed, not noise) but the edge itself is too small to matter.

**Near the bar?** Not close on tradability (best gross barely clears zero). The one real signal —
close-only continuation — is genuine but an order of magnitude below cost. **Mechanism verdict is
separate from tradability on purpose**: the periodicity exists, barely, at the close; it was never
going to be a standalone trade.

## 3. #15 earnings-day gap fade, large sample — 144 cells, 30 names / 598 reaction days — **sub-cost** (reverses round 1's pass)
Round 1 tested this on 4 names / 76 events and passed (fragile). This round uses every STOCKS name
with both minute bars and an earnings-date file (30 of 34; COIN/PLTR/SHOP/UBER excluded for no
file, not zero-filled). Grid: gap threshold {any, >=1%, >=2%, >=4%} x direction {fade, follow} x
entry {open, 09:45, 10:00} x exit {11:30, 14:00, close} x gap measure {absolute, vs SPY's same-day
gap}.

Best: `thresh=2%, fade, entry=open, exit=close, gap vs SPY` — n=282, net +11.4 bps, gross +17.9 bps,
**PF 1.10** (needs >1.3), 4/5 years. LOYO across the full 144-cell grid: P&L −3,859, 1/5 years
(badly unstable — the grid-selection process chases noise). Concentration: **NVDA is 81% of the
P&L**, top-3 trades are 91% — essentially unchanged from round 1's 4-name sample even at 7x the
events. Random-session control: this fade rule on *all* sessions (not just reaction days) nets
+4.3 bps; the reaction-day sample's +11.4 bps beats only 66% of 5,000 random same-size draws
(p=0.34) — not distinguishable from an ordinary gap-fade day.

**Near the bar?** The years-positive count (4/5) actually clears; it's specifically PF (1.10 vs
1.3) that fails, and the random-session control confirms there's nothing earnings-specific left
once NVDA's handful of trades are this dominant. Round 1's "pass-pretest, fragile" should not have
stood — the larger sample is the fairer test, and it fails.

## 4. #17 turn-of-month, day-of-cycle — 66 cells, SPY/QQQ/IWM — **pass** (new finding, not pre-specified by the literature)
Grid: day-of-cycle {−2,−1,0,+1,+2,+3,+4} (0 = last session of month) individually, plus 4 windows
([−1,0], [0,+1], [−1,+1], [−2,+4]), x exit {open->close, open->11:30} x instrument.

Best: **day +2** (2nd session of the new month), open->close, **QQQ**: n=57, net **+26.8 bps**,
gross +33.0 bps, PF **1.86**, 4/5 years. Same day on SPY: net +18.7 bps, PF 1.83, **5/5 years**; on
IWM: net +31.5 bps, PF 1.86, but only 3/5 years. Random-session control (QQQ): day +2's +26.8 bps
beats 97% of 5,000 random open->close draws on random sessions (pool mean −3.4 bps, p=0.029).
Which days carry it (SPY, open->close, net bps / PF / years+): day −2 −1.8/0.95/2, day −1 +0.8/1.02/2,
day 0 +5.1/1.15/4, day +1 +0.7/1.00/2, **day +2 +18.7/1.83/5**, day +3 +4.3/1.17/3, day +4
**−25.8/0.41/0**. The effect is concentrated almost entirely in day +2; day +4 is clearly bad and
drags down any window built to include it.

**Near the bar?** This clears the bar with real margin on SPY and QQQ, and the control is
genuinely significant. The caveat: LOYO run across the *full* 22-point-per-instrument grid is
unstable (QQQ P&L −740, 1/5 years; SPY −405, 3/5; only IWM's grid LOYO is positive) because the
grid's year-by-year "best" pick often prefers windows (e.g. [−2,+4]) that include losing days.
Day +2 in isolation, checked year by year, is positive 2022/2024/2025/2026 on SPY and misses only
2023 narrowly — robust on its own, just not the point a blind grid-search would always land on.
Day +2 is also not McConnell-Xu's own point (close(d−2)->close(d+3), or the ETF-era "day 1"); this
is a neighbourhood discovery and should be studied as its own candidate, not reported as a
replication.

## 5. #9 pre-FOMC drift, variants + decay split — 15 cells, SPY/QQQ/IWM — **sub-cost, decay now measured**
Variants: the catalog's overnight window (flagged non-tradable), FOMC-day 09:30->14:00, day-before
open->close, post-announcement 14:00->close same day, next-day open->close; each on SPY/QQQ/IWM;
40 FOMC days.

Best tradable: **FOMC day 09:30->14:00, IWM** — n=38, net +5.8 bps, gross +12.3 bps, **PF 1.292**
(needs >1.3), 3/5 years. LOYO across the 12 tradable cells: P&L −422, 1/5 years. Random-session
control: +5.8 bps beats only 77% of random same-type draws on IWM (p=0.23) — not significant.
**Decay, measured** (gross bps, 2022-24 pooled vs 2025-26 pooled): FOMC-day 09:30->14:00 — SPY
+11.0 -> −8.7, QQQ +23.5 -> −13.3 (both flip sign), IWM +9.1 -> +17.7 (no decay, the one exception).
The catalog's own overnight window: SPY +49.7 -> −6.6 (flips), QQQ +71.2 -> +0.9 (collapses to
flat), IWM +42.5 -> +18.6 (weakens but survives). "The disappearing pre-FOMC drift" is a measured
claim here, not an assertion, on every instrument but IWM's intraday leg.

**Near the bar?** The single closest miss in this round: PF **1.292 vs 1.3**, 3/5 years vs 4/5 —
essentially sitting on the line, and still net positive gross. Not significant against the random
control, though, and every other instrument/variant combination decays outright.

---

## round 1b result
| cell | cells scanned | best cell | net bps | PF | years+ | verdict |
|---|---|---|---|---|---|---|
| #1 QQQ ORB | 81 (+9 cross-inst) | 5-min OR, stop other side, close, vol filter >=1% | +2.2 | 1.07 | 2/5 | sub-cost |
| #6 HKS half-hour | 28 | k=5, decile, open+close | −4.8 | 0.88 | 0/5 | sub-cost (mechanism marginal, close only) |
| #15 earnings gap | 144 | 2% thresh, fade, open->close, vs SPY | +11.4 | 1.10 | 4/5 | sub-cost (reverses round 1's pass) |
| #17 turn-of-month | 66 | day +2, open->close, QQQ | +26.8 | 1.86 | 4/5 | **pass** |
| #9 pre-FOMC | 15 | FOMC day 09:30->14:00, IWM | +5.8 | 1.29 | 3/5 | sub-cost (right at the bar) |

One real pass: **day +2 of the new month**, open->close, on both SPY and QQQ — genuinely new (not
the literature's point), random-session-significant, and worth the lead's attention for an engine
study. Everything else that round 1 called `dead`/`needs-product`/`fragile` is confirmed with a
wider, pre-registered scan rather than narrowed. The earnings-gap "pass" from round 1 specifically
reverses: a 7x larger, honest sample shows it was NVDA-driven noise. Nothing proposed to
`scripts/pipeline/`.
