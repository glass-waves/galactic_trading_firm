# v15c short-only: did losers exit too late or too early, and which exit knob fixes it?

data: `data/v15c_{2022..2026}_trades.csv` (1,286 trades), `data/v15c_<year>_ticks.csv` (composite path), `data/bars/<TICKER>.csv` (1-min RTH bars).
all numbers at 36 % sizing. scripts: `exitsim.py` (loader + simulator), `verify.py`, `analysis.py`, `sweeps.py`, `sweeps2.py`, `robust.py`; full tables in `tables_paths.md`, `tables_sweeps.md`, `tables_sweeps2.md`, `tables_robust.md`; per-trade paths in `paths.csv`.

note: the brief quotes 2022 = +3,564; the trade rows in `v15c_2022_trades.csv` sum to **+3,654** (all other years match the brief). I use the file.

---

## headline findings (the ones that would change a knob)

### 1. losers exit too LATE, and the fix is the existing `loss_reduction_ms` knob: losing-side limit 75 min -> 30 min

- losers show their hand early: **74 % of eventual losers are already underwater at 15 min, 82 % at 30, 90 % at 45**, and the loss keeps growing (mean -0.41 % @15, -0.66 % @30, -0.96 % @45, -1.16 % final). only 24 % of losers ever had >= 0.5 % open profit; 7 % had >= 1.0 %. so "we gave back a win" is the minority case; "we sat in a trade that never worked" is the majority.
- eventual winners are the mirror image: **only 11 % of winners are negative at 30 min**; mean +0.77 % @30, +0.93 % @45.
- the engine's MaxHoldTimeout is evaluated every bar, so `max_hold_ms - loss_reduction_ms` is literally "exit the first bar after N minutes on which the position is losing". the "new time-stop that needs code" is therefore **already a config knob** (verified: simulated `timestop 30` == `losing limit 30`, identical P&L every year).
- single knob change `loss_reduction_ms 900000 -> 3600000` (losing limit 30, winning limit unchanged at 120):

| scenario | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | yrs>=0 | win% | PF 22/23/24/25/26 | maxDD$ 22/23/24/25/26 |
|---|---|---|---|---|---|---|---|---|---|---|
| current (L75 / W120) | +3654 | +479 | +395 | +697 | +666 | **+5890** | 5 | 50.6% | 1.57 / 1.17 / 1.13 / 1.20 / 1.26 | 605 / 415 / 870 / 676 / 539 |
| loss_reduction 60 min (L30 / W120) | +3860 | +470 | +894 | +811 | +1071 | **+7105** | 5 | 43.6% | 1.86 / 1.22 / 1.42 / 1.29 / 1.64 | 491 / 243 / 675 / 709 / 329 |
| + profit_extension 0 (L30 / W90) | +3922 | +467 | +900 | +1052 | +1166 | **+7507** | 5 | 44.2% | 1.87 / 1.22 / 1.42 / 1.38 / 1.70 | 491 / 207 / 675 / 580 / 329 |

- profit factor improves in every year, max drawdown falls in 4 of 5 years (2025 slightly up under W120, down under W90). 2023 is flat (-12 / -9 $), never negative.
- counter-evidence checked: (a) the surface is bumpy — L=45 (+6,211) is a local dip between L=40 (+6,661) and L=50 (+6,456) — but **every L in 20..60 beats the current 75 in every column of the grid**, so the direction is robust even if the exact optimum is not; (b) leave-one-year-out picks (L=30, W=90) in all five folds and the held-out year improves in 4/5 (+268, -12, +505, +356, +500); (c) by ticker: L30/W90 beats current for MSFT in 5/5 years, NVDA 4/5, AAPL 3/5, AMZN 3/5 — AMZN/AAPL 2022-23 give back $50-280; (d) by window: both windows improve on the five-year total (thrust +3,286 -> +3,931; core +2,604 -> +3,576), thrust is slightly worse in 2022/2023; (e) win rate drops from 50.6 % to 44 % — the rule cuts 442 original winners early (they lose $4,702 of their $21,346) and 380 original losers early (they save $6,703 of their $13,738); net +$1,615 on the 1,116 affected trades.

### 2. the winning side is NOT the problem — don't extend holds

- winners exited by MaxHoldTimeout capture 77 % of their path MFE, SessionClose winners 79 %; 28 % / 52 % of them exit within 0.2 % of the best fill available. forgone P&L to 11:55 on winners is +$303 (MaxHold) / $0 (SessionClose) over five years — noise.
- `profit_extension_ms` 0 / 30 / 60 / 90 => +6,365 / +5,890 / +6,165 / +6,165: non-monotone, all within noise of each other on the winner side; 60 and 90 are identical because the 11:55 close binds first. no evidence for more extension.
- `max_hold_ms` alone: 45 / 60 / 75 / 90 / 105 / 120 / 150 => +6,886 / +6,719 / +6,172 / +5,890 / +5,791 / +5,381 / +5,077 — monotone: every extension of hold loses money, every shortening gains. this is driven entirely by the losing-side limit (45-15 = 30 min), not by cutting winners.

