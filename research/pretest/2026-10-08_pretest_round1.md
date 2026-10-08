# pretest round 1 — matrix top 5 (2026-10-08)

Stage-0 pretests of the first five cells of the ranked list in `research/edge_matrix.md`. Python
replicas on cached bars, no engine. Numbers: `results/*.json`, `results/summary.md`
(`python3 research/pretest/harness.py` reproduces everything in ~8 s).

## harness (research/pretest/harness.py + cells.py)
1. Data: IEX 1-minute RTH bars 2022-01-03 → 2026-10-06 (`data/bars_iex`), one 390-slot grid per session, ET clock.
2. Rules emit `Order(entry session, entry clock, side, exit session, exit clock, stop, target)`; exit in a later session = overnight.
3. Fills: entry at the open of the first bar at/after the clock, exit at the close of the last bar before it.
4. Stops/targets on bar high/low, gap-through at the bar open, stop wins a bar that touches both.
5. Costs 3 bps + $0.005/share per leg; size 36 % of $10,000 in whole shares (as `research/entries/summarize.py`).
6. Metrics: P&L, n, win %, PF, per-year, max DD on daily P&L, worst day, net/gross bps/trade, trades/yr, active-day share, P&L ex top-3 trades.
7. Grid cells: first grid point = the paper's parameters; another point may pass only if leave-one-year-out selection (LOYO) also clears the bar.
8. Kill criteria copied from the matrix before running; `verdict()` applies them mechanically.
9. Overnight / multi-day versions are flagged and can at most earn `needs-product` (the engine is intraday-only).
10. Long-only calendar rules are compared with the same trade on random sessions (5000 draws, p = share of draws ≥ the rule).

---

## 1. #9 pre-FOMC drift — SPY (home), QQQ — **dead**
Kill: net positive ≥ 4/5 years and PF > 1.3. FOMC decision days from `event_calendar.rs`, 38 events (2026: 6).

| variant | inst | n | net bps | PF | per year 22/23/24/25/26 | p vs random |
|---|---|---|---|---|---|---|
| paper 14:00 (d-1) → 14:00 (d) **[overnight]** | SPY | 38 | +22.7 | 2.57 | +239/+47/+63/−0/−56 | 0.07 |
| same | QQQ | 38 | +39.0 | 2.97 | +373/+64/+101/−7/−14 | 0.04 |
| FOMC day 09:30 → 14:00 | SPY | 38 | −2.5 | 0.78 | +53/−13/−2/−23/−44 | 0.43 |
| same | QQQ | 38 | +3.7 | 1.26 | +116/+4/+22/−48/−40 | 0.32 |
| prior session open → close | SPY | 38 | −14.5 | 0.57 | −122/+62/−41/−53/−34 | 0.78 |
| same | QQQ | 38 | −20.6 | 0.58 | −194/+95/−73/−19/−73 | 0.81 |

Verdict: **dead.** The tradable versions fail outright (SPY 09:30→14:00 −2.5 bps, PF 0.78, 1/5 years).
The paper's overnight 24h window still has a high PF but only 3/5 years: 2022 is most of it, and 2025
and 2026 are ≤ 0 on both instruments. That matches "the disappearing pre-FOMC drift", so there is
no new product to want here either. What remains of the drift sits overnight and in the
early afternoon, outside both intraday windows.

## 2. #17 turn of month — SPY (home), QQQ — **needs-product (weak); intraday misses by 0.01 PF**
Kill: net positive ≥ 4/5 years and PF > 1.3. Paper = McConnell-Xu [−1, +3]: close of day −2 → close of day +3.
Intraday = long open→close on day −1 and days +1..+3 (grid: +1 only).

| variant | inst | n | net bps | PF | per year 22/23/24/25/26 | LOYO | p vs random |
|---|---|---|---|---|---|---|---|
| paper close(−2) → close(+3) **[4-session hold]** | SPY | 57 | +21.3 | 1.33 | −86/+172/−63/+22/+328 | — | 0.37 |
| same | QQQ | 57 | +35.6 | **1.47** | −99/+245/+48/+75/+426 | — | 0.34 |
| intraday d−1, d+1..+3 | SPY | 231 | +6.4 | 1.22 | +45/+141/−121/+154/+268 | +487 (4/5) | 0.02 |
| same | QQQ | 231 | +11.1 | 1.29 | +54/+260/−7/+234/+342 | +884 (4/5) | 0.02 |
| intraday d−1, d+1 only | SPY / QQQ | 115 | +3.0 / +5.4 | 1.09 / 1.14 | — | — | — |

