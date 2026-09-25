# Study D — relative-strength long (spread bet vs SPY)

**Question.** The long book (disabled since v15) lost on the "morning drift" bet even with the SPY-flat + VPIN
filters. Does a *spread* version work — long a name that is outperforming SPY since the prior close while SPY
itself is flat — and how much volume does it add on top of the v18 shorts?

**Design.** IEX 1-minute cache, 2022-01-01..2026-09-10, honest costs (3 bps slippage + $0.005 half-spread per leg),
`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`. Every patch adds `prior_day_levels` as `pdl_5m`
(weight 0, `scale_pct` 0.005), whose metadata `rel_close_pct` = (name's return since prior close − SPY's) × 100.
The promoted v18 short windows stay enabled in every cell.

- **Part 1** (`rs_win_x<X>`): the three v17-filtered long windows from `research/entries/variants/long_filters.json`
  (`window_5m_thrust_x`, `window_candle_reversal_x`, `window_strong_core_x`; composite/timescale conditions + SPY-flat
  ±0.4 + VPIN ≥ 0.217) each get `pdl_5m.rel_close_pct ≥ X`, X ∈ {0.3, 0.5, 0.8, 1.2}.
- **Part 2** (`rs_solo_x<X>`): originals stay disabled; one long window `window_rs_long` (priority 30):
  composite ≥ 0.2, cross_1m ∈ [−0.4, 0.4], vpin_1m.raw_vpin ≥ 0.217, rel_close_pct ≥ X, X ∈ {0.5, 1.0, 1.5}.
- **Follow-ups** on X = 0.5 (best of parts 1–2): `rs_solo35_x0.5` (composite ≥ 0.35), `rs_novpin_x0.5` (VPIN
  condition removed), `rs_pair_x0.5` (adds `window_rs_short`: composite ≤ −0.2, same SPY-flat + VPIN, rel_close_pct ≤ −0.5).
- Reference: `rl_none` = the same long_filters.json on the IEX cache with no RS condition (long subset +775 / 1,249 / PF 1.06).

## Grid (combined book; per-year cells are P&L (PF); Δ vs iex_v18 = +2,154 / 407 / PF 1.48)

| tag | X | variant | 5y P&L | n | win% | PF | maxDD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | Δn / ΔP&L | LONG 5y P&L / n / PF / +yrs / worst yr |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| iex_v18 | — | baseline (shorts only) | +2154 | 407 | 39 | 1.48 | −526 | +1565 (2.98) | −181 (0.83) | −93 (0.92) | +451 (1.57) | +413 (1.60) | 3 | — | — |
| rl_none | — | 3 filtered long windows, no RS | +2926 | 1656 | 35 | 1.16 | −1215 | +2509 (1.77) | −151 (0.96) | −2 (1.00) | −204 (0.94) | +775 (1.29) | 2 | +1249 / +772 | +775 / 1249 / 1.06 / 4 / −652 |
| rs_win_x0.3 | 0.3 | 3 filtered long windows + rel≥X | +3458 | 1484 | 36 | 1.21 | −800 | +2476 (1.86) | +79 (1.02) | +55 (1.01) | +93 (1.03) | +755 (1.29) | 5 | +1077 / +1304 | +1303 / 1077 / 1.11 / 4 / −356 |
| rs_win_x0.5 | 0.5 | 3 filtered long windows + rel≥X | +3435 | 1403 | 35 | 1.22 | −800 | +2416 (1.90) | −52 (0.99) | +281 (1.07) | +56 (1.02) | +735 (1.28) | 4 | +996 / +1281 | +1284 / 996 / 1.12 / 4 / −393 |
| rs_win_x0.8 | 0.8 | 3 filtered long windows + rel≥X | +2931 | 1230 | 34 | 1.21 | −817 | +2082 (1.86) | −246 (0.93) | +295 (1.09) | +283 (1.12) | +516 (1.21) | 4 | +823 / +777 | +778 / 823 / 1.08 / 3 / −166 |
| rs_win_x1.2 | 1.2 | 3 filtered long windows + rel≥X | +2845 | 1038 | 34 | 1.24 | −636 | +2378 (2.21) | +79 (1.03) | +131 (1.05) | +141 (1.06) | +115 (1.05) | 5 | +631 / +691 | +693 / 631 / 1.09 / 3 / −309 |
| rs_solo_x0.5 | 0.5 | solo rs long (comp≥0.2, SPY-flat, VPIN) | +3443 | 1941 | 35 | 1.17 | −880 | +2035 (1.51) | +25 (1.01) | +288 (1.06) | +69 (1.02) | +1026 (1.31) | 5 | +1534 / +1289 | +1289 / 1534 / 1.08 / 4 / −382 |
| rs_solo_x1.0 | 1.0 | solo rs long | +2665 | 1400 | 34 | 1.17 | −1002 | +2212 (1.83) | +78 (1.02) | +49 (1.01) | −63 (0.98) | +388 (1.14) | 4 | +993 / +511 | +503 / 993 / 1.05 / 3 / −515 |
| rs_solo_x1.5 | 1.5 | solo rs long | +1857 | 1026 | 35 | 1.15 | −1215 | +2326 (2.10) | −350 (0.87) | −278 (0.91) | −60 (0.97) | +219 (1.10) | 2 | +619 / −297 | −298 / 619 / 0.96 / 1 / −512 |
| rs_solo35_x0.5 | 0.5 | solo, composite_min 0.35 | +3505 | 1728 | 35 | 1.19 | −773 | +2101 (1.60) | −12 (1.00) | +290 (1.07) | +126 (1.04) | +1000 (1.34) | 4 | +1321 / +1351 | +1349 / 1321 / 1.10 / 4 / −325 |
| rs_novpin_x0.5 | 0.5 | solo, VPIN condition removed | +596 | 3167 | 33 | 1.02 | −2815 | +718 (1.09) | −365 (0.95) | −381 (0.95) | −1363 (0.81) | +1986 (1.38) | 2 | +2760 / −1558 | −1573 / 2760 / 0.95 / 1 / −1811 |
| rs_pair_x0.5 | 0.5 | solo + mirrored rs short | +1256 | 2861 | 33 | 1.04 | −2025 | +1491 (1.24) | −40 (0.99) | −146 (0.98) | −713 (0.88) | +664 (1.13) | 2 | +2454 / −898 | +1351 / 1535 / 1.08 / 4 / −377 |