### 3. `stop_loss_pct` 2.5 % is too wide *given the current 75-min losing limit*; it stops mattering once the losing limit is 30

- HardStop trades (34 in five years, -2.64 % mean) already showed -1.49 % at 15 min and -2.14 % at 30 min. 74 % of them bounce > 0.3 % after the stop (regret +1.20 %) but holding to 11:55 would have been worse still (forgone -0.88 %).
- alone: stop 1.0 / 1.5 / 2.0 / 2.5 / 3.5 % => +6,897 / +6,806 / +6,198 / +5,890 / +5,555, all 5/5 years, drawdown falls in every year at 1.0-1.5 %. leave-one-year-out picks 1.0 % in 4 folds, 1.5 % in one; held-out delta positive in all five years (+51, +153, +243, +130, +178).
- but on top of L30/W90: stop 1.0 / 1.5 / 2.0 / 2.5 => +7,124 / +7,505 / +7,612 / +7,507 — flat. the time stop removes most trades before they reach -2.5 %. tighter stop still lowers max DD (1.5 %: 436/234/588/440/305 vs 491/207/675/580/329) at no P&L cost. **secondary recommendation: 1.5 %, for drawdown, not P&L.**

### 4. `exit_threshold` (ScoreExit) is a late loss-cutter; leave it at 0.30 (or 0.20), do not raise it

- 261 ScoreExits: 199 losers, 32 breakeven, 30 winners; mean -0.89 %; median loser fires at 51 min with only 39 % of the final loss present at 30 min (table C). it is doing the job the time stop would do, later.
- alone: T = 0.10 / 0.20 / 0.30 / 0.45 / never => +6,382 / +6,031 / +5,890 / +5,478 / +5,349. lower T helps 2024-2026 (+400 / +60 / +210) but hurts 2022-2023 (-144 / -39) — mixed per year, not a clean win. raising or removing it costs money in 4/5 years.
- on top of L30/W90: T = 0.1 / 0.2 / 0.3 / never => +7,086 / +7,459 / +7,507 / +7,578 — irrelevant once the time stop is in. **no change recommended.**

### 5. `breakeven_stop.trigger_pct` does nothing today — this is a code finding, not a knob

- `crates/engine/src/tick_loop.rs` handles `ActionSignal::ModifyStop` with a comment "for now we just note it happened"; the breakeven monitor never changes any exit. the brief's "breakeven_stop monitor: once profit >= 1.5 %, stop moves to entry" is not what the engine does. the reproduction confirms it: the simulator reproduces all 1,286 exits with NO breakeven logic.
- if it were implemented (simulated: once close-based MFE >= trigger, exit when close >= entry): trigger 0.5 / 1.0 / 1.5 / never => +6,529 / +6,538 / +6,386 / +5,890 (all 5/5). on top of L30/W90, 0.5 % gives +7,809 with the lowest drawdowns of any scenario (491/204/549/321/274). worth implementing, but it needs code, and the per-year pattern (2022 -359 vs +$500-700 elsewhere) is less uniform than the time stop.

---

## sanity / reproduction (analysis 4)

- exit convention: next-bar open, `exit = open*(1-0.0003) - 0.005`, `entry = open*(1+0.0003) + 0.005`; `hold = fill-bar ts - fill-bar ts`. verified against raw bars for every trade.
- simulator with current params vs real trades: **1,285 / 1,286 same exit reason, 1,284 / 1,286 same exit timestamp, 1,286 / 1,286 exit price within 0.2 %** (worst 0.065 %). per-year P&L real vs sim: 2022 3654/3654, 2023 479/479, 2024 395/395, 2025 699/697, 2026 665/666.
- the two mismatches: AMZN 2026-05-21 (sim ScoreExit one bar earlier — composite printed as 0.3000 in the tick file, engine had 0.29996; $1.2 difference) and NVDA 2025-07-22, the single TrailingStop in five years (sim: MaxHold two bars later; $2.3). the ATR trailing stop is not simulated (7 x ATR fires once in 1,286 trades).
- path cross-check: 106,713 in-trade bars compared to the engine's `unrealized_pct` / `hold_min` columns: 0 mismatches, worst difference 5e-5.
- no trades excluded.

---

## analysis 1: path reconstruction

per year x outcome bucket, all exit reasons (mean % of entry price, + = profit; `@N` = fill you would get by signalling at N minutes; MFE/MAE close-based, before the actual exit signal):

