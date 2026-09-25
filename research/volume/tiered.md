# Study B — tiered sizing below the VPIN floor (IEX feed, honest costs)

**Question.** v18 only enters when `vpin_1m.raw_vpin ≥ 0.217`. Can we take the entries that fire just
below that floor at reduced size, adding volume without hurting PF?

**Design.** windows patch `iex_b0.4_v<L>.json` (same v18 windows, SPY ±0.2 %, VPIN floor lowered to L)
+ sizing patch `tier_l<L>_m<M>.json`: `sizing_fixed` disabled, `indicator_tiered` on `vpin_1m.raw_vpin`,
base 0.36, tiers `[{0.217 → 1.0}, {L → M}]`, fallback 0 (entry skipped). Grid L ∈ {0.12, 0.15, 0.18} ×
M ∈ {0.33, 0.5, 0.67}; follow-ups add a top tier `{0.26 → 1.25}` with the position cap at 0.45 (unclamped)
and 0.36 (clamped). 2022-01-01..2026-09-10 on `data/bars_iex`, `--sizing-fraction 0.36 --cross-index SPY`,
3 bps slippage + $0.005 half-spread. Baseline `iex_v18`: **+2,154 / 407 trades / PF 1.48 / 3 positive years**.
Bar: pooled PF ≥ 1.3, ≥ 4 of 5 years positive, > 407 trades.

## 1. Grid

Per-year cells are `P&L (PF)`. `+yrs` = positive years of 5. Δ columns are vs `iex_v18`.
`e_iex_b0.4_v*` rows are the full-size references (same windows, M = 1.0, `sizing_fixed`), already on disk.

| tag | L | M | 5y P&L | trades | win % | PF | max DD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | Δ trades | Δ P&L |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| iex_v18 (baseline) | 0.217 | — | +2154 | 407 | 39 | 1.48 | -526 | +1565 (2.98) | -181 (0.83) | -93 (0.92) | +451 (1.57) | +413 (1.60) | 3 | 0 | 0 |
| tier_l0.12_m0.33 | 0.12 | 0.33 | +1872 | 592 | 36 | 1.42 | -487 | +1264 (2.43) | -183 (0.83) | -74 (0.93) | +437 (1.53) | +427 (1.71) | 3 | +185 | -282 |
| tier_l0.12_m0.5 | 0.12 | 0.5 | +1968 | 592 | 36 | 1.39 | -526 | +1406 (2.41) | -224 (0.81) | -70 (0.94) | +450 (1.49) | +405 (1.56) | 3 | +185 | -186 |
| tier_l0.12_m0.67 | 0.12 | 0.67 | +2032 | 593 | 36 | 1.36 | -526 | +1488 (2.28) | -242 (0.80) | -51 (0.96) | +452 (1.44) | +384 (1.46) | 3 | +186 | -122 |
| tier_l0.15_m0.33 | 0.15 | 0.33 | +1822 | 539 | 36 | 1.42 | -536 | +1268 (2.54) | -193 (0.82) | -65 (0.94) | +377 (1.47) | +436 (1.77) | 3 | +132 | -332 |
| tier_l0.15_m0.5 | 0.15 | 0.5 | +1903 | 539 | 36 | 1.40 | -532 | +1393 (2.52) | -208 (0.82) | -58 (0.95) | +358 (1.41) | +419 (1.63) | 3 | +132 | -251 |
| tier_l0.15_m0.67 | 0.15 | 0.67 | +2009 | 539 | 36 | 1.39 | -505 | +1522 (2.52) | -207 (0.83) | -37 (0.97) | +328 (1.34) | +403 (1.54) | 3 | +132 | -145 |
| tier_l0.18_m0.33 | 0.18 | 0.33 | +1854 | 487 | 38 | 1.42 | -544 | +1381 (2.75) | -233 (0.78) | -90 (0.92) | +382 (1.46) | +414 (1.67) | 3 | +80 | -300 |
| tier_l0.18_m0.5 | 0.18 | 0.5 | +1926 | 487 | 38 | 1.41 | -538 | +1491 (2.76) | -241 (0.78) | -78 (0.93) | +370 (1.42) | +384 (1.56) | 3 | +80 | -228 |
| tier_l0.18_m0.67 | 0.18 | 0.67 | +1998 | 487 | 38 | 1.41 | -537 | +1607 (2.78) | -235 (0.80) | -84 (0.93) | +355 (1.38) | +355 (1.47) | 3 | +80 | -156 |
| tier3_l0.12_cap45 | 0.12 | 0.67 (+1.25 ≥0.26, cap 0.45) | +2377 | 593 | 36 | 1.38 | -559 | +1645 (2.29) | -240 (0.83) | -8 (0.99) | +527 (1.46) | +454 (1.52) | 3 | +186 | +223 |
| tier3_l0.12_cap36 | 0.12 | 0.67 (+1.25 ≥0.26, cap 0.36) | +2032 | 593 | 36 | 1.36 | -526 | +1488 (2.28) | -242 (0.80) | -51 (0.96) | +452 (1.44) | +384 (1.46) | 3 | +186 | -122 |
| e_iex_b0.4_v0.12 | 0.12 | 1.0 | +2175 | 604 | 36 | 1.32 | -632 | +1704 (2.08) | -306 (0.78) | -24 (0.98) | +464 (1.38) | +336 (1.32) | 3 | +197 | +21 |
| e_iex_b0.4_v0.15 | 0.15 | 1.0 | +2186 | 548 | 36 | 1.36 | -528 | +1775 (2.36) | -232 (0.82) | -4 (1.00) | +280 (1.24) | +366 (1.39) | 3 | +141 | +32 |
| e_iex_b0.4_v0.18 | 0.18 | 1.0 | +2106 | 491 | 38 | 1.38 | -537 | +1797 (2.66) | -252 (0.80) | -58 (0.96) | +329 (1.31) | +289 (1.33) | 3 | +84 | -48 |