## Long vs short subsets and per-window breakdown

**Short subset.** In every cell except `rs_pair_x0.5` the short subset is the v18 book to within noise: 407 trades in all
nine cells, 5y P&L within −2..+15 of +2,154 (the replay runs tickers independently, so the 3-position cap never
binds; the only interaction is a long occupying a ticker when a short window would have fired, which cost/gained a
handful of dollars). In `rs_pair_x0.5` the rs-short window (priority 31) pre-empts the v18 windows: the v18 windows
fired only 143 times (+1,660, PF 2.18 — the survivors are the good ones) and the rs short itself lost −1,755 over
1,183 trades (PF 0.87, negative every year: −285 / −266 / −609 / −466 / −129).

**Long subset per year (P&L / n / PF):**

| cell | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|
| rl_none (no RS) | +942 / 185 / 1.38 | +29 / 298 / 1.01 | +95 / 319 / 1.03 | −652 / 255 / 0.77 | +360 / 192 / 1.18 |
| rs_win_x0.3 | +908 / 152 / 1.44 | +260 / 264 / 1.09 | +150 / 273 / 1.05 | −356 / 208 / 0.83 | +341 / 180 / 1.17 |
| rs_win_x0.5 | +848 / 140 / 1.44 | +128 / 240 / 1.05 | +379 / 255 / 1.15 | −393 / 186 / 0.81 | +321 / 175 / 1.17 |
| rs_win_x0.8 | +514 / 114 / 1.31 | −64 / 203 / 0.97 | +393 / 202 / 1.18 | −166 / 148 / 0.89 | +102 / 156 / 1.06 |
| rs_win_x1.2 | +809 / 91 / 1.69 | +260 / 142 / 1.17 | +230 / 149 / 1.14 | −309 / 114 / 0.78 | −298 / 135 / 0.83 |
| rs_solo_x0.5 | +466 / 234 / 1.15 | +207 / 385 / 1.05 | +387 / 364 / 1.11 | −382 / 290 / 0.88 | +612 / 261 / 1.23 |
| rs_solo_x1.0 | +638 / 143 / 1.34 | +259 / 250 / 1.10 | +146 / 235 / 1.06 | −515 / 177 / 0.76 | −26 / 188 / 0.99 |
| rs_solo_x1.5 | +754 / 95 / 1.57 | −168 / 142 / 0.90 | −179 / 156 / 0.90 | −512 / 108 / 0.67 | −193 / 118 / 0.87 |
| rs_solo35_x0.5 | +528 / 195 / 1.19 | +171 / 323 / 1.05 | +390 / 324 / 1.12 | −325 / 253 / 0.88 | +585 / 226 / 1.26 |
| rs_novpin_x0.5 | −864 / 478 / 0.88 | −185 / 631 / 0.97 | −286 / 658 / 0.96 | −1811 / 545 / 0.71 | +1572 / 448 / 1.35 |
| rs_pair_x0.5 (long leg) | +470 / 234 / 1.15 | +230 / 385 / 1.06 | +412 / 365 / 1.12 | −377 / 290 / 0.88 | +616 / 261 / 1.23 |

