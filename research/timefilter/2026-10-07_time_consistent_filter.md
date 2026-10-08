# Time-consistent market filter: does the rest of the session hold the same edge? (2026-10-07)

**Question.** 346 of v18's 436 entries fall in 09:30–10:00 ET. Is the SPY-flat filter (session return within ±0.2 %)
a clock — and the VPIN floor (raw ≥ 0.217)? Would filters with the same meaning at any time of day open the session?

**Answer, short.** The SPY band is a clock; VPIN is not. The edge does **not** extend past 11:00 under any filter
(every afternoon cell loses from 11:30 on). What the clock hid is a real 10:00–11:00 extension of the morning edge:
band SPY's session return ÷ sqrt(minutes since the open) at ±0.05 (= v18's ±0.2 % at 09:45), keep v18's own band
before 10:00. That cell (`c_sq05_1100`) gives **+2,195 / 460 trades / PF 1.46 / 5 of 5 years / maxDD −481** vs
v18 +1,728 / 436 / 1.37 / 3 / −469, beats v18 in every leave-one-year-out fold and holds at one-minute live lag.
Proposed as pipeline candidate **#44 `spy-sqrt-band`** (volume-config). The volume gain is modest (+24 trades,
+8 sessions with a trade): mostly quality.

## 1. Design

- Five-year IEX replay (2022-01-03 … 2026-09-10), honest costs (3 bps + $0.005 per leg), `--sizing-fraction 0.36
  --max-position-pct 0.36 --cross-index SPY`, `BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh tf_<cell> …
  --patch-json <cell>.json` via `run_cell.sh`, one sweep at a time under `logs/.research.lock`.
- **Scaffold** `am.json`: both promoted short windows disabled and re-added as `_am` copies (own 11:30 / 11:55
  clock), session opened with the `session` key (15:30 / 15:55), weight-0 `vpin_p` (pctile_window 1950) for the dump.
  `tf_am` reproduces `iex_v18` **436 / 436 trade keys, P&L to the cent**; swept with `DUMP_TICKS=1` (every bar
  09:30–16:00, 4,700 ticker-days).
- **Code (one metadata key):** `cross_context.rs` emits `index_session_ret_per_sqrt_min` = SPY session return % ÷
  sqrt(minutes since 09:30 ET incl. the current bar, clamped 1…390); 4 unit tests in the file (EDT/EST, clamping,
  score unchanged). `cargo test -p indicators` and `cargo clippy --workspace -- -D warnings` clean (`--tests` keeps
  the pre-existing `session_signals.rs` failure). Release binary rebuilt under the lock.
- Budget: 20 sweeps (scaffold + 17 cells + 2 live-lag checks) plus a two-day patch parity check.
- k = 0.05 was **set before any sweep by calibration, not returns**: v18's median entry is ~15 min after the open
  and 0.2 % / sqrt(16) = 0.05. The dial was then swept at 0.04 / 0.05 / 0.06.

## 2. Diagnosis (scaffold dump; `diag.py`, full output in `diag_out.txt`)

Trigger bars: thrust or strong-core conditions, recomputed offline from the dumped scores. Entries: non-overlapping
samples (≥ 30 min apart per ticker-day). Forward returns: the short's gross close-to-close bps (round trip ≈ 8 bps).

**A. Who blocks, by half-hour** (25,039 trigger bars, all years)

| half-hour | trigger bars | band fails | VPIN fails | both pass → entries | SPY abs session ret p50 | raw VPIN p50 / p80 |
|---|---|---|---|---|---|---|
| 09:30 | 8,887 | 46 % | 71 % | 1,463 → 356 | 0.18 % | 0.128 / 0.268 |
| 10:00 | 6,924 | 71 % | 84 % | 307 → 80 | 0.37 % | 0.100 / 0.197 |
| 10:30 | 2,095 | 81 % | 70 % | 141 → 39 | 0.47 % | 0.145 / 0.270 |
| 11:00 | 1,483 | 85 % | 66 % | 84 → 26 | 0.51 % | 0.162 / 0.274 |
| 11:30–12:30 (3) | 2,148 | 83 % | 68 % | 96 → 35 | 0.56–0.88 % | ~0.15 / 0.27 |
| 13:00–15:30 (5) | 2,648 | 86 % | 66 % | 129 → 36 | 0.69–1.10 % | ~0.15 / 0.29 |

- **The band is a clock:** its fail rate climbs 46 % → 71 % → 81–93 %, tracking SPY's typical distance from the
  open (0.18 % → 0.37 % → 0.5–1.1 %).