## 2. Tier-2 breakdown

Tier-2 = trades whose (ticker, entry_time) is not in `iex_v18`'s trade set; tier-1 = the rest. "Displaced" =
v18 trades absent from the cell. Every displaced trade, in every cell, is explained by the same mechanism:
a tier-2 entry on the same ticker was already open when the v18 entry would have fired
(`research/volume/displacement.py`: 100 % "same-ticker open (t2)", 0 concurrency-cap cases).

| tag | tier-2 n | share | tier-2 P&L | tier-2 win % | tier-2 PF | tier-2 mean sh | tier-1 n | tier-1 P&L | tier-1 Δ vs v18 | v18 trades displaced (their v18 P&L) |
|---|---|---|---|---|---|---|---|---|---|---|
| tier_l0.12_m0.33 | 279 | 47% | +168 | 32 | 1.18 | 5.2 | 313 | +1704 | -450 | 94 (+452) |
| tier_l0.12_m0.5 | 279 | 47% | +263 | 32 | 1.18 | 8.1 | 313 | +1705 | -449 | 94 (+452) |
| tier_l0.12_m0.67 | 281 | 47% | +356 | 32 | 1.18 | 10.9 | 312 | +1676 | -478 | 95 (+484) |
| tier_l0.15_m0.33 | 218 | 40% | +165 | 31 | 1.23 | 5.1 | 321 | +1657 | -497 | 86 (+496) |
| tier_l0.15_m0.5 | 218 | 40% | +246 | 31 | 1.21 | 8.0 | 321 | +1657 | -497 | 86 (+496) |
| tier_l0.15_m0.67 | 219 | 41% | +383 | 31 | 1.24 | 10.8 | 320 | +1627 | -527 | 87 (+528) |
| tier_l0.18_m0.33 | 144 | 30% | +123 | 35 | 1.26 | 4.8 | 343 | +1731 | -423 | 64 (+423) |
| tier_l0.18_m0.5 | 144 | 30% | +195 | 35 | 1.26 | 7.6 | 343 | +1731 | -423 | 64 (+423) |
| tier_l0.18_m0.67 | 145 | 30% | +300 | 35 | 1.29 | 10.2 | 342 | +1699 | -455 | 65 (+455) |
| tier3_l0.12_cap45 | 281 | 47% | +358 | 32 | 1.18 | 10.9 | 312 | +2019 | -135 | 95 (+484) |
| tier3_l0.12_cap36 | 281 | 47% | +356 | 32 | 1.18 | 10.9 | 312 | +1676 | -478 | 95 (+484) |
| e_iex_b0.4_v0.12 | 295 | 49% | +575 | 32 | 1.17 | 15.8 | 309 | +1600 | -554 | 98 (+554) |
| e_iex_b0.4_v0.15 | 231 | 42% | +631 | 32 | 1.25 | 15.6 | 317 | +1554 | -600 | 90 (+599) |
| e_iex_b0.4_v0.18 | 151 | 31% | +486 | 36 | 1.29 | 14.9 | 340 | +1620 | -534 | 67 (+532) |