| year | bucket | n | share | actual | MFE_pre | MAE_pre | @15 | @30 | @45 | @60 | @90 | @11:55 | MFE>=0.5% | MFE>=1.0% | neg@15 | neg@30 | neg@45 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2022 | loss | 128 | 31% | -1.39 | +0.38 | -1.59 | -0.49 | -0.72 | -1.12 | -1.31 | -1.53 | -1.51 | 27% | 9% | 72% | 78% | 90% |
| 2022 | breakeven | 97 | 23% | -0.04 | +0.74 | -0.49 | +0.14 | +0.18 | +0.19 | +0.15 | +0.09 | -0.09 | 63% | 25% | 38% | 36% | 36% |
| 2022 | win | 189 | 46% | +1.53 | +1.98 | -0.25 | +0.60 | +0.94 | +1.10 | +1.24 | +1.48 | +1.53 | 97% | 85% | 17% | 10% | 8% |
| 2023 | loss | 76 | 36% | -1.03 | +0.24 | -1.22 | -0.43 | -0.71 | -0.88 | -0.96 | -1.14 | -1.30 | 20% | 4% | 80% | 88% | 95% |
| 2023 | breakeven | 48 | 23% | -0.03 | +0.63 | -0.46 | +0.12 | +0.14 | +0.08 | +0.13 | -0.02 | -0.08 | 58% | 12% | 48% | 40% | 40% |
| 2023 | win | 85 | 41% | +1.11 | +1.45 | -0.15 | +0.48 | +0.63 | +0.67 | +0.82 | +1.10 | +1.14 | 98% | 65% | 12% | 11% | 6% |
| 2024 | loss | 75 | 35% | -1.11 | +0.40 | -1.28 | -0.37 | -0.58 | -0.81 | -1.02 | -1.34 | -1.40 | 29% | 11% | 72% | 81% | 93% |
| 2024 | breakeven | 63 | 30% | -0.02 | +0.80 | -0.41 | +0.24 | +0.22 | +0.18 | +0.15 | -0.01 | -0.11 | 63% | 29% | 38% | 44% | 35% |
| 2024 | win | 74 | 35% | +1.32 | +1.75 | -0.15 | +0.58 | +0.77 | +0.97 | +1.14 | +1.28 | +1.37 | 96% | 78% | 12% | 9% | 3% |
| 2025 | loss | 82 | 34% | -1.17 | +0.32 | -1.41 | -0.42 | -0.77 | -1.21 | -1.21 | -1.37 | -1.30 | 27% | 7% | 73% | 87% | 90% |
| 2025 | breakeven | 68 | 28% | -0.03 | +0.67 | -0.50 | +0.08 | +0.06 | +0.10 | +0.11 | +0.15 | -0.04 | 51% | 26% | 49% | 54% | 49% |
| 2025 | win | 93 | 38% | +1.26 | +1.71 | -0.27 | +0.41 | +0.66 | +0.86 | +0.97 | +1.25 | +1.28 | 98% | 75% | 25% | 14% | 11% |
| 2026 | loss | 75 | 36% | -0.93 | +0.23 | -1.11 | -0.30 | -0.48 | -0.64 | -0.85 | -0.97 | -1.03 | 15% | 3% | 73% | 80% | 84% |
| 2026 | breakeven | 58 | 28% | -0.03 | +0.56 | -0.42 | +0.13 | +0.09 | +0.08 | +0.16 | +0.01 | -0.09 | 53% | 9% | 38% | 48% | 38% |
| 2026 | win | 74 | 36% | +1.22 | +1.55 | -0.19 | +0.43 | +0.65 | +0.87 | +0.96 | +1.20 | +1.22 | 99% | 76% | 18% | 14% | 8% |
| ALL | loss | 436 | 34% | -1.16 | +0.32 | -1.36 | -0.41 | -0.66 | -0.96 | -1.10 | -1.30 | -1.33 | 24% | 7% | 74% | 82% | 90% |
| ALL | breakeven | 334 | 26% | -0.03 | +0.69 | -0.46 | +0.14 | +0.14 | +0.14 | +0.14 | +0.05 | -0.08 | 58% | 21% | 42% | 44% | 39% |
| ALL | win | 515 | 40% | +1.34 | +1.75 | -0.21 | +0.52 | +0.77 | +0.93 | +1.07 | +1.31 | +1.35 | 97% | 77% | 17% | 11% | 8% |

the same pattern holds in every year: losers are negative from the first 15 minutes and get worse; breakevens sit at +0.14 % for an hour and then decay to -0.08 % by 11:55 (they are winners that were held too long, not losers cut too soon — 58 % had >= 0.5 % open profit at some point); winners are positive at 15 min and climb monotonically.

five-year by exit reason x bucket (full per-year version in `tables_paths.md` table A):