- **VPIN is not a clock:** fail rate 58–84 % with no trend, median / p80 flat after the 10:00 post-open lull. A
  time-bucketed VPIN floor has nothing to correct, so none was swept (task item 3).
- **The trigger itself is front-loaded:** 63 % of trigger bars fall before 10:30.

**B. Forward short return by half-hour and filter state** (entries, fwd 60 min, mean bps (hit rate))

| half-hour | both pass | band fails, VPIN ok | band ok, VPIN fails | both fail |
|---|---|---|---|---|
| 09:30 | 356: **+12.8 (58 %)** | 270: −7.4 (51 %) | 570: +14.3 (55 %) | 504: +0.9 (53 %) |
| 10:00 | 80: −3.4 (49 %) | 162: +5.0 (56 %) | 267: +8.9 (56 %) | 464: −0.5 (54 %) |
| 10:30 | 39: +9.2 (56 %) | 108: +8.8 (59 %) | 65: +19.8 (57 %) | 207: +7.0 (58 %) |
| 11:00 | 26: +10.2 (42 %) | 88: −3.2 (47 %) | 27: +14.6 (48 %) | 137: +1.2 (49 %) |
| 11:30–15:30 (with VPIN floor; any SPY filter tried) | | | | −16 … +5, below cost |

Only at the open does the band separate good bars from bad (+12.8 vs −7.4); from 10:00 on it mostly removes bars
that pay as well as the ones it keeps. After 11:30 no market filter tried is above cost over a clock range (table C
in `diag_out.txt`; with the VPIN floor 11:30–13:00 is −2 to −16 bps, 13:00–15:30 −5 to +5); the positive single
half-hours (e.g. 15:00) have n ≤ 74 and do not hold across neighbours.

**E. Time-scaled band** |SPY session % / sqrt(min)| ≤ k, with VPIN ≥ 0.217 (entries / fwd60 bps):

| filter | 09:30 | 10:00 | 10:30 | 11:00 | 10:00–11:30 by year (n / bps) |
|---|---|---|---|---|---|
| band ±0.2 % (v18) | 356 / +12.8 | 80 / −3.4 | 39 / +9.2 | 26 / +10.2 | 27/+39 31/−8 35/−20 15/+22 23/+9 |
| k = 0.04 | 315 / +15.4 | 96 / +4.8 | 63 / +12.2 | 53 / −0.9 | 38/+33 40/−5 48/−11 26/+15 36/+17 |
| **k = 0.05** | 344 / +15.1 | 118 / +9.2 | 70 / +14.1 | 63 / +3.4 | 54/+20 46/+3 55/+5 30/+9 41/+11 |
| k = 0.06 | 375 / +14.3 | 133 / +9.0 | 79 / +11.7 | 71 / +1.6 | 63/+19 51/+5 63/+1 33/+7 46/+7 |
| SPY 15-min abs ret ≤ 0.10 % | 409 / +7.1 | 97 / +10.8 | 57 / +11.1 | 47 / −7.9 | 51/+21 40/+8 40/−8 25/−2 28/+12 |

So: the 10:00–11:00 edge exists and a time-consistent band roughly doubles its entries at a better forward return;
11:00–11:30 is marginal; after 11:30 nothing works. A rolling 15-min band cannot replace the open filter
(`index_ret_15m` is 0 before 09:45, so the open goes unfiltered).

## 3. Grid (5y and per year; P&L at 36 % sizing; gates from `scripts/pipeline/gates.py` vs iex_v18)

Clocks: →11:30 = v18's; `c_` = v18's band 09:30–10:00, new filter from 10:00 (`_pm` windows, priorities 30 / 40);
`_1100` = entries stop at 11:00; `x1300` / `x1530` = entries to 13:00 / 15:30, exit 15:55. Exit stack unchanged.

