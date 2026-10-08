# a second, decoupled strategy: index intraday momentum (2026-10-07)

**question.** v18 shorts one mega-cap breaking down while SPY is calm (mornings, ~90 trades/yr,
27 % of sessions). can the same building blocks host a second book that trades the *market's*
direction on trend days (the opposite condition) and is active on many more days? literature:
Zarattini, Aziz & Barbon 2024 (noise-area breakout on SPY), Gao, Han, Li & Zhou 2018 (first
half-hour → last half-hour); notes in `docs/analysis/2026-09-15_second_window_research.md` §2-3.
§13.1 of the plan tested these only on the four names; this is the first test on the index.

**answer.** the paper's strategy does **not** survive on 2022-26 IEX bars at our costs, on SPY or
QQQ, in any of its stop / cadence / band variants (best SPY PF 0.90). Gao's rule has no gross edge
(PF 0.99 gross, 0.49 net). one narrow cell clears the owner's bar: **QQQ, decisions 12:00-15:30,
hold to 15:58, only on days whose 14-session average open-to-close move is ≥ 0.7 %** — PF 1.50,
59 trades/yr, 4 of 5 years, DD −182; daily correlation with v18 −0.03; combined with v18 it raises
active days 28 % → 45 % with a *lower* drawdown. it is proposed as pipeline candidate #46
`qqq-noise-pm-vol`, with three caveats that the gate cannot see (§8): 55 % of its P&L is 2022, it
fails at 2× cost (PF 1.21), and **it cannot trade live** until the trader warms ≥ 22 calendar days
(it warms 8; the 4-session version that fits fails).

## 1. rules as implemented