Splitting tier-2 further (`research/volume/frontrun.py`) into **front-runners** (a v18 entry on the same
ticker would have fired during the tier-2 hold — VPIN crossed L and then 0.217 a median 3–4 min later) and
**genuinely new** (VPIN never reached 0.217 during the hold):

| cell | front-runners P&L / n / PF | v18 trades they replaced (full size) | genuinely new P&L / n / PF |
|---|---|---|---|
| tier_l0.12_m0.33 | +397 / 93 / 3.18 | +452 / 94 / 1.46 | −229 / 186 / 0.69 |
| tier_l0.12_m0.67 | +906 / 94 / 3.32 | +484 / 95 / 1.49 | −550 / 187 / 0.66 |
| tier_l0.15_m0.5 | +613 / 85 / 3.10 | +496 / 86 / 1.53 | −366 / 133 / 0.58 |
| tier_l0.15_m0.67 | +842 / 86 / 3.15 | +528 / 87 / 1.57 | −459 / 133 / 0.61 |
| tier_l0.18_m0.33 | +231 / 64 / 2.61 | +423 / 64 / 1.66 | −108 / 80 / 0.67 |
| tier_l0.18_m0.67 | +549 / 65 / 2.76 | +455 / 65 / 1.72 | −249 / 80 / 0.65 |
| e_iex_b0.4_v0.12 (M 1.0) | +1,509 / 97 / 3.47 | +554 / 98 / 1.55 | −934 / 198 / 0.65 |
| e_iex_b0.4_v0.15 (M 1.0) | +1,405 / 89 / 3.29 | +599 / 90 / 1.63 | −774 / 142 / 0.60 |
| e_iex_b0.4_v0.18 (M 1.0) | +937 / 67 / 2.90 | +532 / 67 / 1.81 | −450 / 84 / 0.62 |

Per share (L 0.12, M 0.67): front-runners +0.85 $/sh, the v18 trades they replace +0.29 $/sh, genuinely-new
−0.28 $/sh; mean holds 59 / 57 / 49 min. Genuinely-new trades are 92 % `5m thrust short +cond` and exit
mostly on `MaxHoldTimeout` (67 %). The genuinely-new subset is negative in 4 of 5 years in every cell
(only 2022 is ≥ break-even).

## 3. Sanity checks

- All 9 grid cells and both follow-ups: 260 / 260 / 262 / 261 / 181 days for 2022–2026, `0 skipped`,
  identical to `iex_v18` (which also reports 181 for 2026, not 183). No non-holiday `skipped:` lines.
- `entry_reason` in every cell is only `window:5m thrust short +cond` / `window:strong core short +cond`
  (the `_x` windows from the variant file); the stock windows do not fire.
- Tier-2 mean size scales with M as expected (≈ 5 / 8 / 11 sh for M 0.33 / 0.5 / 0.67 vs ≈ 16 sh tier-1).
- Tier-1 subsets are P&L-identical to the corresponding v18 trades (Δ tier-1 = −(displaced P&L) to the
  dollar), so the sizing action does not touch trades at or above 0.217. Cells at the same L share the same
  trade set (M only changes size), bar 1–2 trades where the different fill changed a stop.
- Sweeps took 12–15 min each (load ≈ 20 from concurrent studies), not the quoted 3–8.

## 4. Interpretation

- **No cell adds volume without giving P&L back.** All 9 grid cells keep PF ≥ 1.36 and add 80–186 trades,
  but every one loses 122–332 vs v18, and none moves the year count: 2023 and 2024 are negative in every
  cell, exactly as in v18. Reduced size only scales the loss (M 0.33 → 0.67 recovers about half of the gap
  at each L) because the tier-2 trades are net positive; the problem is not their P&L, it is what they displace.
- **The tier-2 bucket is two different populations.** ~30 % of tier-2 entries are *front-runners*: VPIN
  crosses L and then 0.217 a median 3–4 min later, so the tier-2 entry takes the ticker slot and the v18
  entry never fires (100 % of displaced v18 trades are "same ticker already open"; the 3-position
  concurrency cap displaces nothing). These are the good trades in the study (PF 2.6–3.5, +0.85 $/sh vs
  +0.29 $/sh for the v18 trades they replace) — but at M < 1 they are taken at 1/3–2/3 of the size v18 would
  have used 4 min later, so the cell books less on them than v18 did. The remaining ~70 % are *genuinely new*
  (VPIN never reaches 0.217 during the hold): PF 0.58–0.69, −0.28 $/sh, 92 % `5m thrust short`, mostly
  timing out, negative in 4 of 5 years in every cell. That is the population the VPIN floor exists to exclude.
