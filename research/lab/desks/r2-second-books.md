# desk r2-second-books — additive books and windows next to row 25 (2026-11-11)

Agent `r2-second-books`. Base `research/lab/specs/base/row25.json` (config row 25 at research sizing, lookback 8).
Specs `research/lab/specs/r2/books/`, patches `research/lab/patches/r2/books/`. Windows run: `2024h2_25h1` (screen),
`train` (confirm). Nothing after 2025-06-30 was run. Lookback 8 everywhere; no cell sets a warm-up (the row-23 family
carries `session.warmup_days 30` in its blob — every QQQ/noise patch here *removes* it with `"warmup_days": null`, so a
materialized row would warm 8 like the primary).

## 0. idea list (written before any run)

Structural facts that decide the design:
- the trader restarts every day (06:10 → 13:10 PT), so live warm-up is 8 calendar days = 5–6 sessions (4–5 after a
  holiday). `noise_area` needs `min_sessions` sessions with a full 09:30 start (default = `lookback_sessions` 14), so
  **row 23 at lookback 8 cannot produce a single signal** — the QQQ book does not "lose something" at 8, it loses
  everything unless the noise band is re-parameterised to fit 5 sessions (`lookback_sessions 5, min_sessions 4`).
- the replay runs tickers independently, and one ticker holds one position: any window *inside* row 25 competes with
  row 25's windows on the same ticker (pre-emption) and inherits row 25's exits (breakeven 0.5 %, ATR×7 trail, 90/40-min
  max hold) unless it overrides them; only `max_hold_ms`, `force_exit_by`, `score_exit_threshold` can be overridden.
  A separate book avoids both.

| # | idea | form | mechanism | expected failure mode |
|---|---|---|---|---|
| Q | QQQ afternoon noise-area long (row 23) re-based to lookback 8: `noise` lookback_sessions 5 / min 4; day_move 0.7 (and a 5-session-scaled 0.8); ± VPIN floor | own book (QQQ) | Zarattini-style intraday momentum: a break of the noise band in the afternoon on a high-vol day continues into the close | a 5-session sigma is noisier than 14 — the band and the day_move gate both move; the edge (thin, PF 1.5 on 229 trades) may not survive the re-parameterisation |
| M | afternoon high-vol breakout long on the four names (long-2026 `r3_q1_vpin`) at lookback 8 with the 5-session noise band; also with 1h bullish (w1); also on QQQ+SPY with the index gate 0.7 | own book | same breakout, on the names; VPIN floor removes 2022 against-trend days | at 8 days the noise band is a different object; April 2025 carried the record; may be correlated with row 25 on stress days (opposite sign = hedge, fine) |
| S | stress variants inside row 25 (tweak T, must not pre-empt): 1.0 % with 1h ≤ −0.6; 2.0 % with a longer hold (max hold 4 h, score exit off); 1.5 % with score exit off and hold to 15:55 | patch on 25 | crash-day continuation shorts; score-exit trades lose everywhere | the stress window is 2022-only (25 of 33 train trades); on the screen window it has ≈ 8 trades — little to learn there, the train run decides |
| R | stress-reversal long: SPY ≥ 1 % below prior close but back above its open (`cross_1m.index_ret_prior_close ≤ −1`, `index_session_ret ≥ +0.3`), name breaking up (5m thrust long, composite ≥ 0.35), hold to the close | own book on row 25 with the shorts disabled | HISTORY D7: "SPY touch reverses half the days" — sell-side exhaustion after a gap down; a long on the reversal half | morning longs are dead under every 1h (D1); this is a morning-ish long whose only difference is the regime filter — it may be the same losing population on fewer days |
| A | afternoon short continuation inside row 25: thrust short 11:00–13:00 (sqrt band, 1h ≤ −0.10, VPIN 0.20, flat 13:55); VWAP-reclaim-fail short 12:00–15:00 (was ≥ 0.5 % below VWAP, back within 0.15 %, 5m ≤ −0.3, 1h ≤ −0.4) | window in row 25 | trend days: names that sold off in the morning and fail to reclaim VWAP keep falling | HISTORY D3 (every half-hour after 11:30 loses), signals-new F1 (afternoon fade loses); expected dead — run as the required check with a trade diff |

