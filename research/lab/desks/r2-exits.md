# r2-exits — exits and holds on row 25's windows

Desk of lab round 2 (`ROUND2.md`). Base `research/lab/specs/base/row25.json` (row 25, research sizing 0.36 / cap 0.45).
Every cell is a patch on `config_id: 25`, specs `research/lab/specs/r2/exits/`, patches `research/lab/patches/r2/exits/`.

## exit mechanics on row 25 (read from the code before designing)

- `max_hold` (`max_hold_timeout`): 90 min if winning, 90 − 50 = **40 min if losing** (`loss_reduction_ms` 3.0 M). Global.
- a window's `exit_overrides.max_hold_ms` is **a floor, not a limit** (`tick_loop.rs` ~l.380): while an override ≠ the engine
  default is active, a `MaxHoldTimeout` signal is ignored until the hold reaches the override. So an override can only
  *lengthen* a window's hold, and it lengthens the losing side as well (stress: 2 h for winners *and* losers). Shortening one
  window needs a shorter global action plus overrides on the others.
- the engine default max hold is the `max_hold` action's `max_hold_ms` in the backtest (`replay.rs` calls `set_max_hold_ms`),
  but the live trader never calls `set_max_hold_ms` (default 2.7 M). Harmless for row 25 (no override equals either value,
  no position-context indicator), but it means an override equal to 5.4 M would be a no-op in replay and active live —
  every override below is chosen ≠ 5.4 M and ≠ 2.7 M.
- score exit: global `scoring.exit_threshold` −0.3 (a short exits when the composite ≥ +0.3); stress overrides −0.6.
- `vwap_stop` scope keys on `position_context.max_hold_ms` (= the active override). To scope it to pm + stress only, the
  am windows get a neutral override of 1 ms and the pm windows a neutral override of 2,400,000 (= the losing-side hold,
  so the floor never binds), scope ≥ 2. Scaffold cell `scaffold_ovr` (these overrides, no vwap stop) must equal the base.

## idea list (written before any run)

| # | cell | change | mechanism | how it fails |
|---|---|---|---|---|
| 1 | scaffold_ovr | neutral overrides am 1 ms / pm 2.4 M | none — scaffold for the pm-scoped VWAP stop | if not identical to base, the override floor reasoning is wrong |
| 2 | mh60 | max hold 60/40 (win/lose) | MaxHold carries the book; shorter winners bank the thrust before the late-morning fade | cuts the right tail; D2 says winners already exit near 77 % of MFE |
| 3 | mh120 | 120/40 | let winners run past 90 min | D2: every extension loses; pm trades hit 11:55 anyway |
| 4 | lr30 | 90/30 | cut losers sooner (losers rarely recover) | the 40-min loser is often mid-retest; cutting sooner books the low |
| 5 | lr60 | 90/60 | give losers more time | more ScoreExit/hard-stop losses |
| 6 | lr0 | 90/90 | loss reduction off | as 5, larger |
| 7 | pm_hold90 | pm windows' losers held 90 (override 5,340,001) | pm thrusts start later in a trend; give them the same clock as winners | as 5, on pm only |
| 8 | pm_fx1230 | pm windows force_exit 12:30 | pm entries after 10:25 are cut by 11:55 before their 90 min | the 11:55 cut sits before the lunch reversal; holding into lunch gives it back |
| 9 | st_mh90 | stress max hold 90 (override 5,340,001) | stress shorts revert; shorter hold banks it | stress is 2022-dominated, few trades — noise |
| 10 | st_mh180 | stress max hold 3 h | crash days trend into the close | half the SPY-touch days reverse (D7) |
| 11 | st_seoff | stress score exit off (−1.1) | ScoreExit loses everywhere; on stress it may cut early | frontier: ScoreExit is selection, not a bad exit |
| 12 | st_se03 | stress score exit at the default −0.3 | tighter exit on stress reversal | more churn |
| 13 | tr5 | ATR trail 5× | tighter trail locks more of the thrust | D2: tighter stops whipsaw |
| 14 | tr10 | ATR trail 10× | trail almost never binds — tests whether it matters | none expected |
| 15 | troff | trail disabled | as 14 | as 14 |
| 16 | hs15 | hard stop 1.5 % | cap the tail loser | stops out at the high of a retest |
| 17 | hs40 | hard stop 4 % | the stop rarely binds — tests whether it matters | larger tail |
| 18 | be03 | breakeven at 0.3 % | protect small winners | BreakevenStop is already −234/50 on v18 (by design); more scratch exits |
| 19 | be08 | breakeven at 0.8 % | fewer scratch exits, let thrusts breathe | larger giveback |
| 20 | beoff | breakeven disabled | as 19, extreme | as 19 |
| 21 | vw_st | VWAP stop on stress only (buffer 0.2 %) | stress shorts are below VWAP; a reclaim kills the thesis | VWAP is far above on stress days — never fires (D7: "VWAP retest never fires when it matters") |
| 22 | vw_pm | VWAP stop on pm + stress (buffer 0.2 %) | pm entries happen near VWAP; a reclaim ends the thrust | whipsaw around VWAP adds losers |
| 23 | vw_all | VWAP stop on every window (buffer 0.2 %) | as 22 for am too | as 22 |
| 24 | rq14_p1 | max hold +1 bar (91/41, stress 2 h 1 m) | RQ-14 sensitivity: live starts the clock one bar later | the book should be insensitive; a large move means the edge sits on a clock edge |
| 25 | rq14_m1 | max hold −1 bar (89/39, stress 1 h 59 m) | as 24 | as 24 |

