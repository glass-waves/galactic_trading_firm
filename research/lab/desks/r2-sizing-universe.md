# r2-sizing-universe desk: sizing, per-ticker overrides, session caps, universe on row 25 (2026-11-11)

Agent `r2-sizing-universe`. Base `research/lab/specs/base/row25.json` (config row 25, research base 0.36, cap 0.45, house
costs, lookback 8, IEX cache). Train trades of the base = `frontier/na_bundle` trade for trade (ROUND2 §base), so the
free diagnostics below read `research/lab/runs/frontier__na_bundle.train.trades.csv`.

## 0. engine facts that decide what can be tested (read from the frozen source)

- **Sizing actions do not compose.** `tick_loop.rs` loops over the enabled sizing actions and the *last* one that
  returns Enter sets `size_fraction`. "volatility_scaled on top of the tier" is not expressible; a second sizing action
  replaces the tier.
- **`ticker_overrides.sizing_fraction` is a no-op on row 25.** `TickerOverrides::apply` (types/src/config.rs) rewrites
  `fixed_fractional.fraction` and `volatility_scaled.base_fraction` only, not `indicator_tiered.base_fraction`. The same
  `apply` runs live (config_watcher), so a per-ticker size override in the blob would silently do nothing. Per-ticker
  sizing here is tested as a union of ticker-subset specs (tickers replay independently; `cross_1m` is SPY-only and
  the peer fields are unused, so a subset spec reproduces the subset's trades exactly).
- **`ticker_overrides.entry_threshold` is a no-op on row 25** (it moves `scoring.entry_threshold` and
  `score_threshold_entry`; row 25 enters only through `entry_window` conditions). `exit_threshold` works (it is the
  short score exit: exit when composite ≥ −exit_threshold).
- **`max_capital_deployed_pct` is a no-op live and in replay**: it reads `market.total_deployed_capital`, which
  `data_feed/market_state.rs` sets to `None` (and the replay never fills). Live exposure is bounded only by
  `max_concurrent_positions` × `max_position_pct` (3 × 0.40 = 1.2× capital).
- **`max_concurrent_positions` is not simulated by the replay** (tickers replay independently). It can only be emulated
  on the merged trade list.
- `volatility_scaled` scales by dollar ATR / `baseline_atr` (one dollar figure for all names on split-adjusted prices):
  a price-level parameter, like MACD's `normalization_factor`.

## 1. idea list (written before any run)

| # | idea | mechanism | expected failure mode |
|---|---|---|---|
| 1 | VPIN band ladder (diagnostic) | encode the VPIN band of every entry in its size (×0.8/0.9/1.0/1.25/1.5/1.75 at 0.20/0.22/0.24/0.26/0.30/0.35, capital 100k for share resolution) to read PF per band on train, then price every tier layout from one run | bands are noisy at ~50 trades each; picking a split from it is in-sample |
| 2 | tier ×1.5 at 0.26 (coordinator) | more weight on the PF-1.96 half | pure leverage: dollars up, PF flat, dd up |
| 3 | tier split 0.22 / 0.30 | move the split | 0.22 dilutes the top tier (PF falls toward the lower half), 0.30 thins it |
| 4 | third tier (×1.5 ≥ 0.30) | if PF rises monotonically with VPIN | non-monotone bands; leverage |
| 5 | shave the bottom band (×0.75 below 0.24) | de-risk the weakest entries | lowers dollars; PF up only if the bottom band is clearly worse |
| 6 | base 0.30 (live) vs 0.36 | the live book's scale | ×1.2 leverage on the same trades, nothing else; one reference run |
| 7 | NVDA size-up (base 0.45) | NVDA ≈ 70 % of P&L, positive every year | per-name selection does not persist (round 1); concentration risk |
| 8 | AAPL down-weight (base 0.18) and AAPL cut (control) | AAPL loses every year on train | AAPL was an in-sample loser; cut = removing a loser always "wins" in-sample |
| 9 | NVDA score exit off (exit_threshold override) | NVDA's 17 ScoreExit trades −532 | frontier: holding score-exit trades does not rescue them |
| 10 | per-ticker entry threshold | — | no-op on row 25 (see §0); not run |
| 11 | max_concurrent 2 / 4, with bigger size | the cap rarely binds | emulated only (§0); 4 = the replay itself |
| 12 | max_daily_loss_pct tighter / entry_cooldown longer | stop re-entering after a loss | re-entries may be winners — check the trade list first |
| 13 | score_scaled sizing (by \|composite\|) | stronger thrust, bigger size | composite strength may not predict P&L — check first |
| 14 | volatility_scaled on top of the tier | smaller size in high vol | not expressible (§0); dollar-ATR baseline is price-level |
| 15 | add QQQ / SPY under row 25 | more trades on the most liquid tapes | round 1: ETFs dead on v18; thin |
| 16 | add one or two names | more volume | round 1: no persistence; needs a first-half/second-half split control |

## 2. free diagnostics on the base's train trades (no ledger lines; `frontier__na_bundle.train.trades.csv` = row 25)

| question | finding | consequence |
|---|---|---|
| does the 3-position cap bind? (emulated on the merged list) | cap 3 blocks 2 trades (+50); cap 2 blocks 12 (+141): 308 kept, +2469 / PF 1.72 | `max_concurrent` 4 = the replay = +50; cap 2 at ×1.25 size ≈ +3086 analytically (P&L is linear in size) — leverage at a *lower* peak exposure (2 × 0.5625 vs 3 × 0.45), not edge. Not run. |
| are same-day re-entries bad? (cooldown / daily-loss idea) | 15 re-entries, 13 of them after a loss, +386 / PF 4.32; gaps 2–284 min | a longer `entry_cooldown_ms` or a tighter `max_daily_loss_pct` removes winners. Dead; not run. |
| does composite strength predict P&L? (score_scaled idea) | corr(entry composite, pnl_pct) −0.02; 5m −0.00; 1h −0.04; composite terciles PF 1.67 / 1.50 / 2.01 (weakest tercile best) | `score_scaled` sizing has nothing to scale on. Dead; not run. |
| NVDA vs the rest, split sample (H1 = 2022-01 → 2023-09, H2 = 2023-10 → 2025-06) | NVDA H1 +936/76/2.06, H2 +901/69/1.90; AMZN +202/1.46 → +314/2.55; MSFT +491/4.16 → +66/1.33; AAPL −116/0.57 → −184/0.48. Mean pnl% per trade: NVDA 0.31/0.32, AMZN 0.13/0.31, MSFT 0.37/0.06, AAPL −0.16/−0.21 | NVDA's share is mostly *volume* (145 of 320 trades); per trade it is not reliably better than AMZN/MSFT. AAPL negative in both halves on this engine — but see §4 (round 1's holdout reversed it). |
| per-name persistence on the core engine (`universe__nt10_u37.train`, 37 names, same H1/H2) | Spearman H1 vs H2 −0.006. Names positive in H1 (n ≥ 15): H2 −927/286/PF 0.75 vs the rest −966/707/0.91. Positive in H2 → H1 −402/228/0.87 vs rest 0.86. Top 3 by H1 (CVX ORCL MRK) → H2 −279/41/0.59 | universe additions picked in-sample are noise on this engine too. Only QQQ (+102/12, +50/10) and SPY (+42/8, +35/4) are positive in both halves, on 22 and 12 trades. |