| reason | bucket | n | actual | median hold | MFE | t_MFE (med, min) | MAE | @15 | @30 | @45 | @60 | @90 | @11:55 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| HardStop | loss | 34 | -2.64 | 30 | +0.33 | 2 | -5.40 | -1.49 | -2.14 | -3.15 | -2.82 | -3.61 | -3.52 |
| ScoreExit | loss | 199 | -1.35 | 51 | +0.41 | 4 | -2.20 | -0.32 | -0.65 | -1.02 | -1.30 | -1.47 | -1.51 |
| ScoreExit | breakeven | 32 | +0.08 | 88 | +1.45 | 32 | -0.39 | +0.67 | +0.81 | +0.74 | +0.61 | +0.42 | +0.17 |
| ScoreExit | win | 30 | +1.14 | 104 | +2.54 | 51 | -0.19 | +1.18 | +1.76 | +1.91 | +2.01 | +1.65 | +1.14 |
| MaxHoldTimeout | loss | 180 | -0.73 | 76 | +0.31 | 7 | -1.38 | -0.33 | -0.43 | -0.54 | -0.63 | -0.77 | -0.81 |
| MaxHoldTimeout | breakeven | 227 | -0.06 | 76 | +0.71 | 35 | -0.73 | +0.06 | +0.08 | +0.09 | +0.11 | +0.01 | -0.15 |
| MaxHoldTimeout | win | 333 | +1.49 | 121 | +1.94 | 98 | -0.25 | +0.52 | +0.78 | +0.95 | +1.08 | +1.41 | +1.51 |
| SessionClose | loss | 23 | -0.63 | 50 | +0.15 | 7 | -0.85 | -0.24 | -0.39 | -0.45 | -0.56 | -0.63 | -0.63 |
| SessionClose | breakeven | 75 | +0.01 | 55 | +0.48 | 15 | -0.33 | +0.16 | +0.06 | +0.03 | +0.02 | +0.03 | +0.01 |
| SessionClose | win | 152 | +1.04 | 84 | +1.31 | 57 | -0.17 | +0.38 | +0.55 | +0.70 | +0.87 | +1.01 | +1.04 |

note the ScoreExit breakeven/winner rows: 62 trades whose MFE was +1.45 % / +2.54 % at 32-51 min, which the score exit then closed at 88-104 min for +0.08 % / +1.14 %. these are the only group where "exited too late on a winner" is visible, and they are 5 % of trades.

## analysis 2: too late vs too early

**losers + breakevens (770 trades): share of the final dollar loss already present at N minutes** (size-weighted; `$@N` = P&L if every trade in the group had exited at N):

| year | n | actual $ | $@15 | $@30 | $@45 | $@60 | $@11:55 | ratio@15 | @30 | @45 | gave back >=0.5% | >=1.0% |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2022 | 225 | -6239 | -1669 | -2568 | -4261 | -5304 | -7015 | 0.27 | 0.41 | 0.69 | 42% | 16% |
| 2023 | 124 | -2772 | -939 | -1640 | -2195 | -2313 | -3580 | 0.34 | 0.59 | 0.79 | 35% | 7% |
| 2024 | 138 | -2968 | -472 | -1026 | -1749 | -2371 | -3968 | 0.16 | 0.35 | 0.59 | 45% | 19% |
| 2025 | 150 | -3427 | -1019 | -2094 | -3267 | -3223 | -3851 | 0.30 | 0.61 | 0.95 | 38% | 16% |
| 2026 | 133 | -2479 | -506 | -1062 | -1495 | -1881 | -2880 | 0.21 | 0.43 | 0.60 | 32% | 5% |
| ALL | 770 | -17885 | -4605 | -8390 | -12967 | -15093 | -21294 | 0.26 | 0.47 | 0.72 | 39% | 13% |

verdict for losers: **too late**. exiting every loser/breakeven at 30 min would have cut their combined loss from -$17,885 to -$8,390; holding all of them to 11:55 would have made it -$21,294. by exit reason (five years): ScoreExit losers -$9,121 actual vs -$3,514 @30 (ratio 0.39); MaxHold losers/BE -$5,085 vs -$2,107 @30 (0.42); HardStop -$3,207 vs -$2,618 @30 (0.81 — the stop is the one exit that is not late, it is just wide).

**"gave back a win"**: 39 % of losers+breakevens had >= 0.5 % open profit before the exit signal, 13 % had >= 1.0 %. mostly breakevens (58 % / 21 %) not losers (24 % / 7 %). a working breakeven stop at 0.5 % addresses exactly this group (finding 5) — but needs code.

**regret / forgone by exit reason** (five years; regret = best fill available after the exit-signal bar up to 11:55 minus actual, floored at 0; forgone = 11:55 fill minus actual, signed):