| cell | market filter / clock | 5y | n | PF | maxDD | worst day | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | gates (vol / qual) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **iex_v18 = tf_am** | band ±0.2 % →11:30 | +1,728 | 436 | 1.37 | −469 | −123 | +991 | −3 | −20 | +332 | +429 | 3 | ✗ / ✗ |
| x1300 | band, →13:00 | +1,469 | 462 | 1.29 | −620 | −123 | +926 | −13 | −156 | +269 | +443 | 3 | ✗ / ✗ |
| x1530 | band, →15:30 | +1,331 | 488 | 1.24 | −628 | −123 | +811 | +20 | −196 | +242 | +454 | 4 | ✗ / ✗ |
| r15_15 | SPY 15m ≤ 0.15 % →11:30 | +893 | 565 | 1.14 | −755 | −162 | +1,188 | +44 | −209 | −236 | +107 | 3 | ✗ / ✗ |
| c_r10 | band; 10:00+ SPY 15m ≤ 0.10 % | +1,434 | 472 | 1.28 | −469 | −120 | +878 | −18 | +15 | +287 | +272 | 4 | ✗ / ✗ |
| sq04 | k 0.04 →11:30 | +1,751 | 441 | 1.39 | −524 | −105 | +1,025 | −83 | +196 | +287 | +325 | 4 | ✓ / ✗ |
| sq05 | k 0.05 →11:30 | +2,028 | 490 | 1.41 | −666 | −105 | +986 | −88 | +350 | +405 | +375 | 4 | ✓ / ✗ |
| sq06 | k 0.06 →11:30 | +1,900 | 543 | 1.34 | −652 | −103 | +1,146 | −137 | +227 | +285 | +378 | 4 | ✓ / ✗ |
| c_sq05 | band; 10:00+ k 0.05 →11:30 | +2,090 | 506 | 1.40 | −523 | −123 | +1,075 | −5 | +196 | +379 | +444 | 4 | ✓ / ✗ |
| sq04_1100 | k 0.04 →11:00 | +1,890 | 401 | 1.46 | −412 | −95 | +1,100 | +3 | +115 | +309 | +363 | 5 | ✗ / ✓ |
| sq05_1100 | k 0.05 →11:00 | +2,137 | 444 | 1.48 | −611 | −95 | +1,054 | −1 | +270 | +397 | +416 | 4 | ✓ / ✓ |
| sq06_1100 | k 0.06 →11:00 | +2,081 | 492 | 1.42 | −524 | −95 | +1,233 | −57 | +177 | +277 | +450 | 4 | ✓ / ✗ |
| **c_sq05_1100** | **band; 10:00–11:00 k 0.05** | **+2,195** | **460** | **1.46** | **−481** | −123 | +1,144 | **+77** | **+116** | +371 | +486 | **5** | **✓ / ✓** |
| sq05_1h05 | k 0.05, thrust 1h ≤ −0.05 | +1,970 | 467 | 1.42 | −547 | −103 | +956 | −9 | +346 | +301 | +376 | 4 | ✓ / ✗ |
| sq05_1h10 | k 0.05, thrust 1h ≤ −0.10 | +1,828 | 406 | 1.46 | −607 | −133 | +606 | −104 | +341 | +399 | +587 | 4 | ✗ / ✓ |
| sq05_1h15 | k 0.05, thrust 1h ≤ −0.15 | +1,881 | 333 | 1.60 | −441 | −133 | +593 | +83 | +180 | +437 | +589 | 5 | ✗ / ✓ |
| sq05_x1300 | k 0.05 →13:00 | +1,573 | 557 | 1.27 | −937 | −116 | +976 | −155 | +131 | +321 | +300 | 4 | ✗ / ✗ |
| sq05_x1530 | k 0.05 →15:30 | +1,517 | 637 | 1.23 | −1,097 | −116 | +828 | −105 | +78 | +380 | +336 | 4 | ✗ / ✗ |

**Trades per year and per half-hour of entry** (n / P&L; half-hours 09:30 · 10:00 · 10:30 · 11:00 · 11:30+):

| cell | trades 2022–2026 | sessions with a trade | 09:30 | 10:00 | 10:30 | 11:00 | 11:30+ |
|---|---|---|---|---|---|---|---|
| iex_v18 | 88 91 102 80 75 | 325 | 346 / +1,506 | 45 / +42 | 23 / +175 | 22 / +4 | — |
| x1530 | 107 99 118 84 80 | 360 | 346 / +1,482 | 45 / +50 | 23 / +239 | 22 / −116 | 52 / −325 |
| sq05 | 105 98 116 87 84 | 352 | 330 / +1,446 | 70 / +235 | 41 / +415 | 49 / −68 | — |
| sq05_x1530 | 155 122 158 103 99 | 436 | 330 / +1,440 | 70 / +245 | 41 / +462 | 49 / −178 | 147 / −452 |
| sq05_1100 | 93 92 107 76 76 | 322 | 330 / +1,446 | 70 / +235 | 41 / +415 | 3 / +40 | — |
| **c_sq05_1100** | 99 94 109 81 77 | 333 | 346 / +1,504 | 70 / +235 | 41 / +415 | 3 / +40 (10:59 signals) | — |