## 3. cells (15 ledger lines; research sizing 0.36 base / tier / cap stated per cell unless marked LIVE)

Base on the same windows: row 25 2024h2_25h1 **+863 / 89 / PF 1.85**; train **+2610 / 320 / PF 1.75**, dd −238, worst day −168,
ex-2022 PF 1.47 (22 +1443 · 23 +136 · 24 +650 · 25h1 +380).

| cell (spec under `research/lab/specs/r2/sizing/`) | change; sizing that applies | window | P&L / n / PF | dd / worst day | ex-best PF | per year | paired vs row 25 (raw) | notional-matched paired |
|---|---|---|---|---|---|---|---|---|
| `ladder` (diagnostic) | VPIN band coded in size, capital 100k, cap 0.63 | train | +30210 / 320 / 1.77 (×10 capital) | | | | — | — |
| `tier_x150` | ×1.5 at VPIN ≥ 0.26; base 0.36, cap 0.54 | 2024h2_25h1 | +989 / 89 / 1.88 | −249 / −199 | | | | |
| `tier_x150` | 〃 | train | **+2984 / 320 / 1.77** | −266 / −199 | 1.48 | 22 +1632 · 23 +194 · 24 +708 · 25 +450 | +374, P 0.002, corr 1.00 | ×1.114: **+75, P 0.195** |
| `tier3` | ×1.25 ≥ 0.26, ×1.5 ≥ 0.30; base 0.36, cap 0.54 | 2024h2_25h1 | +1001 / 89 / 1.92 | −248 / −199 | | | | |
| `tier3` | 〃 | train | +2888 / 320 / 1.76 | −266 / −199 | 1.49 | 22 +1564 · 23 +188 · 24 +693 · 25 +442 | +278, P 0.010 | ×1.082: +64, P 0.229 |
| `live_base` (reference) | row 25 at LIVE sizing: base 0.30, ×1.25 ≥ 0.26, cap 0.40 | train | +2131 / 316 / 1.73 | −194 / −137 | 1.46 | 22 +1177 · 23 +104 · 24 +539 · 25 +310 | — | — |
| `tier_x150_live` | ×1.5 at ≥ 0.26 at LIVE: base 0.30, cap 0.45 | train | **+2444 / 316 / 1.76** | −216 / −168 | 1.48 | 22 +1332 · 23 +152 · 24 +595 · 25 +366 | vs live_base +313, P 0.004 | ×1.116: **+67, P 0.182** |
| `p_nvda_45` + `p_b3_nonvda` (union) | NVDA base 0.45 (tier 0.5625, cap 0.5625); others 0.36/0.45 | train | +3067 / 320 / 1.77 | −310 / −201 | 1.48 | 22 +1676 · 23 +201 · 24 +730 · 25 +461 | +457, P 0.002 | ×1.118: +148, P 0.069 |
| `p_aapl_half` + `p_b3_noaapl` (union) | AAPL base 0.18 (tier 0.225); others 0.36/0.45 | train | +2766 / 320 / 1.87 | −255 / −168 | 1.55 | 22 +1504 · 23 +138 · 24 +719 · 25 +405 | +156, P 0.054 | ×0.929: +340, P 0.000 |
| `p_b3_noaapl` (AAPL-cut control) | AMZN NVDA MSFT; 0.36/0.45 | train | +2910 / 276 / 2.01 | −271 / −168 | 1.64 | 22 +1557 · 23 +139 · 24 +789 · 25 +425 | +300, P 0.057 | ×0.862: +661, P 0.000 |
| `nvda_seoff` | `--ticker-override NVDA:exit_threshold=-1.5` (score exit off on NVDA); 0.36/0.45 | 2024h2_25h1 | +859 / 89 / 1.85 | −217 / −168 | | | −4 vs base (one trade) | not confirmed |
| `u_qqq` | row 25 engine on QQQ alone; 0.36/0.45 | 2024h2_25h1 | +23 / 10 / 1.25 | | | | | |
| `u_qqq` | 〃 | train | +95 / 30 / 1.29 | −198 / −112 | 1.40 | 22 +58/17 · 23 +10/1 · 24 +39/6 · 25 −11/6 | | |
| `u_spy` | row 25 engine on SPY alone; 0.36/0.45 | 2024h2_25h1 | +31 / 5 / 2.64 | | | | | not confirmed (5 trades a year) |