| reason | bucket | n | actual | regret mean | regret median | regret>0.3% | forgone mean | forgone $ | capture = actual/MFE |
|---|---|---|---|---|---|---|---|---|---|
| HardStop | loss | 34 | -2.64 | +1.20 | +1.10 | 74% | -0.88 | -1105 | - |
| ScoreExit | loss | 199 | -1.35 | +0.61 | +0.39 | 59% | -0.16 | -1115 | - |
| ScoreExit | breakeven | 32 | +0.08 | +0.37 | +0.32 | 53% | +0.09 | +92 | 0.05 |
| ScoreExit | win | 30 | +1.14 | +0.38 | +0.19 | 40% | -0.00 | -28 | 0.45 |
| MaxHoldTimeout | loss | 180 | -0.73 | +0.38 | +0.29 | 48% | -0.08 | -536 | - |
| MaxHoldTimeout | breakeven | 227 | -0.06 | +0.31 | +0.18 | 38% | -0.09 | -745 | - |
| MaxHoldTimeout | win | 333 | +1.49 | +0.23 | +0.16 | 29% | +0.02 | +303 | 0.77 |
| SessionClose | all | 250 | +0.58 | 0 | 0 | 0% | 0 | 0 | 0.61 (win 0.79) |

- regret is a mean-reversion artefact: after any exit there is usually a better fill later in a 1-min path (median +0.16-0.39 %), but the **forgone** column says holding to 11:55 would have lost money for every losing/breakeven group and gained nothing material for winners. no group has both large regret and positive forgone, i.e. no exit reason is systematically early.
- MaxHoldTimeout by which limit fired (table F): the 296 trades cut at 75 min (losing at the check) average -0.49 %, were -0.35 % at 45 and -0.42 % at 60, and would have been -0.57 % at 11:55 (4 % end as winners). the 349 cut at 120 min (winning) average +1.43 %, forgone +$271 total. the 95 that flipped losing after 75 min average -0.02 %.

verdict for winners: **about right**. MaxHold winners capture 77 % of MFE; the 11:55 forgone is +$303 over five years; ScoreExit winners capture only 45 % but there are 30 of them and their forgone is -$28.

## analysis 3: exit-rule simulations (one knob at a time; entries fixed; overlap with a later real entry on the same ticker-day drops that entry — "dropped" column)