Budget: ≤ 25 screen cells, ≤ 8 train confirmations.

## 1. cells (19 distinct specs / 26 ledger runs; `q3_ls5_long_dm09` and `m4_r3_ls5_dm125` written, not run)

Generator: `python3 research/lab/patches/r2/books/_make.py` (writes every patch and spec; `_row25_blob.json` is the row-25
blob it copies the stress / pm windows from). Book analysis (alone / correlation / pooled) and the in-row trade diff were
computed from the lab's cached rows with `lab._rows_for`, `lab.summarize` and `pm.paired_day_bootstrap` — the same
functions `lab.py compare` and `judge.py "a+b"` use.

Base `row25`: screen +863 / 89 / PF 1.86 (24 +482 · 25h1 +380); train **+2610 / 320 / PF 1.75**, dd −238, worst day −168
(22 +1443/119 · 23 +136/71 · 24 +650/89 · 25h1 +380/41).

### Q — QQQ afternoon noise book (row 23) at lookback 8 (own book, QQQ)

| cell | change | window | P&L / n / PF | dd | per year | note |
|---|---|---|---|---|---|---|
| (ref) meta/qqq_noise_pm_dm07, lb 22 | round-1 book | screen | +182 / 50 / 1.31 | −125 | 24 −54 · 25 +236 | corr with row25 +0.06 (not a new run; cached rows) |
| q0_asis_lb8 | row 23 as-is, lookback 8 | screen | **0 trades** | | | noise needs 14 full sessions; 8 calendar days hold 5–6 |
| q1_ls5 | noise lookback 5 / min 4, both sides | screen | −151 / 57 / 0.78 | −242 | 24 −155 · 25 +4 | |
| q1_ls5 | same | **train** | +499 / 260 / 1.21 | −327 | 22 +676 · 23 +9 · 24 −190 · 25h1 +4 | 2023_25h1 −177/135/PF 0.86; ex-2022 PF 0.86; corr +0.07 |
| q2_ls5_long | q1 long only | screen | −47 / 30 / 0.85 | −220 | 24 −151 · 25 +103 | ex-Apr-2025 −180 |
| q4_ls5_long_vp | q2 + VPIN ≥ 0.217 | screen | +131 / 16 / 2.17 | −69 | 24 −66 · 25 +197 | 16 trades, one day |
| q5_ls4_min3 | noise 4 / min 3, both sides | screen | −211 / 54 / 0.70 | −248 | 24 −201 · 25 −10 | |

**What the QQQ book loses at 8: everything.** As specified it cannot fire; re-based to a 5-session band it is a 2022 strategy
(train ex-2022 PF 0.86, 2024 −190). Round 1's book (train +982/229/PF 1.51 at lb 22) depended on the 14-session sigma.

### M — afternoon high-vol breakout long, own book (row 23 + long-2026 patches + 5-session noise band, lookback 8)

| cell | change | window | P&L / n / PF | dd | worst day | per year | ex 2025-04 | 2023_25h1 | corr w/ row25 | pooled with row25 |
|---|---|---|---|---|---|---|---|---|---|---|
| m1_r3_ls5 | r3 (dm ≥ 1.5, VPIN ≥ 0.217), 4 names | screen | +550 / 25 / 2.15 | −182 | −98 | 24 −113 · 25 +663 | −69 | | | |
| m1_r3_ls5 | | **train** | +935 / 153 / 1.42 | −278 | −150 | 22 +286 · 23 +51 · 24 −65 · 25h1 +663/14 | +316/147/1.15 | +649/84/1.50 | +0.006 | +3545 / 473 / 1.62 |
| m2_w1_ls5 | w1 (r3 + 1h std supertrend bullish) | screen | +404 / 18 / 2.61 | −112 | −91 | 24 +86 · 25 +318 | +82 | | | |
| m2_w1_ls5 | | **train** | +706 / 124 / 1.42 | −309 | −123 | 22 +315 · 23 −74 · 24 +147 · 25h1 +318/10 | +384/120/1.24 | +392/70/1.37 | +0.000 | +3316 / 444 / 1.64 |
| m5_r3_ls5_dm175 | r3 with dm ≥ 1.75 (round-1 plateau v4) | screen | +627 / 16 / 2.81 | −182 | −98 | 24 −131 · 25 +758 | −131 | | | |
| m5_r3_ls5_dm175 | | **train** | **+1071 / 97 / 1.69** | −234 | −150 | 22 +255 · 23 +126 · 24 −67 · 25h1 +758/6 | +361/92/1.24 | +817/53/1.91 | +0.004 | **+3681 / 417 / 1.73** |
| m3_idx_vp | r3 shape on QQQ+SPY, dm ≥ 0.7, VPIN | screen | +80 / 23 / 1.40 | −137 | −47 | 24 −137 · 25 +217 | | | | |
| m3_idx_vp | | **train** | +247 / 121 / 1.25 | −404 | −80 | 22 +331 · 23 −94 · 24 −207 · 25h1 +217 | +64/113/1.07 | −84/59/0.85 | +0.03 | +2857 / 441 / 1.64 |

