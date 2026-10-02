# Entry-trigger study: is there a better trigger for the short book's edge? (2026-10-02)

**Question.** The two promoted short windows (row 12 / v18) fire on a 5-minute thrust confirmed hourly:
`5m thrust short` (composite ≤ −0.35, 5m lagging every other timescale by 0.1, 5m ≤ −0.5, 1h ≤ 0) and
`strong core short` (composite ≤ −0.35, 5m ≤ −0.4, 1h ≤ −0.4). Both also need SPY within ±0.2 % (`cross_1m` in
[−0.4, 0.4]) and `vpin_1m.raw_vpin ≥ 0.217`. Is there a trigger that gives more trades at the same quality, or
the same trades without the flat years (2023, 2024)?

**Answer, short.** No trigger adds volume that holds up out of sample, and none fixes 2023/2024. One dial
improves quality: tightening the thrust window's hourly condition from 1h ≤ 0 to **1h ≤ −0.15** gives
+1,714 / 278 trades / **PF 1.63** / 4 of 5 years positive (v18: +1,728 / 436 / 1.37 / 3). That is the same P&L on
36 % fewer trades. It passes `quality-config` and has been proposed as candidate #29 `thrust-1h15`. The
leave-one-year-out check picks the same end of the dial in all five folds. 2023/2024 stay flat (+55 / −56).

## 1. Design