Dropped before any run: `tr10` and `hs40` — the base's train trades have 0 TrailingStop exits and 3 HardStop exits (−295), the screen window none, so a wider trail is identical by construction and a wider hard stop touches 3 trades; `troff` and `hs15` stay as the informative ends.

Not run, already known: score exit off / −0.5 on the am/pm windows (frontier `na_seoff` = `na_se05` on the core, "no change"
on the bundle); profit extensions and re-entry (D2).

## stage 1: screen, `2024h2_25h1` (249 sessions), 23 cells

Base row 25 on this window: +863 / 89 / PF 1.86 / dd −217 / p 0.048. "chg / add / rem" = trades whose exit changed / trades
added / trades removed vs the base (`.trades.csv` diff keyed on date, ticker, entry time).

| cell | P&L | n | PF | dd | p | chg / add / rem | 2024h2 | 2025h1 |
|---|---:|---:|---:|---:|---:|---|---|---|
| base | +863 | 89 | 1.86 | −217 | 0.048 | — | +482/48/1.85 | +380/41/1.87 |
| scaffold_ovr | +863 | 89 | 1.86 | −217 | 0.048 | 0/0/0 | identical | identical |
| be03 | +925 | 90 | 2.15 | −223 | 0.022 | 19/1/0 | +535/49/2.37 | +391/41/1.94 |
| be08 | +872 | 87 | 1.87 | −217 | 0.047 | 9/0/2 | +499/46/1.91 | +372/41/1.81 |
| beoff | +892 | 86 | 1.86 | −217 | 0.048 | 12/0/3 | +493/45/1.85 | +400/41/1.88 |
| hs15 | +707 | 89 | 1.64 | −174 | 0.066 | 8/0/0 | +483/48/1.86 | +223/41/1.42 |
| lr0 (90/90) | +953 | 89 | 1.91 | −242 | 0.041 | 28/0/0 | +415/48/1.65 | +538/41/2.32 |
| lr30 (90/30) | +534 | 90 | 1.50 | −171 | 0.107 | 38/1/0 | +360/49/1.63 | +174/41/1.34 |
| lr60 (90/60) | +903 | 89 | 1.84 | −244 | 0.050 | 26/0/0 | +363/48/1.53 | +541/41/2.39 |
| mh60 (60/40) | +741 | 89 | 1.74 | −217 | 0.058 | 36/0/0 | +359/48/1.64 | +382/41/1.88 |
| mh120 (120/40) | +727 | 89 | 1.68 | −217 | 0.074 | 31/0/0 | +490/48/1.85 | +237/41/1.48 |
| pm_fx1230 | +838 | 89 | 1.84 | −217 | 0.051 | 5/0/0 | +467/48/1.83 | +371/41/1.85 |
| pm_hold90 | +1004 | 89 | 2.04 | −217 | 0.032 | 8/0/0 | +488/48/1.87 | +516/41/2.26 |
| st_mh90 | +836 | 89 | 1.83 | −217 | 0.051 | 2/0/0 | +482/48/1.85 | +353/41/1.80 |
| st_mh180 | +818 | 89 | 1.80 | −217 | 0.055 | 2/0/0 | +482/48/1.85 | +336/41/1.74 |
| st_seoff | +863 | 89 | 1.86 | −217 | 0.048 | 0/0/0 | identical | identical |
| st_se03 | +671 | 89 | 1.59 | −217 | 0.088 | 4/0/0 | +482/48/1.85 | +188/41/1.33 |
| tr5 | +863 | 89 | 1.86 | −217 | 0.048 | 0/0/0 | identical | identical |
| troff | +863 | 89 | 1.86 | −217 | 0.048 | 0/0/0 | identical | identical |
| vw_st | +733 | 89 | 1.68 | −217 | 0.067 | 2/0/0 | +482/48/1.85 | +250/41/1.50 |
| vw_pm | +700 | 89 | 1.63 | −217 | 0.075 | 5/0/0 | +458/48/1.78 | +242/41/1.47 |
| vw_all | +527 | 97 | 1.46 | −177 | 0.122 | 28/8/0 | +376/53/1.59 | +150/44/1.29 |
| rq14_p1 (+1 bar) | +834 | 89 | 1.82 | −215 | 0.051 | 51/0/0 | +476/48/1.82 | +358/41/1.83 |
| rq14_m1 (−1 bar) | +859 | 90 | 1.82 | −221 | 0.052 | 51/1/0 | +478/49/1.81 | +381/41/1.84 |