Union vs row 25 alone on train (paired day bootstrap of pooled − base): m1 +935, P(≤0) 0.150; m2 +706, P 0.127; m5 +1071,
P 0.096. **Without the single day 2025-04-09** (the tariff-pause rally: +757 in m1/m5, +431 in m2): m1 +178, P 0.36;
m2 +276, P 0.28; m5 +314, P 0.25.

What lookback 8 costs the afternoon long (ticker-day diff vs round-1 lb-22 runs on train): r3 → m1 shares 108 ticker-days
(+1577 at lb 22, +1411 at lb 8), drops 50 (−109) and adds 44 (**−368**); w1 → m2 shares 86 (+1261 / +1088), drops 35 (−57),
adds 37 (**−274**). The 5-session band moves about a third of the decisions and the new ones lose; PF falls 1.66 → 1.42
(r3) and 1.74 → 1.42 (w1). A higher day-move gate (m5) recovers PF by trading less.

### S — stress-window variants inside row 25 (patches on config 25; judged as tweaks)

| cell | change | window | P&L / n / PF | paired vs row25 | trade diff |
|---|---|---|---|---|---|
| s1_s10_1h06 | stress SPY ≤ −1.0 %, 1h ≤ −0.6 | screen | +807 / 81 / 1.84 | −56, P 0.76 | the tighter 1h removes 8 stress trades worth +56; the 1.0 % threshold adds none |
| s2_s20_hold4h | stress ≤ −2.0 %, max hold 4 h, score exit off | screen | +916 / 86 / 1.94 | +53, P 0.021 | removes 3 losers (−28), hold +24 |
| s2_s20_hold4h | | **train** | +2344 / 303 / 1.67 | **−266, P 0.96** | removes 20 stress trades worth **+212 / PF 2.71** (17 in 2022), adds 3 (−69) |
| s3_s15_hold4h | stress 1.5 % (live), max hold 4 h, score exit off | screen | +872 / 89 / 1.86 | +9 | same entries |
| s3_s15_hold4h | | **train** | +2531 / 320 / 1.69 | **−79, P 0.72** | 320/320 entries kept; the longer hold costs −79, all in 2022 (+1355 vs +1443) |
| s4_s15_se_off | stress score exit off only | screen | +863 / 89 / 1.86 | 0 | identical to base: the −0.6 stress score exit never binds on the screen |

None of the stress changes touches the am/pm windows (every am/pm entry is kept in all four). The stress window is a 2022
lever and its 1.5 % / 2 h / −0.6 setting is already at the local optimum; the 2.0 % screen "win" was three 2025 losers,
and on train 2.0 % throws away 17 good 2022 trades.

### R — stress-reversal long, own book (row 25 with every short window, breakeven and ATR trail off; hold to 15:55)