new indicator `noise_area` (`crates/indicators/src/custom/noise_area.rs`, weight 0, 9 unit tests):
- sigma(t) = mean over the last `lookback_sessions` (14) full sessions of |close at t / open − 1|;
  upper = max(open, prior close)·(1 + band_mult·sigma), lower = min(open, prior close)·(1 − …)
  (the paper's gap adjustment).
- **from the hourly window**, because the 1m window is capped at 200 bars (live and replay); the
  200 hourly candles hold ~28 sessions. sigma is exact at 10:00, 11:00 … 16:00 (clock-aligned
  hourly closes) and interpolated in sqrt(time) between them (and from 0 at 09:30). stateless.
- metadata: `pos` (|pos| ≥ 1 ⇔ outside the band; 0 inside the gap zone), `upper`, `lower`,
  `avg_move_pct`, `day_move_pct` (sigma at 16:00 = recent average open-to-close move, constant
  within a day), `sessions`, `decision` (1 on the first bar at/after each 30-min mark, ≤ 2 min late
  — IEX misses 1.6 % of SPY / 6 % of QQQ decision minutes), `vwap_pct`, `ret_open_pct`,
  `ret_first30_pct`, `ret_hour_pct`.
- check: an independent per-minute Python replica (`sim.py`, exact sigma) of the engine cell
  `na_spy` gives 1,100 trades / −896 vs the engine's 1,075 / −894.

windows (`make_patches.py`), standalone book: v18's two short windows **disabled**, plus its
breakeven monitor, 5m ATR×7 trail and both 1m-noise reject gates; 2.5 % hard stop and daily-loss
breaker kept. `session` key: entries to 15:30, flat 15:58. `tickers` = the cell's.
- `im long`: decision ∧ pos ≥ 1 ∧ vwap_pct ≥ 0 (short mirrored; the paper's stop is
  max(UB, VWAP), so an entry on the wrong side of VWAP would be stopped at once). clock 09:59 →
  15:30 (the 09:59 bar closes at 10:00), fill next bar open. exit_overrides: no score exit,
  7 h max hold, force 15:58.
- exits: `vwap_stop` (scope = the 7 h max hold) **checked every bar** — the paper checks
  max(UB, VWAP) only at decision points; no existing action can (§6) — or none (hold to 15:58).
- Gao: at the 15:29 bar, long if `ret_first30_pct` ≥ 0 else short; hold to 15:58.
- research sizing 0.36 / cap 0.36, costs 3 bps + $0.005/share per leg, five-year IEX replay,
  `--lookback-days 22` (14 sessions need ~20 trading days; at the default 8 the indicator is
  `None`). `--cross-index SPY` with `SPY` as the traded ticker works (SPY's series is loaded twice
  into the same map key; harmless; the cross context is unused here).

## 2. grid (engine sweeps, 14 of the ~15-20 budget)

| cell | rules | tr/yr | 5y P&L | PF | win | yrs+ | DD | active |
|---|---|---|---|---|---|---|---|---|
| `na_spy` | SPY, 10:00-15:30 /30 min, VWAP stop | 231 | −894 | 0.84 | 31 % | 1/5 | −1111 | 59 % |
| `na_qqq` | same, QQQ | 218 | −186 | 0.97 | 34 % | 2/5 | −554 | 56 % |
| SPY+QQQ | union (tickers run independently) | 448 | −1080 | 0.91 | 33 % | 1/5 | −1584 | 67 % |
| `na_spy_b10` | entry & stop 0.10 % beyond VWAP | 175 | −521 | 0.90 | 40 % | 1/5 | −959 | 57 % |
| `na_spy_s25` | stop 0.25 % through VWAP | 169 | −723 | 0.88 | 43 % | 1/5 | −1059 | 59 % |
| `na_spy_nostop` | hold to 15:58 | 149 | −658 | 0.89 | 51 % | 1/5 | −839 | 59 % |
| `na_spy_h60` | hourly decisions | 175 | −615 | 0.86 | 34 % | 1/5 | −1009 | 51 % |
| `na_spy_pm_hold` | SPY, from 12:00, hold | 119 | −398 | 0.89 | 50 % | 1/5 | −880 | 47 % |
| `na_qqq_pm_hold` | QQQ, from 12:00, hold | 111 | +817 | 1.22 | 53 % | 4/5 | −348 | 44 % |
| `na_qqq_dm07` | `na_qqq` + day_move ≥ 0.7 | 114 | +479 | 1.11 | 37 % | 4/5 | −326 | 30 % |
| **`na_qqq_pm_hold_dm07`** | **QQQ pm hold + day_move ≥ 0.7** | **59** | **+1095** | **1.50** | **56 %** | **4/5** | **−182** | **23 %** |
| `…_dm07_lb4` | same, 4-session noise area, lookback 8 d | 65 | +341 | 1.12 | 55 % | 1/5 | −555 | 26 % |
| `gao_spy` | Gao sign rule, SPY | 231 | −2218 | 0.49 | 36 % | 0/5 | −2218 | 92 % |

SPY base: −2.5 bps/trade net, i.e. ~+3.7 bps gross against ~6.2 bps round-trip cost; the VWAP
stop exits 716 trades for −4,876, the 359 that reach the close make +3,981. the stop variants only
move cost around. QQQ base is flat (−0.5 bps): longs PF 1.11, shorts 0.87.

**sim scan** (`sim.py`, 72 cells: SPY/QQQ × stop {every bar, decision points (the paper), none} ×
band {1.0, 1.5} × first decision {10:00, 12:00, 14:00} × every {30, 60} min): the paper's own
decision-point stop is *worse* on SPY (PF 0.78; zero-cost PF 1.19, 2026 negative even gross). the
best cell is QQQ / from 12:00 / hold / band 1.0 / 30 min (PF 1.28); 2024 is negative in every
top-10 cell. that region was taken to the engine (`na_qqq_pm_hold`, PF 1.22).

**Gao** (sim, 2022-26, 15:30 → 15:59): SPY r1-sign PF 0.99 gross / 0.48 net, both-signs-agree
1.02 / 0.47; QQQ 0.94 / 0.51 and 0.96 / 0.52; negative every year. the engine confirms on SPY.

**vol filter.** `day_move_pct` is constant within a day, so a day-level floor = the base cell's
trades on the days that pass (`analyze.py dmgrid`; the real window condition `na_qqq_dm07`
gives 531 tr / +479 vs the derived 540 / +447). on SPY it never reaches PF 1.25 (all-day
1.20 at ≥ 1.2 % on 12 trades/yr; pm-hold 1.22 at ≥ 0.7, 2/5 years). on QQQ pm-hold:

| day_move ≥ | none | 0.5 | 0.6 | 0.7 | 0.8 | 0.9 | 1.0 | 1.2 |
|---|---|---|---|---|---|---|---|---|
| tr/yr | 111 | 96 | 76 | 60 | 48 | 40 | 35 | 21 |
| PF | 1.22 | 1.24 | 1.25 | 1.47 | 1.44 | 1.54 | 1.59 | 1.57 |
| yrs+ | 4/5 | 4/5 | 4/5 | 4/5 | 4/5 | 5/5 | 4/5 | 3/5 |

0.7 is the lowest floor on the 1.44-1.59 plateau (most volume); the patch uses it.

## 3. the candidate standalone (`im_na_qqq_pm_hold_dm07`, real sweep)

| year | trades | P&L | PF | DD | active |
|---|---|---|---|---|---|
| 2022 | 118 | +600 | 1.59 | −182 | 47 % |
| 2023 | 57 | +168 | 1.54 | −116 | 23 % |
| 2024 | 24 | −22 | 0.89 | −77 | 10 % |
| 2025 | 39 | +227 | 1.48 | −157 | 16 % |
| 2026 (to 09-10) | 37 | +122 | 1.64 | −65 | 21 % |
| **all** | **275 (59/yr)** | **+1,095** | **1.50** | **−182** | **23 %** |

win 56 %, +11.8 bps/trade net, worst day −90, one trade per active day. long 142 tr +871
PF 1.84; short 133 tr +224 PF 1.20. exits: session close 272 (+1,358), 2.5 % hard stop 3 (−263).

## 4. decoupling and the combined book (1,175 common sessions)

| | v18 (`iex_v18`) | candidate | v18 + candidate | spy-sqrt-band (`tf_c_sq05_1100`) | sqrt-band + candidate |
|---|---|---|---|---|---|
| 5y P&L | +1,728 | +1,095 | **+2,823** | +2,195 | **+3,290** |
| PF | 1.37 | 1.50 | **1.41** | 1.45 | **1.47** |
| max DD | −469 | −182 | **−390** | −481 | **−351** |
| worst day | −123 | −90 | −123 | −123 | −123 |
| active days | 325 (28 %) | 275 (23 %) | **524 (45 %)** | 333 (28 %) | 526 (45 %) |
| years + | 3/5 | 4/5 | 4/5 (2024 −42) | | 5/5 |

daily P&L correlation (zero-filled, all sessions): −0.031 vs v18, −0.024 vs spy-sqrt-band; on the
76 shared days −0.064. overlap 76 days; on the 33 shared days v18 lost, the candidate won on 15.
(spy-sqrt-band's tag `cand_44` is not swept yet; `tf_c_sq05_1100` is the same patch, verified
identical in the time-filter round.) the base cells are just as uncorrelated (`na_spy` +0.01) —
decoupled by construction — but they lose money, so combining only dilutes v18 (v18 + `na_spy`:
PF 1.08, DD −827).

## 5. LOYO

- **rule choice** (72-cell sim scan, pick the best cell on four years, score the fifth): the
  same cell is picked in every fold (QQQ / 12:00 / hold / band 1.0 / 30 min); held-out
  +798 / −3 / −29 / +133 / +94 (stitched +993). stable choice, but its unfiltered edge outside 2022
  is thin.
- **day_move floor** (QQQ pm-hold, pick the floor with the best PF on four years, ≥ 40 trades/yr):
  2022 → 0.7 (held-out PF 1.60), 2023 → 0.9 (1.25), 2024 → 1.0 (0.43, 4 trades), 2025 → 0.9
  (1.64), 2026 → 1.0 (2.39); stitched held-out 178 trades, +874, **PF 1.55**, 4/5 years positive.
  the picked value moves within 0.7-1.0, i.e. along the plateau.
- **ticker**: the same rules on SPY (pm-hold ≥ 0.7) give PF 1.22, 2/5 years. QQQ-specific.

## 6. cost sensitivity

| cell | 1× (3 bps + $0.005) | 2× (6 bps + $0.01) |
|---|---|---|
| `na_qqq_pm_hold` | +817, PF 1.22, 4/5 | −284, PF 0.93, 1/5 |
| candidate | +1,095, PF 1.50, 4/5 | +511, PF 1.21, 4/5 (2024 −72); combined with v18 PF 1.31, DD −421 |

for SPY/QQQ the default is conservative (spread ≈ 0.2 bps; the paper charges ~0.1 bps), so 2× is
a stress, not the expectation. even at zero cost the SPY base is only PF 1.19 (sim), so no cost
assumption rescues the paper's full-day strategy here.

what the building blocks could not express: the paper's stop is checked only at decision points
and trails max(UB, VWAP); `vwap_stop` checks every bar and knows only VWAP. the sim shows the
decision-point stop does not help on 2022-26 data, so a new exit action is **not** recommended.

## 7. verdict against the bar

| owner's bar | candidate |
|---|---|
| PF > 1.4 standalone | 1.50 ✓ |
| ≥ 4 of 5 years positive | 4/5 ✓ (2024 −22 on 24 trades) |
| LOYO-stable | rule pick identical in every fold; floor 0.7-1.0 plateau; stitched held-out PF 1.55 ✓ |
| materially more active days with v18 | 28 % → 45 % of sessions (+61 %) ✓ |
| combined DD not out of proportion | −469 → −390 (better); PF 1.37 → 1.41 ✓ |

clears the bar on paper and is proposed (#46, gate `quality-config`). **the existing gates compare
a candidate with the promoted config swept on the candidate's tickers — here v18 on QQQ, a
different strategy — so the gate's pass/fail is not the verdict for a standalone book; the lead /
owner should read it against §4 instead.**

## 8. caveats (read before shadowing)

1. **live blocker.** 14 sessions need ≥ 22 calendar days of warmup; `paper_trader` warms 8
   (`WARMUP_LOOKBACK_DAYS`, `crates/data_feed/src/main.rs:52`). a shadow book would compute `None`
   all day and never trade (checked: the patch at `--lookback-days 8` takes 0 trades on 2025-02-27).
   the 4-session version that fits fails (PF 1.12, 1/5). a data_feed change (outside this round)
   is required before the shadow stage means anything; the gate sweep is fine (`_sweep_args`).
2. **2022 concentration.** +600 of +1,095. ex-2022: 157 trades, +495, PF ≈ 1.4; 2024 is a
   ~flat year with 24 trades (low vol keeps the floor shut).
3. **selection.** the cell is the best region of a 72-cell scan × 8 floors × 2 tickers; LOYO
   (§5) is the guard, but SPY's failure says "QQQ afternoon trend on volatile days", not "index
   momentum". long side carries most of it (PF 1.84 vs 1.20) during a QQQ bull market.
4. **volume.** 59 trades/yr (fewer in 2024-26: 24-39); more active days *combined*, but not the
   "fires most days" of the hypothesis: the unfiltered strategies that fire most days lose.
5. fails 2× costs on its own bar (PF 1.21), stays positive 4/5 years.

## 9. commands

```bash
set -a; source .env; set +a
research/index_momentum/build_locked.sh                       # release backtest under the lock
python3 research/index_momentum/make_patches.py [cell ...]     # writes research/index_momentum/<cell>.json
research/index_momentum/run_cell.sh na_qqq_pm_hold_dm07        # tag im_<cell>, LOOKBACK=22 default
TAG=im_na_qqq_pm_hold_dm07_c2 COST_ARGS="--slippage-bps 6.0 --half-spread 0.01" \
    research/index_momentum/run_cell.sh na_qqq_pm_hold_dm07    # 2x costs
LOOKBACK=8 research/index_momentum/run_cell.sh na_qqq_pm_hold_dm07_lb4
python3 research/index_momentum/analyze.py card|grid|decouple|dmgrid|loyo im_na_qqq_pm_hold [--dm-min 0.7]
python3 research/index_momentum/analyze.py grid im_na_spy+im_na_qqq      # two-ticker book = union
python3 research/index_momentum/sim.py QQQ --stop none --start 150 [--band 1.0 --every 30 --cost-mult 0]
python3 research/index_momentum/make_patches.py --pipeline na_qqq_pm_hold_dm07 research/index_momentum/qqq_noise_pm_vol.patch.json
research/index_momentum/verify_patch.sh im_na_qqq_pm_hold_dm07 research/index_momentum/qqq_noise_pm_vol.patch.json 2023-01-06 2025-02-27
#   -> both identical (1 trade each)
python3 scripts/pipeline/pipeline.py propose --name qqq-noise-pm-vol --kind config \
    --patch research/index_momentum/qqq_noise_pm_vol.patch.json --gate quality-config --notes "..."   # -> #46
```