Verdict, applied mechanically: **needs-product**. The only version that clears the bar is the 4-session hold on QQQ
(PF 1.47, 4/5 years), and it fails on SPY, the home instrument (3/5). There is a caveat: the multi-day
version is no better than a random 4-session QQQ hold (random holds make +19.8 bps, the rule
+35.6 bps, p = 0.34), so most of it is overnight beta, not a calendar effect. The intraday
version is the reverse. Its PF is just under the bar (QQQ 1.286, SPY 1.22, both 4/5 years, max DD
−328 / −194), but it clearly beats random sessions: random open→close averages −3.4 bps on QQQ,
these 4 days average +11.1 bps, p ≈ 0.02 on both instruments. The day-1-only variant is weaker, so the ETF-era
"day 1" concentration does not show up intraday. Keeping the pre-registered bar, this
is not a pass. It is the nearest miss of the round, and the lead may want to look at it.

## 3. #6 HKS same-clock half-hour periodicity — AAPL+AMZN+MSFT+NVDA pooled — **dead**
Kill: as a trade, net bps per half-hour clears costs (≥ 4/5 years, PF > 1.3). As a filter, the v18
trades that agree with the HKS sign beat the ones that disagree.

Predictive statistics (half-hour returns, z-scored per name and half-hour, 4 names × 13 halves × ~1,190 days):

| lag (days) | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|
| same-clock corr (pooled) | +0.017 | +0.002 | −0.017 | −0.002 | +0.004 |
| R² | 0.0003 | 0.0000 | 0.0003 | 0.0000 | 0.0000 |
| adjacent-clock placebo corr | +0.001 | −0.007 | +0.002 | −0.012 | −0.007 |
| Fama-MacBeth γ over the 4 names (t) | −0.009 (−0.9) | +0.015 (1.6) | −0.010 (−0.8) | −0.011 (−1.1) | +0.018 (1.8) |

The 40-session same-clock-mean signal has an IC of **−0.021**, and its gross edge is −0.8 bps per half-hour.
The pooled lag-1 and lag-3 correlations are opposite in sign and similar in size. Their naive t of
±4 is overstated because the four names co-move. The cross-sectional slopes, which are the HKS test
itself, are all insignificant.

| trade version | grid t-filter | n | gross bps | net bps | PF | years + |
|---|---|---|---|---|---|---|
| all 13 half-hours | 0 / 1 / 2 | 59,760 / 18,660 / 2,997 | −0.8 / −1.1 / +1.3 | −7.2 / −7.6 / −5.2 | 0.64 / 0.61 / 0.73 | 0/5 each |
| first + last half-hour | 0 / 1 / 2 | 9,192 / 2,918 / 539 | −1.6 / −2.5 / +6.2 | −8.1 / −9.0 / −0.3 | 0.73 / 0.68 / 0.99 | 0, 0, 3/5 |

Filter test on the live v18 book (436 trades): trades that agree with the HKS sign made **$3.75** per trade
(n = 204), and trades that disagree made **$4.15** (n = 232). The filter adds no value.
Verdict: **dead**. The killing numbers are the IC of −0.02 and −7.2 bps/half-hour net (best grid
point −0.3 bps, PF 0.99), and it does not work as a filter either. A 4-name cross-section cannot reproduce a
cross-sectional anomaly; the broad-STOCK cell would need ~50 names of minute bars.

## 4. #1 Zarattini-Aziz 5-min ORB — QQQ (home), SPY — **dead**
Kill: net positive ≥ 4/5 years and PF > 1.3. Rule as in the paper: the 09:30–09:35 candle's direction
sets the side (no trade on a doji). Entry at the 09:35 open, stop at the other side of the candle,
target 10R, otherwise exit at the close, one trade a day. Grid: 15-minute range. Sizing: our fixed 36 %, not the paper's 1 %-risk / 4x-leverage sizing.