"Notional-matched" (`research/lab/patches/r2/sizing/levm.py A B`, run from the repo root; `ucmp.py` there is the union-aware compare, `bands.py` decodes the ladder): row 25's P&L is scaled by the ratio of total entry
notional (cell ÷ row 25) and the paired day bootstrap is run on that. It asks "is the cell better than row 25 at
the same capital use?" The same test on row 25's own tier (row 25 vs `frontier/na_s15`, the no-tier twin) gives
+60, P 0.262: **the ×1.25 tier already in row 25 is leverage, not edge.**

Live-sizing note: at base 0.30 on $10k, four AMZN entries in Jan–Apr 2022 (pre-split, ~$3,300 a share on the raw
cache) cost more than 0.30 × capital, so they are skipped (316 vs 320 trades). Not relevant after the June 2022 split.

### VPIN band table (from `ladder`; P&L at 1.0× and $10k scale, flooring ignored)

| VPIN at entry | n | P&L | PF | mean pnl % | per year n (22/23/24/25) |
|---|---:|---:|---:|---:|---|
| 0.20–0.22 | 59 | +520 | 1.86 | 0.26 | 20/13/22/4 |
| 0.22–0.24 | 49 | −110 | 0.85 | −0.08 | 22/9/13/5 |
| 0.24–0.26 | 39 | +224 | 1.55 | 0.16 | 22/8/2/7 |
| 0.26–0.30 | 52 | +501 | 2.01 | 0.28 | 23/8/13/8 |
| 0.30–0.35 | 47 | +524 | 2.26 | 0.30 | 12/14/14/7 |
| ≥ 0.35 | 70 | +573 | 1.90 | 0.23 | 20/19/22/9 |