**Per window, part 1 (5y P&L / n / PF):**

| window | X=0.3 | X=0.5 | X=0.8 | X=1.2 |
|---|---|---|---|---|
| 5m thrust +filters | +193 / 138 / 1.13 | +259 / 128 / 1.19 | +73 / 112 / 1.06 | +3 / 95 / 1.00 |
| candle reversal +filters | +426 / 82 / 1.50 | +230 / 64 / 1.34 | +147 / 48 / 1.25 | +295 / 30 / 1.94 |
| strong core +filters | +684 / 857 / 1.07 | +794 / 804 / 1.09 | +559 / 663 / 1.07 | +394 / 506 / 1.07 |
| 5m thrust short (v18) | +1451 / 360 / 1.36 | +1447 / 360 / 1.36 | +1447 / 360 / 1.36 | +1446 / 360 / 1.36 |
| strong core short (v18) | +705 / 47 / 2.48 | +705 / 47 / 2.48 | +705 / 47 / 2.48 | +706 / 47 / 2.48 |

The candle-reversal window is the only long window with a PF above 1.2 at any X, and it fires 30–82 times in five
years; strong core supplies ~80 % of the long volume at PF ≈ 1.07 whatever X is.

**Best cell (`rs_win_x0.3`) long-trade exit mix:** MaxHoldTimeout 765 (+3,304), BreakevenStop 197 (−969),
SessionClose 78 (+1,363), ScoreExit 27 (−1,456), HardStop 10 (−940). Average hold 54.5 min, median 41 min, win 34 %.
The same shape holds in every cell: timeouts and session closes carry all the profit; the 3–4 % of trades that hit
a score exit or hard stop give back about two thirds of it.

## Sanity checks

- All 10 sweeps: 260 / 260 / 262 / 261 / 181 days for 2022–2026, `0 skipped`, no non-holiday `skipped:` lines.
- `entry_reason` shows the study's window names: `window:5m thrust +filters rs0.3` etc. in part 1, `window:rs long`
  in part 2, `window:rs short` in the pair cell; the v18 windows keep their names.
- Single-day smoke (2025-04-04, `rs_solo_x0.5`, `--dump-ticks`): `pdl_5m.rel_close_pct` present on all 1,559 bar rows,
  range −4.4 .. +3.6 %, mean +0.43 %; `cross_1m.index_ret_prior_close` also present. The window did not fire that day
  (SPY −6 %, so the SPY-flat condition blocks), and the v18 NVDA short still fired — plumbing intact.
- Machine was loaded (load average ~18 on 12 cores from concurrent studies); each sweep took ~12 min instead of 3–8.
  No effect on results, only wall time.
- Sweeps were run one at a time, in the order listed in the commands block.

## Interpretation

- The relative-strength condition is a real but small improvement on the filtered long book: at X = 0.3 it drops
  172 of 1,249 long trades and lifts the long subset from +775 (PF 1.06, worst year −652) to +1,303 (PF 1.11, worst
  −356). The condition removes the worst 2025 losers but does not change the book's character.
- Tighter is worse, monotonically. Both in part 1 (long PF 1.11 → 1.12 → 1.08 → 1.09 for X 0.3/0.5/0.8/1.2) and
  part 2 (1.08 → 1.05 → 0.96 for 0.5/1.0/1.5). A name already +1.5 % vs SPY intraday is more likely to mean-revert
  than to keep spreading; the edge, such as it is, lives in the 0.3–0.5 % band.