- Five-year IEX replay (2022-01-03 … 2026-09-10), honest costs (3 bps + $0.005 per leg),
  `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, `BARS_DIR=data/bars_iex
  scripts/run_cached_sweep.sh tr_<cell> …`, one sweep at a time under `logs/.research.lock` (`run_cell.sh`).
- **Scaffold** `am.json`: both promoted windows disabled and re-added as `_am` copies (same conditions, own
  11:30 / 11:55 clock) plus the new weight-0 `trig_1m` indicator. `tr_am` reproduces `iex_v18` **trade for trade
  (436 / 436 keys, P&L to the cent)**. SPY band and VPIN floor are fixed in every window of every cell.
- **Code (one indicator file):** `crates/indicators/src/custom/trigger_context.rs` (`trigger_context`, registered,
  5 tests in `tests/custom_indicators.rs`): pullback geometry over today's last 15 1m bars (`drop_pct`,
  `bars_since_low`, `retrace`, `bounce_pct`, `last_ret_pct`) and `vpin_now` / `vpin_slope_3` / `vpin_slope_5`
  (raw VPIN recomputed on the window minus its last k bars = exactly what `vpin_1m` said k bars ago; stateless).
  `vpin.rs` untouched. Indicators cannot see the composite, so "thrust in the last N bars" is a *price* thrust
  (≥ X % high-to-low in the 15-bar window) with the 5m score still ≤ −0.4. `cargo test -p indicators` passes;
  `cargo clippy --workspace -- -D warnings` clean (the `--tests` target has one pre-existing failure in
  `session_signals.rs`).
- Thresholds for c / d set on bar *counts* in the scaffold's morning tick dump (`calibrate.py`), never on returns.
  Budget: 20 sweeps (scaffold + 19 cells) plus a two-year check of the candidate patch.

## 2. Grid (5y and per year vs iex_v18; P&L at 36 % sizing)

R = replaces a promoted window; A = adds a window on top of the unchanged two.
Gates: the pipeline's own functions (`scripts/pipeline/gates.py`) against `iex_v18`.

| cell | what | 5y P&L | n | PF | maxDD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | gates |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **iex_v18 / tr_am** | promoted | +1,728 | 436 | 1.37 | −469 | +991 | −3 | −20 | +332 | +429 | 3 | — |
| a_c30 | R composite ≤ −0.30 | +1,719 | 439 | 1.36 | −477 | +982 | −7 | −17 | +332 | +429 | 3 | — |
| a_c40 | R composite ≤ −0.40 | +1,700 | 381 | 1.41 | −491 | +1,039 | +61 | −138 | +276 | +462 | 4 | quality ✗ (PF 1.412 < 1.416) |
| a_c45 | R composite ≤ −0.45 | +1,307 | 309 | 1.37 | −510 | +994 | +158 | −256 | +233 | +180 | 4 | — |
| a_t60 | R thrust 5m ≤ −0.6 | +1,319 | 306 | 1.38 | −519 | +1,079 | +60 | −203 | +256 | +127 | 4 | — |
| b_no1h | R thrust: no 1h condition | +1,774 | 486 | 1.33 | −511 | +1,079 | +64 | −266 | +464 | +433 | 4 | **volume ✓** |
| b_1h05 | R thrust 1h ≤ −0.05 | +1,635 | 414 | 1.36 | −365 | +934 | +62 | +8 | +198 | +434 | **5** | — (n, PF) |
| b_1h10 | R thrust 1h ≤ −0.10 | +1,567 | 353 | 1.43 | −421 | +654 | −30 | −36 | +348 | +631 | 3 | — |
| **b_1h15** | R thrust 1h ≤ −0.15 | **+1,714** | **278** | **1.63** | **−348** | +644 | +55 | −56 | +396 | +676 | 4 | **quality ✓** |
| b_1h20 | R thrust 1h ≤ −0.20 | +1,268 | 204 | 1.64 | −258 | +402 | +104 | −4 | +283 | +483 | 4 | quality ✗ (n < 262) |
| b_lead1m | R thrust: 1m leads by 0.1 (literal) instead of 5m lag | −160 | 73 | 0.83 | −424 | +71 | −84 | −29 | −47 | −70 | 1 | — |
| b_lag1m | R thrust: 1m most bearish by 0.1 instead of 5m lag | +240 | 180 | 1.10 | −518 | +456 | −78 | +279 | −104 | −313 | 2 | — |
| b_1m40 † | R thrust + 1m ≤ −0.4 | +1,504 | 312 | 1.45 | −418 | +945 | +92 | −132 | +326 | +274 | 4 | quality ✓ (post hoc) |
| c_pb | R thrust → pullback (drop ≥ 0.5 %, low 2–12 bars ago, retrace 0.3–0.7, 5m ≤ −0.4) | +175 | 221 | 1.06 | −649 | +550 | −405 | −12 | +168 | −126 | 2 | — |
| d_s5 | A thrust/core with VPIN in [0.17, 0.217) and slope_5 ≥ 0.02 | +1,736 | 508 | 1.30 | −614 | +1,059 | −109 | +253 | +131 | +402 | 4 | volume ✓ |
| d_s5_03 | A same, slope_5 ≥ 0.03 | +1,825 | 504 | 1.33 | −472 | +1,103 | −61 | +255 | +132 | +396 | 4 | volume ✓ |
| d_lvl | A control: VPIN in [0.17, 0.217), **no slope** | +1,900 | 537 | 1.31 | −593 | +1,120 | −115 | +372 | +106 | +416 | 4 | volume ✓ |
| bd_no1h_s5 | b_no1h + d_s5 combined | +1,372 | 582 | 1.20 | −665 | +1,071 | −91 | −228 | +301 | +319 | 3 | — |
| e_0945_1100 | R both windows 09:45–11:00 | +599 | 282 | 1.21 | −569 | +552 | −10 | −304 | +33 | +328 | 3 | — |
| e_0930_1100 | R both windows 09:30–11:00 | +1,766 | 416 | 1.39 | −459 | +984 | +61 | −53 | +322 | +453 | 4 | — (n < 436, PF) |

Notes on the grid:
- No cell passes `additive-config`. Every "A" cell displaces base trades: its window fires minutes earlier on the
  same ticker and takes the slot.
- † `b_1m40` came from bucketing v18's own five-year trades by entry 1m score (1m in (−0.4, −0.2]: −201 / 138,
  negative 3 of 5 years), so its pass is in-sample by construction. Not proposed.
- `e_1000_1130`, `a_t70`, `d_s3_02` and `d_s5_l15` were generated but not swept. The first
  family cell made each moot: delaying entry kills the book; a stronger 5m threshold is already worse at −0.6;
  and the slope is inert (§3d).

## 3. By family: what changed and why

Trades are split against `iex_v18` (`analyze.py diff`): **kept** (same key), **removed** (base only),
**re-timed** (new entry on the ticker-day of a removed one), **new** (new ticker-days).

**a. Thrust strength.** composite ≤ −0.30 is not binding (5m ≤ −0.5 at weight 0.6 already implies it; +3 trades).
≤ −0.40 removes the composite (−0.40, −0.35] bucket (−133 / 76 / PF 0.84) → PF 1.41, 4 years, but 2024 −138.
≤ −0.45 and 5m ≤ −0.6 cut good trades too (the (−0.45, −0.40] bucket is v18's best: +605 / 86 / PF 1.89).
A stronger thrust is not a better thrust past −0.40.

**b. Confirmation timescale.** The 1h ceiling is a clean quality/volume dial:

| 1h ceiling | none | 0 (v18) | −0.05 | −0.10 | −0.15 | −0.20 |
|---|---|---|---|---|---|---|
| trades / PF | 486 / 1.33 | 436 / 1.37 | 414 / 1.36 | 353 / 1.43 | 278 / 1.63 | 204 / 1.64 |
| 5y P&L / +yrs | +1,774 / 4 | +1,728 / 3 | +1,635 / 5 | +1,567 / 3 | +1,714 / 4 | +1,268 / 4 |

PF rises (almost monotonically) as the hourly confirmation tightens; the positive-year count does not (it flips on ±50 in
2023/2024, noise at ~60 trades a year). v18's own trades by 1h at entry: (−0.1, 0] 139 / +44 / PF 1.02 (+516 in
2022, then −196 / −93 / −5 / −178); (−0.2, −0.1] 141 / +580 / 1.39; (−0.4, −0.2] 125 / +900 / 1.84 — the
hourly-neutral thrusts paid only in 2022. Swapping the 5m-lag confirmation for 1m confirmation fails both ways:
the literal 1m lead (b_lead1m, 73 trades, PF 0.83) enters on a 1m bounce inside a 5m drop; "1m most bearish"
(b_lag1m) gives PF 1.10. The 5m-lags-1m shape is part of the edge.

**c. Pullback entry ("enter the resumption, not the thrust").** It fails clearly.
c_pb trades the same ticker-days as v18 but enters on the bounce: re-timed 100 / +156 / PF 1.11 vs the 370 thrust
entries it removed (+1,427 / PF 1.36); new ticker-days −279 / 55; 2023 −405. v18's own thrust bars already sit near
the low (median `retrace` 0.20, `bars_since_low` 2); waiting for 0.3–0.7 of the drop to come back hands the move
away. The other c variants were not swept after a −1,553 first cell.

**d. VPIN slope (the tiered study's "front-runner" population).**
d_s5 / d_s5_03 pass volume-config, but the control with no slope condition (d_lvl) is *better* (+1,900 / 537 /
PF 1.31). The slope at the entry bar does not predict a crossing: on the dump, thrust/core bars with the SPY band
and VPIN in [0.17, 0.217) reach 0.217 within 5 bars 45 % of the time overall, 43 % with slope_5 < 0, 45 % with
slope_5 ≥ 0.03. The replay split reproduces the tiered study on the VWAP-fixed record:

| cell | re-timed (front-runners, median −2 min) | base trades they replace | new ticker-days |
|---|---|---|---|
| d_s5 | +1,375 / 62 / PF 5.5 | +759 | −606 / 71 / PF 0.45 |
| d_s5_03 | +1,177 / 57 | +596 | −481 / 67 / PF 0.49 |
| d_lvl | +1,469 / 87 / PF 3.6 | +528 | −768 / 100 / PF 0.49 |

The slope thins both populations equally. The added windows lose in 2025/2026 (thrust-slope subset −130 / −99).
What d_lvl shows is a VPIN-floor change (0.217 → 0.17 at lower priority) — out of scope (filters fixed) and
already judged in plan §14.1 / study B.

**e. Time of day.**
The edge is front-loaded: 228 of v18's 436 entries are before 09:45 (median 14 min after the open); 09:45–11:00
collapses to +599 / PF 1.21. Cutting only the 11:00–11:30 tail (20 trades, −38) is harmless (+1,766 / 416 / PF
1.39 / 4 yrs) — a tidy-up, not a trigger; it fails volume (n) and quality (PF).

**Near misses** (scaffold dump, thrust window; 48,648 morning bars with composite ≤ −0.35, 1,884 pass everything
→ 436 trades). Bars failing exactly one condition: VPIN 5,228 · SPY band 2,599 · 5m lag 2,179 · 1h ≤ 0 351 ·
5m ≤ −0.5 124. The trigger is rarely the binding constraint — the filters are — so loosening it adds little
(b_no1h +50 trades).

## 4. Leave-one-year-out

Selection rule: pick the cell with the best pooled PF (or P&L) on four years, then report its fifth year.
Choice set: all 19 cells + iex_v18.

| held out | chosen by PF | held-out year | chosen by P&L | held-out year | iex_v18 that year |
|---|---|---|---|---|---|
| 2022 | b_1h20 | +402 / 48 / 1.78 | b_1h15 | +644 / 60 / 2.00 | +991 / 88 / 2.10 |
| 2023 | b_1h15 | +55 / 58 / 1.09 | d_lvl | −115 / 110 / 0.92 | −3 / 91 / 1.00 |
| 2024 | b_1h15 | −56 / 62 / 0.93 | b_no1h | −266 / 118 / 0.84 | −20 / 102 / 0.98 |
| 2025 | b_1h20 | +283 / 36 / 1.93 | d_lvl | +106 / 98 / 1.09 | +332 / 80 / 1.41 |
| 2026 | b_1h20 | +483 / 32 / 4.59 | d_lvl | +416 / 96 / 1.45 | +429 / 75 / 1.67 |
| Σ held-out | | +1,167 | | +785 | +1,728 |

Reading:
- **Selecting on quality generalizes in direction.** The tight-1h end (−0.15 / −0.20) is chosen in 5 of 5 folds.
  Its held-out PF beats v18's in 2023, 2025 and 2026, and is lower in 2022 and 2024. It earns less in total
  because it trades 36–53 % less.
- **Selecting on P&L does not generalize.** The volume cells (d_lvl, b_no1h) win four-year P&L and then lose
  in the held-out year: 2023 −115, 2024 −266. Σ +785 vs +1,728.
- Restricting the choice set to the slope family picks v18 itself in 4 of 5 folds by PF; by P&L the held-out
  sum is +1,508 vs +1,728. So the volume passes in §2 (b_no1h, d_s5, d_s5_03, d_lvl) are not winners. Each one
  "gets" its fourth positive year by flipping 2023 or 2024 by about ±100 while another year gets worse.

## 5. The best cell and why it trades differently — `b_1h15` (proposed as `thrust-1h15`)

| | trades | P&L | PF |
|---|---|---|---|
| kept | 233 | +1,551 | 1.68 |
| removed | 203 | +176 | 1.07 (2022 +593; 2023 −103 · 2024 −95 · 2025 +12 · 2026 −231) |
| re-timed (median +4 min later) | 45 | +163 | 1.36 |

By window: `5m thrust short` +1,542 / 251 / PF 1.65; `strong core short` unchanged (+173 / 27).
Max drawdown improves −469 → −348.

**What kind of bar it enters on.** v18's thrust window accepts any 5-minute flush as long as the hourly score is
merely not bullish (≤ 0). About half of its entries (1h in (−0.15, 0]) are a single name breaking down on a
morning whose hourly picture is neutral: the 5m flush *is* the whole move.

`b_1h15` only takes the flush when the hourly score has already turned down (mean entry 1h −0.23 vs −0.16 for
v18). That means the hourly set (EMA-20, supertrend, ADX, VWAP distance, bandwidth) already leans bearish, so
the thrust is a continuation of an hourly decline rather than a one-off break. Those continuation thrusts are
the ones that keep paying after 2022. The neutral-hour flushes were the 2022 bear-tape trade and have been flat
to negative since. The few re-timed entries are the same episodes caught ~4 minutes later, once the hourly
score crosses −0.15. They are slightly worse per trade, which is consistent with the study's other finding (§3)
that this edge is front-loaded and every minute of delay costs.

What it does **not** do: fix 2023/2024 (+55 / −56). In those years the hourly-confirmed thrusts themselves are
only break-even (kept-set PF 1.20 / 1.13).

## 6. Verdict against the three gates

| cell | volume-config | quality-config | additive-config | LOYO | action |
|---|---|---|---|---|---|
| b_1h15 | ✗ (n 278 < 436) | **✓** PF 1.63 ≥ 1.42, 4 yrs, n ≥ 262, min yr −56 | ✗ (replaces) | direction holds (chosen 5/5) | **proposed, #29 `thrust-1h15`** |
| b_1m40 | ✗ | ✓ PF 1.45, 4 yrs | ✗ | n/a: in-sample bucket choice | not proposed (post hoc) |
| b_no1h | ✓ (PF 1.33, 4 yrs by 2023 +64) | ✗ | ✗ | fails (2024 held out −266) | not proposed |
| d_s5 / d_s5_03 | ✓ (PF 1.30 / 1.33) | ✗ | ✗ (displaces 61 / 56 base trades) | fails (v18 chosen 4/5 by PF) | not proposed: slope inert, dominated by its own control |
| d_lvl | ✓ | ✗ | ✗ | fails | not proposed: a filter change, out of scope |
| everything else | ✗ | ✗ | ✗ | | — |

**Proposed:** `python3 scripts/pipeline/pipeline.py propose --name thrust-1h15 --kind config --patch
research/trigger/thrust_1h15.patch.json --gate quality-config --notes "…"` (candidate #29, stage `proposed`;
the nightly job gates it independently).
The patch is `b_1h15.json` minus the weight-0 `trig_1m` (unused by the cell, and not in the deployed binary);
replaying that exact patch on 2024 + 2026 (tag `tr_cand_1h15`) reproduces the cell's 108 trades key-for-key and to
the cent. Live expectation: ~0.23 trades per session instead of 0.36, PF ~1.6, the same dollars — a quality
candidate, not the volume the question hoped for.

**Answer to the question.**
- *More trades at the same quality:* no. Every trigger that adds trades either adds the "genuinely new"
  sub-floor population (PF ~0.5) or flips one flat year at the cost of another, and fails out of sample.
- *The same trades without the flat years:* no. 2023/2024 are flat inside every subset tried. The best a trigger
  change does is the same P&L with fewer, better trades (b_1h15).
- The one robust lesson for future triggers is that the edge is earliest-in-the-thrust. Delaying entry by time
  (e), by waiting for a pullback (c) or by waiting for confirmation (re-timed entries in a / b) always costs.
  Entering earlier only pays on episodes that go on to confirm, and no condition available at the entry bar
  (VPIN level or slope, 1h, 1m) identifies those in advance.

## 7. Files and commands

`research/trigger/`: `make_patches.py` (regenerates every cell; kept: `am.json`, `thrust_1h15.patch.json`, every
other grid patch pruned), `run_cell.sh` / `run_queue.sh` (tags `tr_<cell>`; outputs in `data/tr_*`,
`logs/sweeps/tr_*`, untracked; scaffold dump `DUMP_TICKS=1 run_cell.sh am --dump-window-only`), `analyze.py
grid|windows|diff|loyo|bars <tags>`, `calibrate.py pullback|slope|nearmiss`; also `research/entries/summarize.py
<tag> --by-window`.