(4 trades decode below 0.20 — a capital-reconstruction rounding; they are counted here in 0.20–0.22.)
By year, ≥ 0.26 vs < 0.26: 2022 PF 4.06 vs 1.63; 2023 1.51 vs 0.71; **2024 1.41 vs 1.73 (reversed)**; 2025h1 2.08 vs 1.54.
The bands are not monotone (0.20–0.22 is as good as the top bands; 0.22–0.24 loses). Above 0.26 PF is flat at ≈ 2.0, so
a third tier at 0.30 buys nothing per trade. Analytic layouts priced on this table (best of 8, $10k): flat +2325/1.70;
row 25 +2725/1.74; ×1.5 @0.26 +3124/1.77; ×1.25 @0.22 +2753/1.69; ×1.25 @0.30 +2600/1.73; 3-tier +2999/1.76;
×0.75 below 0.24 +2599/1.78; ×0.75 on 0.22–0.24 only +2752/1.79. All PF 1.69–1.79: **every layout is the same book at a
different leverage.** The split at 0.26 sits at the in-sample best of the splits; moving it either way lowers PF.

## 4. reading

1. **Sizing levers are leverage.** Every tier cell raises dollars in proportion to notional, PF moves ≤ 0.04, dd and
   worst day grow with size. Raw paired P values of 0.002 are what any ×k on a PF-1.75 book produces; at matched notional
   the gains are +60…+75 (P 0.18–0.26). The ×1.25 tier in row 25 is itself leverage by this test.
2. **Per-ticker sizing cannot be deployed today.** `ticker_overrides.sizing_fraction` does not touch `indicator_tiered`
   (§0), live or replay. NVDA-up (+148 at matched notional, P 0.069) is the strongest per-ticker result, and it is also
   concentration: NVDA becomes ~75 % of P&L.
3. **AAPL.** Cutting or halving AAPL wins in-sample by construction (it is the in-sample loser, negative both halves on
   this engine). Round 1 already answered the out-of-sample question: `universe/nt10_b3` (core without AAPL) was holdout
   PF 1.86 / 95 vs 2.02 / 108 for the core with AAPL (REPORT.md finalists table) — **AAPL's holdout trades were net
   positive, and dropping it made the holdout worse.** I do not nominate the cut or the down-weight. The cut also fails
   T's target-window count (≈ 131 of 150 trades if AAPL's share holds).
4. **Universe.** QQQ under row 25 is 30 trades / +95 / PF 1.29 over 3.5 years, SPY 5 a year. Both fail A's ≥ 60 added
   trades by construction, and pooled with row 25 they lower PF. The split-sample control on the 37-name core-engine run
   says names picked on one half of train lose on the other (§2). No name cell was run beyond these.
5. **Session knobs.** `max_concurrent_positions` barely binds (2 trades at 3); `max_capital_deployed_pct` is dead code
   live; re-entries are the book's best trades, so longer cooldowns and tighter daily-loss stops cut winners;
   `entry_threshold` overrides are no-ops on row 25; NVDA score exit off changes one trade (frontier: HISTORY D2 holds).

## 5. dead ends