| scenario | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | yrs>=0 | n | dropped | win% | PF 22/23/24/25/26 | maxDD$ 22/23/24/25/26 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **current** | +3654 | +479 | +395 | +697 | +666 | +5890 | 5 | 1286 | 0 | 50.6% | 1.57 / 1.17 / 1.13 / 1.20 / 1.26 | 605 / 415 / 870 / 676 / 539 |
| hold to 11:55 only | +2865 | -202 | -169 | +451 | +275 | +3220 | 3 | 1255 | 31 | 53.9% | 1.37 / 0.95 / 0.96 / 1.11 / 1.09 | 1333 / 665 / 919 / 747 / 1000 |
| no ScoreExit | +3682 | +353 | +191 | +606 | +517 | +5349 | 5 | 1282 | 4 | 50.2% | 1.58 / 1.12 / 1.06 / 1.17 / 1.19 | 621 / 462 / 891 / 660 / 709 |
| no HardStop | +3466 | +477 | +577 | +400 | +661 | +5580 | 5 | 1280 | 6 | 50.8% | 1.53 / 1.17 / 1.20 / 1.10 / 1.26 | 624 / 415 / 684 / 1025 / 539 |
| no MaxHold | +3434 | +263 | +185 | +700 | +576 | +5158 | 5 | 1275 | 11 | 52.4% | 1.50 / 1.08 / 1.05 / 1.19 / 1.20 | 679 / 566 / 885 / 694 / 677 |
| max_hold 45 | +3677 | +376 | +930 | +828 | +1075 | +6886 | 5 | 1286 | 0 | 44.7% | 1.82 / 1.18 / 1.44 / 1.30 / 1.65 | 491 / 223 / 675 / 519 / 328 |
| max_hold 60 | +3604 | +426 | +904 | +753 | +1033 | +6719 | 5 | 1286 | 0 | 47.7% | 1.65 / 1.18 / 1.37 / 1.23 / 1.50 | 504 / 335 / 713 / 752 / 360 |
| max_hold 75 | +3716 | +614 | +570 | +462 | +810 | +6172 | 5 | 1286 | 0 | 49.8% | 1.61 / 1.24 / 1.21 / 1.13 / 1.35 | 522 / 315 / 745 / 821 / 420 |
| max_hold 105 | +3497 | +433 | +319 | +851 | +689 | +5791 | 5 | 1277 | 9 | 51.1% | 1.54 / 1.15 / 1.09 / 1.24 / 1.26 | 624 / 475 / 895 / 615 / 571 |
| max_hold 120 | +3435 | +302 | +267 | +810 | +567 | +5381 | 5 | 1276 | 10 | 51.4% | 1.51 / 1.10 / 1.08 / 1.23 / 1.20 | 633 / 579 / 907 / 703 / 669 |
| max_hold 150 | +3434 | +230 | +180 | +672 | +561 | +5077 | 5 | 1275 | 11 | 52.0% | 1.50 / 1.07 / 1.05 / 1.18 / 1.19 | 679 / 583 / 887 / 716 / 681 |
| loss_red 0 | +3589 | +402 | +127 | +833 | +644 | +5594 | 5 | 1277 | 9 | 51.7% | 1.56 / 1.14 / 1.04 / 1.24 / 1.24 | 634 / 477 / 899 / 641 / 572 |
| loss_red 30 | +3638 | +685 | +646 | +448 | +738 | +6155 | 5 | 1286 | 0 | 49.4% | 1.59 / 1.27 / 1.24 / 1.13 / 1.32 | 508 / 315 / 745 / 807 / 473 |
| loss_red 45 | +3464 | +431 | +864 | +529 | +924 | +6211 | 5 | 1286 | 0 | 46.8% | 1.63 / 1.18 / 1.36 / 1.16 / 1.45 | 478 / 334 / 713 / 877 / 392 |
| **loss_red 60** | +3860 | +470 | +894 | +811 | +1071 | +7105 | 5 | 1286 | 0 | 43.6% | 1.86 / 1.22 / 1.42 / 1.29 / 1.64 | 491 / 243 / 675 / 709 / 329 |
| prof_ext 0 | +3708 | +497 | +468 | +985 | +707 | +6365 | 5 | 1286 | 0 | 52.2% | 1.59 / 1.18 / 1.15 / 1.28 / 1.28 | 609 / 412 / 906 / 659 / 482 |
| prof_ext 60 (=90) | +3585 | +591 | +598 | +706 | +686 | +6165 | 5 | 1286 | 0 | 50.2% | 1.56 / 1.21 / 1.20 / 1.20 / 1.27 | 595 / 424 / 870 / 661 / 532 |
| stop 1.0% | +3706 | +632 | +638 | +1079 | +843 | +6897 | 5 | 1286 | 0 | 48.8% | 1.68 / 1.24 / 1.23 / 1.36 / 1.35 | 409 / 430 / 555 / 448 / 460 |
| stop 1.5% | +4071 | +554 | +551 | +827 | +803 | +6806 | 5 | 1286 | 0 | 50.4% | 1.70 / 1.20 / 1.19 / 1.24 / 1.33 | 480 / 463 / 721 / 607 / 470 |
| stop 2.0% | +3908 | +489 | +369 | +751 | +681 | +6198 | 5 | 1286 | 0 | 50.6% | 1.64 / 1.17 / 1.12 / 1.22 / 1.27 | 499 / 415 / 891 / 636 / 532 |
| stop 3.5% | +3446 | +477 | +474 | +497 | +661 | +5555 | 5 | 1283 | 3 | 50.7% | 1.53 / 1.17 / 1.16 / 1.13 / 1.26 | 635 / 415 / 787 / 819 / 539 |
| thr 0.10 | +3510 | +440 | +801 | +754 | +878 | +6382 | 5 | 1286 | 0 | 50.5% | 1.62 / 1.17 / 1.31 / 1.23 / 1.39 | 624 / 389 / 708 / 611 / 402 |
| thr 0.20 | +3597 | +393 | +494 | +721 | +825 | +6031 | 5 | 1286 | 0 | 50.5% | 1.58 / 1.14 / 1.17 / 1.20 / 1.34 | 653 / 432 / 794 / 695 / 473 |
| thr 0.45 | +3745 | +359 | +224 | +616 | +534 | +5478 | 5 | 1283 | 3 | 50.0% | 1.59 / 1.12 / 1.07 / 1.17 / 1.20 | 612 / 483 / 892 / 660 / 702 |
| thr never | +3682 | +353 | +191 | +606 | +517 | +5349 | 5 | 1282 | 4 | 50.2% | 1.58 / 1.12 / 1.06 / 1.17 / 1.19 | 621 / 462 / 891 / 660 / 709 |
| breakeven 0.5% (needs code) | +3361 | +617 | +609 | +1230 | +712 | +6529 | 5 | 1286 | 0 | 49.7% | 1.68 / 1.27 / 1.28 / 1.50 / 1.32 | 531 / 305 / 573 / 433 / 468 |
| breakeven 1.0% (needs code) | +3872 | +375 | +651 | +936 | +704 | +6538 | 5 | 1286 | 0 | 50.9% | 1.67 / 1.14 / 1.24 / 1.29 / 1.28 | 506 / 364 / 717 / 489 / 539 |
| breakeven 1.5% (needs code) | +3851 | +499 | +551 | +819 | +666 | +6386 | 5 | 1286 | 0 | 51.1% | 1.63 / 1.18 / 1.19 / 1.25 / 1.26 | 597 / 372 / 783 / 553 / 539 |
| timestop 20 (== L20/W120) | +3536 | +550 | +974 | +260 | +842 | +6162 | 5 | 1286 | 0 | 40.4% | 1.84 / 1.29 / 1.55 / 1.10 / 1.58 | 507 / 241 / 557 / 493 / 270 |
| timestop 30 (== L30/W120) | +3860 | +470 | +894 | +811 | +1071 | +7105 | 5 | 1286 | 0 | 43.6% | 1.86 / 1.22 / 1.42 / 1.29 / 1.64 | 491 / 243 / 675 / 709 / 329 |
| timestop 45 (== L45/W120) | +3464 | +431 | +864 | +529 | +924 | +6211 | 5 | 1286 | 0 | 46.8% | 1.63 / 1.18 / 1.36 / 1.16 / 1.45 | 478 / 334 / 713 / 877 / 392 |