| cell | change | window | P&L / n / PF | per year |
|---|---|---|---|---|
| r1_rev_pc10 | SPY ≤ −1 % vs prior close, ≥ +0.3 % vs open, 5m ≥ 0.5, composite ≥ 0.35, 10:00–14:00 | screen | −220 / 6 / 0.21 | 24 −280 · 25 +60 |
| r2_rev_pc075_loose | SPY ≤ −0.75 %, ≥ 0 vs open, 5m ≥ 0.4, no composite | screen | −713 / 30 / 0.37 | 24 −268 · 25 −446 |
| r3_rev_idx | r1 on SPY+QQQ, no composite | screen | −50 / 3 / 0.02 | |

Dead on the screen; not taken to train (HISTORY D1/D7 stand: the gap-down-then-recovery long on the names is the losing
morning-long population with a regime label). A "day +1 after a stress day" trigger cannot be expressed: no indicator
exposes yesterday's index return (`cross_1m.index_ret_prior_close` is today's index vs yesterday's close).

### A — afternoon short windows inside row 25 (pre-emption check)

| cell | change | window | combined P&L / n / PF | base trades kept | added population | paired vs row25 |
|---|---|---|---|---|---|---|
| a1_thrust_1100_1300 | thrust short (pm conditions) 11:00–13:00, flat 13:55 | screen | +838 / 99 / 1.77 | 89/89 (P&L change 0) | −25 / 10 / 0.72 | −25, P 0.68 |
| a2_vwap_fail | VWAP-reclaim-fail short 12:00–15:00 (≤ −0.6 % earlier, back within 0.15 %, 5m ≤ −0.3, 1h ≤ −0.4, VPIN 0.2) | screen | identical to base | 89/89 | **never fired** | 0 |
| a3_vwap_fail_loose | a2 loosened (−0.4 %, within 0.25 %, 1h ≤ −0.1, no VPIN) | screen | +733 / 106 / 1.61 | 88/89 (pre-empts one stress trade, +28) | −100 / 18 / 0.50 | −130, P 0.87 |

The afternoon windows do not pre-empt the morning windows (they cannot: entries are time-disjoint and the morning trades
are flat by 11:55), but they can pre-empt the stress window, which runs to 15:30 on the same names (a3 took one). Both
added populations lose on the screen; HISTORY D3 stands. Not taken to train.

## 2. reading

1. **Lookback 8 is the binding constraint for every afternoon book.** `noise_area` is the only afternoon edge the
   programme has found, and its sigma needs more sessions than an 8-day warm-up holds. At 5 sessions the QQQ book has no
   post-2022 edge (2023_25h1 PF 0.86) and the four-name long falls from PF 1.66–1.74 to 1.42. Raising the day-move gate
   (m5, 1.75) brings it back to PF 1.69 on 97 trades, with every year but 2024 (−67) positive, and correlation with row 25
   of +0.004.
2. **One day carries the long book.** 2025-04-09 is +757 of m5's +1071 and +757 of m1's +935. Without it m5 is
   +314 / 96 / PF ≈ 1.24 on train, and the 2025h1 row is that one day (6 trades). Round 1's long-2026 desk found the same
   (ex-April PF 1.37 at lb 22); at lb 8 the ex-April edge is thinner (1.15–1.24). It is the kind of day the strategy is
   meant to catch, but it is still one observation.
3. **Test A's "pooled PF ≥ row 25's" clause cannot be passed by any book whose own PF is under 1.75**, because pooled PF is
   a weighted mean of the two. m5 comes closest (pooled 1.73 vs 1.75); it passes every other train clause of test A
   (97 trades ≥ 60, PF 1.69 ≥ 1.3, worst year −67 ≥ −200, correlation 0.004 < 0.3). m1/m2 also pass the count / PF /
   worst-year / correlation clauses and fail the pooled clause (1.62 / 1.64). The coordinator should decide whether the
   clause is meant literally (then no book passes) or as "pooled return/risk improves" (then the union's lower drawdown
   per dollar and its +0.004 correlation are the argument).
4. **Inside row 25 there is no additive window or stress variant.** The stress window is at its local optimum, the
   afternoon short windows lose (−25 / 10, −100 / 18), and the stress-reversal long loses.

## 3. dead ends