Screen reading (trade diffs):
- **be03**: of +63, +89 is one *added* pm trade (an earlier trade scratched out sooner, freeing the ticker); the 19 changed
  exits net −26 (7 pm MaxHold winners +54 → breakeven −34). The PF jump is from fewer large ScoreExit losers turning into scratches.
- **lr0 / lr60 / pm_hold90**: holding a 40-min loser longer splits it in two — about half recover into MaxHold / SessionClose
  exits (+), the other half run into a ScoreExit at a bigger loss (−). pm_hold90's +141 is 8 trades, two of them SessionClose +99.
- **max hold 90 min is a local optimum** both ways (60 → −122, 120 → −136). mh60 cuts pm winners (−109 on 6 trades).
- **ATR trail never binds** (7× 5m ATR; 0 TrailingStop exits on the screen and on train) — tr5 / troff identical.
- **VWAP stop loses everywhere**, stress-only too: the stress trades it cuts (2 of 8) were +61 winners that dipped above VWAP + 0.2 %
  and then came back; on pm it cuts 3 losers at a larger loss. D7 ("VWAP retest never fires when it matters") extends to exits.
- **stress exits**: score exit off = identical (the stress composite never reaches +0.6 while short), tighter −0.3 cuts 2 winners
  (−192); 90 min / 3 h each −27 / −45 on 2 trades. The stress window's exits are at their best values on this window.
- **hard stop 1.5 %** turns 2 stress winners into stops (−200).
- **RQ-14 ±1 bar**: −29 / −4 with 51 changed exits — the book does not sit on a clock edge.

## stage 2: train (874 sessions), 6 cells, each `lab.py compare … base/row25.json --window train`