Reading:
- **Session extension fails with either filter.** With v18's band the afternoon adds 52 trades for −325 and
  11:00–11:30 turns negative once positions may run past 11:55. With the time-scaled band 11:30–15:30 adds 147
  trades for −452 (11:30, 12:00, 13:00, 14:00 lose −137…−328 each; the positive half-hours have 11–16 trades). The
  1h-dial afternoon variant was not swept: the dump shows no filter above cost after 11:30.
- **Rolling SPY return fails**: r15_15 loses the open's filter; c_r10's 10:00 half-hour is −81 / 55.
- **The time-scaled band works 10:00–11:00**: k 0.05 turns v18's +217 / 68 trades there into +650 / 111 (k 0.04:
  +239 / 92; k 0.06: +519 / 129). Both clocks peak in PF at the pre-registered k = 0.05. 11:00–11:30 is negative in
  every sqrt cell (−64 … −105); the entry-trigger study already found that tail flat for v18 (§3e, −38 / 20), so
  the `_1100` clock is its tidy-up, not a new fit.
- **At the open v18's band is as good as the sqrt band** (c_sq05_1100 vs sq05_1100): at 09:35 k 0.05 is a tighter
  ±0.12 %, dropping 16 open trades and adding drawdown (−611 vs −481).
- **The 1h dial still trades volume for quality**: sq05_1h15 +1,881 / 333 / PF 1.60 / 5 of 5 beats candidate #29
  `thrust-1h15` (+1,714 / 278 / 1.63 / 4) but is under the trade bar.

## 4. By-window subsets of the winner `c_sq05_1100`, and the split vs iex_v18

| window | 5y | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|---|
| 5m thrust short (09:30–10:00, v18 band) | +1,504 / 332 / 1.42 | +736 / 61 | +25 / 73 | +147 / 79 | +233 / 62 | +364 / 57 |
| strong core short (09:30–10:00) | −36 / 17 / 0.88 | +47 / 7 | −29 / 1 | −28 / 3 | −52 / 5 | +24 / 1 |
| 5m thrust short pm (10:00–11:00, k 0.05) | +490 / 97 / 1.58 | +138 / 23 | +113 / 19 | −48 / 26 | +195 / 12 | +92 / 17 |
| strong core short pm (10:00–11:00) | +237 / 14 / 3.45 | +223 / 8 | −32 / 1 | +45 / 1 | −5 / 2 | +6 / 2 |

| set vs iex_v18 | 5y | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|---|
| kept | +1,760 / 407 / 1.39 | +1,021 / 83 | +105 / 85 | −53 / 97 | +239 / 73 | +448 / 69 |
| removed (v18 only) | −32 / 29 / 0.87 | −30 / 5 | −106 / 6 | +32 / 5 | +93 / 7 | −22 / 6 |
| added: re-timed (median −9 min) | +40 / 9 / 1.63 | +1 / 3 | −44 / 2 | — | +86 / 3 | −4 / 1 |
| added: new ticker-days | +395 / 44 / 2.37 | +121 / 13 | +17 / 7 | +169 / 12 | +47 / 5 | +41 / 7 |

Removed = v18's 10:00–11:30 band entries the new filter or the 11:00 cut drops (net negative, PF 0.87). Added =
mostly new ticker-days at PF 2.37, positive in all five years. The open is identical to v18 by construction; the
additive gate fails only on `base_unchanged_trades` (29 removed).

## 5. Leave-one-year-out

**Unconstrained** (all 17 cells + v18): by four-year PF sq05_1h15 is chosen 5/5 (held-out Σ +1,881 vs +1,728, the
quality branch as in the trigger study); by P&L sq05_1100 / c_sq05_1100 / sq05_1h15, held-out Σ +1,495.

**Volume-constrained** (owner's question: best four-year PF among cells with ≥ v18's four-year trade count):

| held out | chosen (4-yr PF, n) | held-out year | iex_v18 that year |
|---|---|---|---|
| 2022 | sq05_1100 (1.30, 351) | +1,054 / 93 / 2.21 | +991 / 88 / 2.10 |
| 2023 | sq05_1100 (1.63, 352) | −1 / 92 / 1.00 | −3 / 91 / 1.00 |
| 2024 | c_sq05_1100 (1.59, 351) | +116 / 109 / 1.09 | −20 / 102 / 0.98 |
| 2025 | sq05_1100 (1.46, 368) | +397 / 76 / 1.53 | +332 / 80 / 1.41 |
| 2026 | sq05_1100 (1.45, 368) | +416 / 76 / 1.65 | +429 / 75 / 1.67 |
| Σ | | **+1,983 / 446** | +1,728 / 436 |