"hold to 11:55 only" is the important baseline: without the exit stack the short book is negative in 2023 and 2024. the edge of this strategy is substantially in the exits, which is why exit knobs move the result this much — and why they should be verified in the real replay before promotion.

**best single change (5y total, >= 4 of 5 years non-negative): `loss_reduction_ms` 60 min (L30/W120): +7,105, 5/5.** runner-up: `stop_loss_pct` 1.0 % (+6,897, 5/5). the max_hold 45 result (+6,886) is the same losing-limit-30 effect with the winning limit at 75.

**best pair: losing limit 30 + winning limit 90 (`loss_reduction_ms` 3,600,000 + `profit_extension_ms` 0, `max_hold_ms` unchanged): +7,507, 5/5, min year +467.** surface (five-year total / years >= 0 / worst year):

| L \ W | 60 | 75 | 90 | 105 | 120 (cur) | 150 |
|---|---|---|---|---|---|---|
| 15 | +5026 /5/ +283 | +5717 /5/ +510 | +6160 /5/ +586 | +5880 /5/ +512 | +5761 /5/ +446 | +5867 /5/ +432 |
| 20 | +5195 /5/ +153 | +6025 /5/ +366 | +6448 /5/ +480 | +6204 /5/ +303 | +6162 /5/ +260 | +6311 /5/ +286 |
| 25 | +5577 /5/ +202 | +6436 /5/ +413 | +6929 /5/ +485 | +6576 /5/ +431 | +6472 /5/ +409 | +6668 /5/ +468 |
| **30** | +5926 /5/ +192 | +6886 /5/ +376 | **+7507 /5/ +467** | +7114 /5/ +393 | +7105 /5/ +470 | +7238 /5/ +562 |
| 35 | +5663 /5/ +137 | +6713 /5/ +344 | +7353 /5/ +440 | +6941 /5/ +343 | +6948 /5/ +427 | +7137 /5/ +512 |
| 40 | +5381 /5/ +230 | +6411 /5/ +444 | +7068 /5/ +539 | +6633 /5/ +433 | +6661 /5/ +520 | +6912 /5/ +595 |
| 45 | +4953 /5/ +132 | +6013 /5/ +346 | +6719 /5/ +426 | +6186 /5/ +328 | +6211 /5/ +431 | +6468 /5/ +513 |
| 50 | +5248 /5/ +236 | +6343 /5/ +530 | +7010 /5/ +625 | +6466 /5/ +447 | +6456 /5/ +466 | +6791 /5/ +482 |
| 60 | +4809 /5/ +281 | +6032 /5/ +632 | +6738 /5/ +700 | +6172 /5/ +462 | +6155 /5/ +448 | +6419 /5/ +461 |
| 75 (cur) | +4485 /5/ +131 | +5618 /5/ +409 | +6365 /5/ +468 | +5870 /5/ +320 | **+5890 /5/ +395** | +6165 /5/ +591 |

reading the surface: the L dimension is where the money is (L 25-40 at any W beats current by $600-1,600); W is second-order and non-monotone (90 > 150 > 120 ~ 105 > 75 > 60 at every L), which is consistent with the winner-side analysis: winners are captured well at 90 and the extra 30 minutes to 120 mostly adds trades that flip to a loss late.

interactions on top of L30/W90: stop 1.0/1.5/2.0/2.5 % => +7,124 / +7,505 / +7,612 / +7,507; thr 0.1/0.2/0.3/never => +7,086 / +7,459 / +7,507 / +7,578; breakeven 0.5 % (code) +7,809. nothing else stacks materially; the stop and score exit become almost inert.

## robustness of the recommendation

per ticker (current / L30W90):