Tier split 0.22 / 0.30, third tier, bottom-band shave (priced on the ladder: PF ±0.05, no edge); `score_scaled` (no
signal in composite strength); `volatility_scaled` on top of the tier (not expressible, dollar-ATR baseline);
cooldown / daily-loss (re-entries win); `max_concurrent` 2/4 (cap barely binds; bigger size at cap 2 = leverage); AAPL cut
or down-weight (in-sample loser; reversed on round 1's holdout); NVDA score exit off (one trade); QQQ / SPY / name
additions (thin, no persistence).

## 6. engine proposals (not implemented; binary frozen)

1. `TickerOverrides::apply` should set `indicator_tiered.base_fraction` (and `score_scaled` min/max) for
   `sizing_fraction`, or reject the key when no sizing action takes it. Today a per-ticker size override in a blob is
   silently ignored live. A per-ticker `max_position_pct` would be needed to size one name above the cap.
2. Sizing should compose (a base action × multipliers) rather than "last enabled sizing action wins", or the registry
   should refuse two enabled sizing actions.
3. `max_capital_deployed_pct` should be enforced in the live main loop (it reads `total_deployed_capital`, which is always
   `None`), or be removed from the session schema.
4. `volatility_scaled` should take ATR as a percentage of price (scale-free), the same fix as MACD's `normalization_pct`.
5. The replay should simulate `max_concurrent_positions` across tickers. It barely binds for row 25, but a 5+-name book
   cannot be judged without it.

## nominations (best of 15 ledger runs and 8 analytically priced tier layouts; programme ledger as of this writing)

**1. `research/lab/specs/r2/sizing/tier_x150.json` (research) / `tier_x150_live.json` (live values). Targets T, and it
is labelled LEVERAGE.** The patch re-declares `sizing_tiered_vpin` with ×1.5 at VPIN ≥ 0.26 (`patches/r2/sizing/tier_x150.json`,
base 0.36, cap 0.54; the live form is `tier_x150_live.json`, base 0.30, `session.max_position_pct` 0.45). Nothing else changes.
- Train, research: +2984 / 320 / PF 1.77, dd −266, worst day −199, ex-2022 PF 1.48. Years 22 +1632 · 23 +194 · 24 +708 · 25h1 +450.
  Paired vs row 25 +374, P 0.002, corr 1.00. **Notional-matched +75, P 0.195.**
- Train, live values: +2444 / 316 / PF 1.76 vs live row 25 +2131 / 316 / 1.73. Notional-matched +67, P 0.182.
- Why: this is the coordinator's listed lever, and it is the only sizing cell that is deployable as a blob change.
  On paper it passes T's train clauses (P < 0.05, PF ≥ 1.6, no year < −300, ex-best ≥ 1.4, n ≥ 250). It passes them
  **because the raw paired test rewards any up-scaling of a profitable book.** The coordinator should judge it at matched
  notional, or treat it as a risk-budget decision (≈ +11 % notional, ≈ +12 % dd), not as an edge finding.
- **Holdout expectation (written before any reveal):** the same 111 trades as row 25. P&L ≈ 1.10–1.15 × row 25's holdout
  (+899 → ≈ +1000), PF within ±0.04 of row 25's 2.00, worst day ≈ 1.2 × row 25's. Raw paired P(diff ≤ 0) < 0.10 if
  ≥ 0.26 trades are net positive on the holdout. Matched-notional diff ≈ 0 ± 100. In 2024 the lower half beat the upper
  half, so a holdout where the tier loses is about 1 in 4.

**2. Not nominated, flagged for an engine change: NVDA size-up** (`p_nvda_45.json+p_b3_nonvda.json`, a union). It is the
strongest per-ticker result at matched notional (+148, P 0.069; raw +457, P 0.002; PF 1.77). It cannot be deployed:
`ticker_overrides.sizing_fraction` ignores `indicator_tiered` (engine proposal 1), and it raises NVDA to ~75 % of P&L. Per
trade, NVDA is not reliably better than AMZN or MSFT (§2). Expectation if ever run: matched-notional diff ≈ 0, raw diff
positive iff NVDA's holdout trades are positive.

**3. None.** AAPL cut / down-weight: round 1's holdout already reversed it (§4.3). Universe and session knobs: §4.4–4.5.

### most important negative results
- Row 25's own ×1.25 tier is leverage at matched notional (+60, P 0.26 vs `frontier/na_s15`). Every tier layout is PF
  1.69–1.79 on train, and the upper-band PF edge reverses in 2024.
- AAPL: the in-sample drag (−300, negative in both halves) did not hold on round 1's holdout (nt10_b3 holdout PF 1.86 vs 2.02 with AAPL).
- QQQ under row 25: 30 trades / +95 / PF 1.29 on train. SPY: about 5 a year. On the core engine, names picked on half of
  train lose on the other half (H1 picks → H2 PF 0.75 vs 0.91 for the rest; Spearman −0.006).
- `max_concurrent` barely binds (2 trades at 3). Re-entries are +386 / PF 4.3, so cooldown and daily-loss knobs cut
  winners. Composite strength does not predict P&L (corr −0.02), so `score_scaled` has nothing to scale on.
- Silent no-ops to fix before anyone relies on them live: `ticker_overrides.sizing_fraction` and `entry_threshold`
  on row 25, `max_capital_deployed_pct`, and a second sizing action, which replaces the tier rather than stacking on it.