`base/row25` on train reproduces `frontier/na_bundle` exactly (+2610 / 320 / PF 1.75). "2023–25h1" = ex-best-year (2022 excluded).
Paired = day bootstrap A − B vs row 25.

| cell | P&L | n | PF | dd | worst day | sign-flip p | 2022 | 2023 | 2024 | 2025h1 | 2023–25h1 | paired diff / P(≤0) |
|---|---:|---:|---:|---:|---:|---:|---|---|---|---|---|---|
| base row 25 | +2610 | 320 | 1.75 | −238 | −168 | 0.0007 | +1443/119/2.43 | +136/71/1.15 | +650/89/1.58 | +380/41/1.87 | +1167/201/1.47 | — |
| pm_hold90 | +2663 | 320 | 1.73 | −280 | −168 | 0.0010 | +1390/119/2.20 | +88/71/1.09 | +669/89/1.61 | +516/41/2.26 | +1273/201/1.51 | +53 / 0.377 |
| lr60 (90/60) | +2625 | 318 | 1.70 | −249 | −190 | 0.0013 | +1392/118/2.24 | +156/70/1.16 | +538/89/1.43 | +541/41/2.39 | +1234/200/1.47 | +16 / 0.467 |
| lr0 (90/90) | +2494 | 318 | 1.63 | −316 | −190 | 0.0020 | +1333/118/2.10 | +87/70/1.08 | +536/89/1.43 | +538/41/2.32 | +1161/200/1.43 | −116 / 0.702 |
| be03 | +2016 | 333 | 1.62 | −275 | −168 | 0.0057 | +1043/124/2.06 | +80/75/1.10 | +503/93/1.51 | +391/41/1.94 | +974/209/1.43 | −593 / 0.982 |
| rq14_p1 (+1 bar) | +2552 | 320 | 1.72 | −242 | −165 | 0.0013 | +1436/119/2.41 | +111/71/1.11 | +648/89/1.57 | +358/41/1.83 | +1116/201/1.44 | −57 / 0.838 |
| rq14_m1 (−1 bar) | +2583 | 322 | 1.73 | −273 | −170 | 0.0007 | +1449/119/2.44 | +140/72/1.15 | +613/90/1.53 | +381/41/1.84 | +1134/203/1.45 | −27 / 0.638 |

Daily P&L correlation with row 25: 0.93 (be03) to 1.00 (rq14).

Which trades changed on train (`.trades.csv` diff vs base):
- **be03** (−593): 16 am-thrust MaxHold winners (+356) become 0.3 % scratches (−75), 14 pm ones +65 → −78, one core winner
  +103 → −1; it rescues 8 ScoreExit losers (+174) and adds 13 re-entries (net −98, one stress HardStop −117). The screen's
  +63 was one added trade. The 0.5 % breakeven is not the leak: the thrusts that pay dip back to entry first.
- **lr0** (−116): 87 changed losers. Holding a 40-min loser to 90 min: 48 improve (37 still MaxHold but smaller, 9 recover into SessionClose, 2 breakeven;
  +545 together), 36 run on into a ScoreExit at a larger loss (−660). The 40-min cut is roughly a fair trade between the two populations; going to 60
  (lr60, +16) is the same split at a smaller scale.
- **pm_hold90** (+53): 35 pm losers. 10 SessionClose recoveries +231, 11 MaxHold +65, 13 that run into ScoreExit −241. 2023
  −48, 2022 −53, 2024 +19, 2025h1 +136: the gain is 2025h1 only.
- **rq14 ±1 bar** (−57 / −27): ~170 exits move by one bar, each by a few dollars; nothing in the book sits on the max-hold
  clock edge. Answer to RQ-14's max-hold item: the live / replay one-bar offset is worth about ±$50 on 3.5 years (≈ 2 % of P&L).

## reading