- QQQ noise book at lookback 8 (q0 cannot fire; q1/q2/q5 lose after 2022). The indices under the long-breakout shape (m3) are PF 1.07 ex-April and lose 2023 and 2024.
- Stress 1.0 % with a tighter 1h (s1), stress 2.0 % with a long hold (s2, train −266, P 0.96), a long hold at 1.5 % (s3, −79), stress score exit off (s4, no change).
- Stress-reversal long on the names or the indices (r1–r3): PF 0.02–0.37 on the screen.
- Afternoon thrust short 11:00–13:00 (a1) and VWAP-reclaim-fail short (a2 never fires; a3 loses and pre-empts the stress window).

## 4. engine proposals

1. **Per-book history depth (most important for this desk).** Let a book declare its own warm-up without changing the
   other books' engines on the same symbol. For example, the trader fetches max(warm-up) bars once, but each book's
   `MarketStateBuilder` truncates its candle windows to its own `warmup_days`. That would let the round-1 lb-22 afternoon
   books (r3 PF 1.66, QQQ PF 1.51) run next to row 25 without moving row 25's trades. Alternative: `noise_area` reads its
   sigma from a daily-bar table (a per-time-of-day move profile) and not from the hourly window, so it needs no warm-up.
2. **A prior-session index return key** on `cross_context` (yesterday's SPY open-to-close / close-to-close). Without it
   a "day +1 after stress" trigger cannot be written.
3. Round 1's per-window ticker scope and window-scoped exits: **not needed for anything that survived here.** The only
   live idea (the afternoon long) works as its own book, where the exits are its own. A window-scoped exit would matter only
   if the afternoon long had to live inside row 25; it would still need proposal 1 for the warm-up.

## 5. nominations (best of 19 distinct specs / 26 runs on this desk; the M family is 5 cells, of which m5 is best)

### N1 — `research/lab/specs/base/row25.json+research/lab/specs/r2/books/m5_r3_ls5_dm175.json` (test A, second book)

Row 25 plus an own-book afternoon high-vol breakout long on AMZN/AAPL/NVDA/MSFT at lookback 8: row 23 + the long-2026
r3 window (noise-area upper break at a 30-min decision 12:00–15:30, above VWAP, VPIN ≥ 0.217, hold to 15:58, 2.5 % hard
stop, no score exit) with the noise band on 5 sessions and day-move ≥ 1.75. Book alone, train: +1071 / 97 / PF 1.69,
dd −234, worst day −150, worst year 2024 −67; daily correlation with row 25 +0.004; pooled +3681 / 417 / PF 1.73
(row 25 1.75), dd −333; union − row 25 +1071, P(≤0) 0.096. Ex 2025-04-09: +314, PF ≈ 1.24, P 0.25. Why: the only
additive mechanism that survives lookback 8 and the only one with ~0 correlation; dm 1.75 was round 1's plateau value
(v4), not a fresh fit, but I picked it from 3 gate values (1.5 / 1.75, plus the w1 filter) after seeing the screen.
Live note: the materialized row must carry no `warmup_days` (the patch nulls row 23's 30); it adds no warm-up to any
symbol. Its trades come from a separate simulated broker and do not interact with row 25's 3-position cap.
**Holdout expectation (written before any reveal):** about 1.5–2 trades a month → **20–35 trades** on 2025-07 → 2026-10;
added P&L **−100…+300**, PF **0.9–1.6**. Roughly a 40 % chance the book loses on the holdout, because the train
edge outside one day is PF ~1.2 on ~90 trades. Pooled with row 25 I expect PF ≈ row 25's −0.05…+0.05.

### N2 — `research/lab/specs/base/row25.json+research/lab/specs/r2/books/m1_r3_ls5.json` (test A, control for N1)

The same book at round 1's own gate (dm ≥ 1.5), i.e. without my gate pick: train +935 / 153 / PF 1.42, worst year −65,
corr +0.006, pooled 1.62, union P(≤0) 0.150; ex 2025-04-09 PF 1.15. Nominate it only as the un-dialled twin, so
the holdout can tell whether the 1.75 gate (N1) is real. **Expectation:** 35–55 trades, PF 0.9–1.4, P&L −150…+300.

No tweak (test T) nomination: nothing inside row 25 beat it (best: s3 −79, P 0.72).