| year | AMZN | AAPL | NVDA | MSFT |
|---|---|---|---|---|
| 2022 | +1022 / +742 | +624 / +569 | +1636 / +2069 | +373 / +542 |
| 2023 | -76 / -113 | +84 / +83 | +427 / +380 | +44 / +116 |
| 2024 | -159 / -21 | -230 / +40 | +660 / +657 | +123 / +224 |
| 2025 | -167 / +6 | +465 / +250 | +411 / +617 | -12 / +179 |
| 2026 | +98 / +102 | +177 / +296 | +138 / +238 | +252 / +531 |
| ALL | +718 / +714 | +1121 / +1238 | +3272 / +3962 | +780 / +1592 |

per entry window (current / L30W90): thrust 2022-2026 = +1967/+1931, +512/+450, +369/+554, +533/+685, -96/+311 (5y +3,286 -> +3,931); core = +1687/+1991, -33/+16, +26/+346, +164/+368, +761/+856 (5y +2,604 -> +3,576). the 2022 and 2023 improvements come from NVDA/MSFT and the core window while AMZN/AAPL/thrust give a little back; 2024-2026 improve everywhere.

what the rule actually does (R3): 1,116 of 1,286 trades exit earlier; 530 exit at exactly 30-31 min; hold distribution goes from 5 % / 16 % / 41 % / 38 % (<=31 / 32-60 / 61-91 / >91 min) to 47 % / 18 % / 35 % / 0 %.

## what the evidence does NOT support / could not be determined

- **freed capital re-entry is not modelled.** cutting at 30 min frees the ticker ~45 minutes earlier on ~half the trades; the windows may fire again before 11:30 on the same ticker. those extra trades' P&L is unknown (could be positive, could be more losers). only the real replay (`backtest` with the overrides below) can settle it. the sim only drops entries that become impossible under *longer* holds (0 dropped for all recommended scenarios).
- **the exact optimum (30 vs 25/35/40) is within noise**: the L surface has bumps (45 is a dip, 50 a bump) that no mechanism explains. treat "25-40 minutes" as the supported range, 30 as the centre. the direction (shorter than 75) is supported in every year, ticker, window, and fold.
- **2023 does not improve** under any version of the time stop (+479 -> +467/+470). it is not hurt either.
- **win rate falls to 44 %** and the exit mix becomes ~88 % MaxHoldTimeout. the strategy becomes "30-minute short scalp with a 90-minute cap"; if the check-in skills or the cockpit key on win rate or exit mix they will look different.
- **ATR trailing stop** not simulated (fires once in five years); **breakeven stop is a no-op in the engine** (finding 5), so the brief's exit list overstates the stack — anything the trailing stop would do under a different ATR multiplier is out of scope here.
- "regret" is close-to-close on 1-minute bars and always finds a better fill somewhere later; it should not be read as "we could have got that".
- the sim is sensitive to the tick file's 4-decimal composite only at the exact 0.30 boundary (1 trade in 1,286).
- lower `exit_threshold` (0.10) and tighter stop (1.0 %) both show 5/5 and pass leave-one-year-out on their own, but they are redundant with the time stop; if the time stop is adopted, they are not worth a separate config version. if it is NOT adopted, `stop_loss_pct` 0.015 is the next-best single change (+916, 5/5, DD down in every year).

## recommended config change (config-only, no code)

`max_hold` action params: `max_hold_ms` 5,400,000 (keep), **`loss_reduction_ms` 900,000 -> 3,600,000**, **`profit_extension_ms` 1,800,000 -> 0**. effect: losing at any bar >= 30 min -> exit; winning -> exit at 90 min; 11:55 close unchanged. optional: `hard_stop.stop_loss_pct` 0.025 -> 0.015 for drawdown (P&L-neutral on top of the time stop).

verify with the real replay before promoting (this also answers the re-entry question):
```
backtest --date <YYYY-MM-DD> --lookback-days N --bars-dir data/bars --short-only --sizing-fraction 0.36 --max-position-pct 0.36 \
  --disable-action window_5m_thrust,window_strong_core,window_candle_reversal \
  --loss-reduction-ms 3600000 --profit-extension-ms 0 [--set-action-param hard_stop:stop_loss_pct=0.015]
```
(flag names from `crates/backtest/src/main.rs`; check `--set-action-param` syntax there.) acceptance: >= current in 4 of 5 years and no year negative in the replay, with the extra re-entries included.

## files

- `/tmp/claude-1000/-home-dylmet-Projects-galactic-trading-firm/48685858-bad9-4000-a57f-b038fffcc726/scratchpad/exits/exitsim.py` — loader, path builder, exit-stack simulator, metrics (`python3 exitsim.py` builds `cache.pkl`, ~4 s)
- `verify.py` — reproduction check (writes `repro_mismatches.csv`)
- `analysis.py` — analyses 1-2 (`tables_paths.md`, `paths.csv`)
- `sweeps.py`, `sweeps2.py`, `robust.py` — analysis 3 (`tables_sweeps.md`, `tables_sweeps2.md`, `tables_robust.md`)