Row 25's exits are at a local optimum on every dial this desk could turn: max hold 90 (60 and 120 both lose ~$120 on the
screen), the 40-minute losing-side cut (30 loses $330 on the screen, 60 / 90 are flat to negative on train), breakeven 0.5 %
(0.3 % −593 on train; 0.8 % / off ≈ 0 on the screen), hard stop 2.5 % (1.5 % −156), the stress window's 2 h / −0.6 (all four
variants ≤ 0). The ATR trail is inert (0 TrailingStop exits in 320 trades). A VWAP stop loses on every scope. The only cells
with a positive train diff (pm_hold90 +53, lr60 +16) are within one bootstrap standard deviation of zero and come from 2025h1.
This is HISTORY D2 again, now on row 25: winners exit near the right place, and the losing trades are selection, not exits —
holding them longer swaps one loser population for another.

## dead ends

VWAP stop at 0.2 % on stress / pm+stress / all windows (−130 / −162 / −336 on the screen); hard stop 1.5 %; tighter stress
score exit; stress hold 90 min / 3 h; max hold 60 or 120; loser cut at 30 min; breakeven 0.3 %; pm flat at 12:30 (−25);
ATR trail 5× / off (identical). Stress score exit off is identical (the composite never reaches +0.6 while a stress short is open).

## engine proposals

1. **The live trader never calls `set_max_hold_ms`** (`crates/data_feed`, only `backtest/src/replay.rs` does), so live
   `default_max_hold_ms` is 2.7 M while replay uses the `max_hold` action's 5.4 M. On row 25 this is harmless (the only override,
   stress 7.2 M, differs from both, and no position-context indicator is enabled), but a window override of exactly 5.4 M would
   be a no-op in replay and a floor live, and `vwap_stop`'s `scope_min_max_hold_ms` sees different values for override-free
   windows (2.7 M live vs 5.4 M replay). Fix: call `set_max_hold_ms` in `config_watcher::try_build_engine` / book build the same way.
2. **A window `max_hold_ms` override is a floor, not a limit** (it delays the `max_hold` action's signal, so it also stretches
   the losing side). A per-window `loss_reduction_ms` (or overriding the action's params per window) would let a window keep a
   short losing hold with a longer winning one. Not worth building on this evidence (lr / pm_hold cells are flat), noted so
   nobody reads a window `max_hold_ms` override as "shorter for this window".
3. A conditional loser hold — keep a 40-min loser only if the composite is still ≤ some short level — is the one idea the
   lr0 diff suggests (the recoverers and the ScoreExit losers are separable only by what the score does after 40 min). It
   needs a `max_hold_timeout` param such as `loss_hold_if_composite_below`; I would expect it to be small.

## nominations

**None for the holdout.** Best of 23 specs (29 runs; 23 screened, 6 confirmed on train). No cell passes success test T's
first clause (paired vs row 25 P(diff ≤ 0) < 0.05 on train): the best is `pm_hold90` at P 0.377. None is additive (A does not
apply to exit changes: they add no trades). I recommend not spending a finalist slot on an exits cell.

If the coordinator wants one exits cell in the finalist list anyway, it is:
- `research/lab/specs/r2/exits/pm_hold90.json` (patch `research/lab/patches/r2/exits/pm_hold90.json`): the pm windows hold
  their losers 90 min instead of 40 (exit override `max_hold_ms` 5,340,001). Targets T. Train +2663 / 320 / PF 1.73, paired
  +53, P 0.377 — fails T's train clause. Best of 23. Holdout expectation (written before any reveal): within ±$60 of row 25
  on the holdout's ~35 pm trades, sign unpredictable; P(diff ≤ 0) ≈ 0.4–0.6.

Results that should change what the next round does:
1. RQ-14's max-hold clock offset is immaterial (±1 bar = −57 / −27 on train, ~170 exits moved by a few dollars each) —
   that item of the restatement bundle can be closed.
2. Exit tuning on row 25 is exhausted (D2 holds on the new base); the VWAP stop is negative on every scope, stress included.