| inst | OR | n | gross bps | net bps | PF | win % | per year 22/23/24/25/26 | LOYO |
|---|---|---|---|---|---|---|---|---|
| QQQ | 5 | 1,167 | +3.3 | −2.9 | 0.88 | 24 | +300/−622/−137/−298/−407 | −449 (1/5) |
| QQQ | 15 | 1,085 | +5.1 | −1.1 | 0.96 | 32 | −61/−159/−76/+238/−391 | |
| SPY | 5 | 1,184 | +1.3 | −4.9 | 0.72 | 20 | −95/−502/−531/−442/−378 | −2,132 (0/5) |

QQQ exits: 873 stops, 35 targets, 259 at the close. The median risk is 23 bps.
Verdict: **dead**: −2.9 bps/trade net, PF 0.88, 1/5 years. The gross edge of +3.3 bps is about half the
~6.2 bps round-trip cost, consistent with the independent replication's break-even at about 2¢/share
of slippage. 2022 is the only good year (a trending bear market).

## 5. #15 earnings-day gap reversal — AAPL+AMZN+MSFT+NVDA pooled — **pass-pretest (fragile)**
Kill: net positive ≥ 3/5 years and PF > 1.3 (thin sample, same bar as `research/earnings/`). The reaction session is the
session after the 8-K 2.02 filing (all four report after the close; NVDA's 2022-08-08 pre-announcement
was dropped). There are 76 events. The trade fades or follows the open-vs-prior-close gap from the 09:30 open. Exits are at 11:30 (the first grid point) or at the close.

| direction | exit | n | net bps | PF | win % | per year 22/23/24/25/26 | ex top-3 | LOYO |
|---|---|---|---|---|---|---|---|---|
| fade | 11:30 | 76 | +17.6 | 1.24 | 42 | +250/+81/−27/+608/−367 | −213 | +472 (3/5) |
| fade | close | 76 | +36.2 | **1.40** | 45 | +89/+190/−134/+1191/−280 | **+9** | |
| continue | 11:30 | 76 | −30.6 | 0.70 | 54 | −322/−154/−43/−678/+314 | | −1,466 (1/5) |
| continue | close | 76 | −49.1 | 0.64 | 54 | −160/−263/+64/−1261/+227 | | |

Verdict: **pass-pretest**, applied mechanically. Fade-to-close clears the bar (PF 1.40, 3/5 years), and LOYO
selection keeps it at +472 with 3/5 years. Fading clearly beats following the gap, which agrees with
the paper's sign. But the result is **fragile**. Removing the 3 best trades leaves +9. NVDA makes
+766 of the +1,055 (AAPL +304, AMZN +72, MSFT −87). 2025 alone is +1,191, and 2026 is −280. Max DD
is −517 and the worst day −203. The 11:30 exit, which was pre-registered as the first point, fails (PF 1.24). An engine
study would mostly measure NVDA's 2022–2025 reaction days. Since `research/earnings/` parked a
+23 bps ORB-continuation rule on the same days, the two are opposite bets on the same 76 sessions.

---

## round result
| cell | best tradable version | net bps | PF | years + | status |
|---|---|---|---|---|---|
| #9 pre-FOMC | FOMC day 09:30→14:00, SPY | −2.5 | 0.78 | 1/5 | dead (overnight paper version 3/5, decayed) |
| #17 turn of month | open→close d−1,+1..+3, QQQ | +11.1 | 1.29 | 4/5 | needs-product (4-day hold, QQQ only; ≈ random holds) |
| #6 HKS half-hour | 13 halves, 40d sign | −7.2 | 0.64 | 0/5 | dead (IC −0.02; no filter value) |
| #1 5-min ORB | QQQ, stop + 10R / close | −2.9 | 0.88 | 1/5 | dead (gross +3.3 < cost) |
| #15 earnings gap | fade open→close, 4 names | +36.2 | 1.40 | 3/5 | pass-pretest (fragile: ex-top-3 +9) |

Candidates for an engine study (the lead decides): the earnings fade, which passes the bar but is fragile and concentrated in NVDA,
and the intraday turn of month, which misses by 0.01 PF but is the only rule in the round that beats random sessions with
p ≈ 0.02 on both index ETFs. Nothing was proposed to the pipeline.