- The "spread" framing does not create a new edge: the standalone rs window (part 2) with a loose composite ≥ 0.2 is
  just a higher-volume, lower-quality version of the filtered windows (PF 1.08 on 1,534 trades vs 1.12 on 996).
  Raising composite_min to 0.35 nudges it to PF 1.10 while keeping 1,321 trades — still below the bar.
- VPIN ≥ 0.217 is doing most of the work. Removing it doubles the trade count and turns the long subset to −1,573
  (PF 0.95), including −1,811 in 2025. Whatever the long edge is, it is confined to high-VPIN bars.
- The mirrored short (rel ≤ −0.5, composite ≤ −0.2) is unambiguously bad: −1,755 over 1,183 trades, negative all five
  years, and because it fires earlier in the day it starves the v18 short windows (407 → 143 trades). "Underperforming
  SPY while SPY is flat" is not a short signal at this composite threshold; the v18 short windows' stricter
  timescale conditions are what make the short book work.
- 2025 is the consistent hole for every long variant (−166 to −652); 2022 and 2026 carry the long P&L. Long PF
  never exceeds 1.12 pooled, so the long leg adds volume (≈ 2.5–4× the v18 trade count) but dilutes the book's
  PF from 1.48 to ≈ 1.2 and deepens max drawdown from −526 to −800.
- The combined book at X = 0.3 has 5 of 5 positive years, but 2023–2025 are only +55..+93 each — i.e. flat years
  where the long leg roughly offsets the weak short years, not years the long leg wins.

## Verdict

Best cell: **`rs_win_x0.3`** (combined +3,458 / 1,484 trades / PF 1.21 / maxDD −800, 5 of 5 years positive; long
subset +1,303 / 1,077 / PF 1.11, 4 of 5 years positive, worst year −356). `rs_win_x0.5` is a statistical tie
(+3,435, PF 1.22, long PF 1.12, worst −393).

**Fails the bar.** It clears the volume test (1,484 > 407), the positive-years test (5/5) and the long-subset-positive
test, but the combined pooled PF is 1.21 (bar ≥ 1.3), the long subset's PF is 1.11 (bar ≥ 1.2), and the long
subset's worst year is −356 (bar ≥ −300). No cell in the grid reaches PF 1.3 combined or PF 1.2 long. The
relative-strength condition is worth keeping in mind as a filter (it is the best single long filter tried so far on
IEX) but not worth promoting; the long book needs a different entry idea, not a tighter RS threshold.

## Commands

```
# smoke (single day, tick dump)
set -a; source .env; set +a
./target/release/backtest --date 2025-04-04 --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 \
  --output-trades-csv --bars-dir data/bars_iex --cross-index SPY --sizing-fraction 0.36 --max-position-pct 0.36 \
  --patch-json research/volume/rs_solo_x0.5.json --dump-ticks <scratch>/rs_smoke.csv

# sweeps (run one at a time, in this order)
B="--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY"
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_win_x0.3    $B --patch-json research/volume/rs_win_x0.3.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_win_x0.5    $B --patch-json research/volume/rs_win_x0.5.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_win_x0.8    $B --patch-json research/volume/rs_win_x0.8.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_win_x1.2    $B --patch-json research/volume/rs_win_x1.2.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_solo_x0.5   $B --patch-json research/volume/rs_solo_x0.5.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_solo_x1.0   $B --patch-json research/volume/rs_solo_x1.0.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_solo_x1.5   $B --patch-json research/volume/rs_solo_x1.5.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_solo35_x0.5 $B --patch-json research/volume/rs_solo35_x0.5.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_novpin_x0.5 $B --patch-json research/volume/rs_novpin_x0.5.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rs_pair_x0.5   $B --patch-json research/volume/rs_pair_x0.5.json

# summaries
python3 research/entries/summarize.py iex_v18 rs_win_x0.3 rs_win_x0.5 rs_win_x0.8 rs_win_x1.2 \
  rs_solo_x0.5 rs_solo_x1.0 rs_solo_x1.5 rs_solo35_x0.5 rs_novpin_x0.5 rs_pair_x0.5 --by-window
```

Unused pre-generated patches (not swept): `rs_solo35_x{1.0,1.5}.json`, `rs_novpin_x{1.0,1.5}.json`, `rs_pair_x{1.0,1.5}.json`.