The same idea (sqrt band, 11:00 clock, k 0.05) is chosen in every fold. Per fold, `c_sq05_1100`'s four-year PF
beats v18's in **5 of 5** (1.27/1.57/1.59/1.46/1.41 vs 1.19/1.48/1.50/1.36/1.32) and its held-out year beats v18's in
**5 of 5** (+1,144 / +77 / +116 / +371 / +486 vs +991 / −3 / −20 / +332 / +429); sq05_1100 does so in 4 of 5 (2026
−13). `c_sq05_1100` is the candidate because its open is v18's own and its drawdown equals v18's; the two differ
by 0.02 PF.

**Live timing** (`--cross-lag 1`: SPY state one minute late):

| tag | 5y | n | PF | maxDD | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|---|---|---|---|
| tf_am_lag1 (v18) | +1,544 | 442 | 1.32 | −425 | +915 | −2 | +66 | +135 | +431 |
| tf_c_sq05_1100_lag1 | +1,926 | 468 | 1.38 | −465 | +912 | +83 | +235 | +207 | +489 |

The margin over v18 holds at lag (+382, +26 trades, +0.06 PF, 5 of 5 years). The lag costs both books about
0.05–0.08 PF, so the absolute lagged PF (1.38) sits under the owner's 1.4 bar, as v18's does (1.32).

## 6. Verdict

| cell | PF > 1.4 | ≥ 4/5 yrs | n > 436 | LOYO | action |
|---|---|---|---|---|---|
| **c_sq05_1100** | ✓ 1.46 | ✓ 5/5 | ✓ 460 | ✓ beats v18 in every fold; family chosen 5/5 | **proposed, #44 `spy-sqrt-band`** |
| sq05_1100 | ✓ 1.48 | ✓ 4 (2023 −1) | ✓ 444 | ✓ chosen 4/5 | same idea, sqrt band also at the open; not proposed separately |
| sq05 / sq06_1100 / sq05_1h05 | ✓ 1.41 / 1.42 / 1.42 | ✓ 4 | ✓ 490 / 492 / 467 | not chosen in any fold | dial neighbours; volume end of the plateau |
| sq05_1h15 | ✓ 1.60 | ✓ 5/5 | ✗ 333 | chosen 5/5 unconstrained | quality alternative to #29; not proposed (trade bar) |
| all extension / rolling cells | ✗ | | | | rejected |

**Proposed:** `python3 scripts/pipeline/pipeline.py propose --name spy-sqrt-band --kind config --patch
research/timefilter/spy_sqrt_band.patch.json --gate volume-config --notes "…"` → candidate **#44**, `proposed`.

The patch is the cell's disable list and four windows only (no weight-0 `vpin_p`, no opened session: every clock sits
inside v18's 11:30 / 11:55 session). `verify_patch.sh` replays it alone on 2024-07-30 and 2026-06-03 (each with an
open and a 10:00–11:00 trade): trade rows identical to the cell's (2 / 2, 3 / 3).

**Live caveat:** the 10:00–11:00 windows read `cross_1m.index_session_ret_per_sqrt_min`, which exists only in
binaries built after this change: `paper_trader` must be rebuilt before a shadow trial, or those windows fail safe
(never fire) and the shadow trades v18 minus its 10:00–11:30 entries.

**Answer to the question.** The SPY band was a clock, but the clock was right about the afternoon: there is no
afternoon edge for this trigger under any market filter, and the trigger rarely fires after 10:30. What it hid is
one more hour of the morning edge (10:00–11:00: 68 trades / +217 → 111 / +650), which lifts the book to PF 1.46
with every year positive — but not a large volume increase (0.39 trades per session instead of 0.37). VPIN needs
no time-of-day treatment.

## 7. Files and commands

- `research/timefilter/`: `make_patches.py` (every cell; `--pipeline CELL OUT`), `run_cell.sh` / `run_queue.sh`
  (tags `tf_<cell>`; dump: `DUMP_TICKS=1 run_cell.sh am`; lag: `run_cell.sh <cell>_lag1 --cross-lag 1`),
  `wait_cell.sh`, `build_locked.sh`, `diag.py [tag]`, `analyze.py grid|loyo|loyo_vol|windows|diff|card`,
  `tables.sh`, `verify_patch.sh`. Kept: `am.json`, `spy_sqrt_band.patch.json`, `diag_out.txt`; grid patches pruned,
  the tick dump (`data/tf_am_*_ticks.csv`, 3.4 GB) deleted.
- Code: `crates/indicators/src/custom/cross_context.rs` (one metadata key + 4 tests). No other crate touched.