- **The full-size references say the same thing.** `e_iex_b0.4_v0.12/0.15/0.18` are +21 / +32 / −48 vs v18
  with PF 1.32–1.38: the front-runners at full size earn +1,509 / +1,405 / +937 but the genuinely-new trades
  give back −934 / −774 / −450. Tiering moves the cells between those two references and v18; it cannot
  separate the two populations because at entry time the engine cannot know which one it is in.
- **Lower L is not more selective.** Genuinely-new PF is flat across L (0.65–0.69 at 0.12, 0.58–0.61 at
  0.15, 0.62–0.67 at 0.18); the 0.15 cells are actually the worst per trade. Going from 0.18 to 0.12 roughly
  doubles both populations without changing their quality.
- **The three-tier cap-0.45 follow-up "wins" for the wrong reason.** +2,377 (Δ +223, PF 1.38) comes entirely
  from the top tier: 220 of the 305 tier-1 trades have VPIN ≥ 0.26 and are sized ×1.25 (+1,614 vs +1,269 for
  the same trades in v18); its tier-2 subset is identical to `tier_l0.12_m0.67`. That is leverage on v18's
  existing trades, not a tiering result, and it still leaves 2023 / 2024 negative (2024 −8) with a deeper
  drawdown (−559). With the cap at 0.36 the top tier is fully clamped and the cell is byte-for-byte
  `tier_l0.12_m0.67`.
- **What the front-runner subset does suggest** (not tested here): the profitable part of "below the floor"
  is a *rising* VPIN that reaches 0.217 within minutes, i.e. a slope condition, not a lower level. A
  VPIN-momentum entry condition (or letting a sub-floor entry upgrade to full size when VPIN crosses 0.217
  while the position is open) would target it directly; a static lower floor does not.

## 5. Verdict

**Best cell: `tier_l0.12_m0.67`** (highest P&L of the 9, +2,032 / 593 trades / PF 1.36 / max DD −526;
`tier_l0.18_m0.67` is the closest to v18 in shape: +1,998 / 487 / PF 1.41). **It fails the bar**: PF 1.36 ≥ 1.3
and 593 > 407 trades pass, but only 3 of 5 years are positive (2023 −242, 2024 −51), and it is −122 vs v18.
No cell in the grid reaches 4 positive years; the only cell above v18 (`tier3_l0.12_cap45`, +2,377) gets there by
upsizing v18's own high-VPIN trades and also stays at 3 of 5 years. Verdict: **do not promote tiered sizing
below the VPIN floor.** Keep the 0.217 floor at full size; the sub-floor population is only worth having when
VPIN is about to cross the floor, which a level-based tier cannot detect. The `indicator_tiered` action itself
works as specified (sizes scale exactly with M, tier-1 trades untouched); the ×1.25 top tier is a separate
sizing question that should be tested on v18 alone if it is pursued.

## Commands

```bash
# common prefix
P="BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh"; A="--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY"
# grid (patch files: research/volume/tier_l<L>_m<M>.json — smoke_tiered.json with tiers [{0.217,1.0},{L,M}])
for L in 0.12 0.15 0.18; do for M in 0.33 0.5 0.67; do
  BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh tier_l${L}_m${M} $A \
    --patch-json research/entries/variants/iex_b0.4_v${L}.json --patch-json research/volume/tier_l${L}_m${M}.json
done; done
# follow-ups (research/volume/tier3_l0.12_m0.67.json: tiers [{0.26,1.25},{0.217,1.0},{0.12,0.67}])
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh tier3_l0.12_cap45 --sizing-fraction 0.36 --max-position-pct 0.45 --cross-index SPY \
  --patch-json research/entries/variants/iex_b0.4_v0.12.json --patch-json research/volume/tier3_l0.12_m0.67.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh tier3_l0.12_cap36 --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY \
  --patch-json research/entries/variants/iex_b0.4_v0.12.json --patch-json research/volume/tier3_l0.12_m0.67.json
# analysis
python3 research/entries/summarize.py iex_v18 tier_l0.12_m0.67 --by-window
python3 research/volume/tiered_tables.py tier_l0.12_m0.67:0.12:0.67 ...   # grid + tier-2 tables
python3 research/volume/tier2_breakdown.py tier_l0.12_m0.67                # per-year tier-1 / tier-2
python3 research/volume/displacement.py tier_l0.12_m0.67                   # why v18 trades went missing
python3 research/volume/frontrun.py tier_l0.12_m0.67                       # front-runners vs genuinely new
```
