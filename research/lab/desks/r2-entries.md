# desk r2-entries: entry filters and window clocks on row 25 (round 2, 2026-11-11)

Agent `r2-entries`. Base `research/lab/specs/base/row25.json` (config row 25, research sizing, cap 0.45, lookback 8).
Every cell is a self-contained patch on row 25 built by `research/lab/patches/r2/entries/make.py`; it re-declares the
affected windows with their full params (replace-in-place by instance_id) and adds weight-0 helper indicators.
Holdout never run. No engine / DB / pipeline / git change.

## 1. idea list (written before any run)

Frontier already mapped k 0.04–0.06, VPIN floor 0.15–0.217, global thrust 1h −0.05…−0.20 and stress 1.5/2.0 %;
those are not re-run. Quality / peer conditions are added to the four am/pm windows only (the stress window is a
different regime with 33 train trades; it stays as is unless the cell is about it).

| id | cell | change | mechanism | how it fails |
|---|---|---|---|---|
| C1 | cut0950 | am → pm cut-over 09:50 (sqrt band from 09:50) | the flat ±0.2 % band is a clock; sqrt band admits 09:50–10:00 thrusts on moving SPY days | the 09:45–10:00 hour is unstable across cells (meta); adds PF≈1 trades |
| C2 | cut1010 | cut-over 10:10 | the flat band is stricter 10:00–10:10, may drop marginal early-pm trades | removes some of the PF-9.7 pm block |
| C3 | pm1115 | pm windows to 11:15 | the 10:00–11:00 block is the one positive-every-period block; extend it | after-11:30 dilution reaches back to 11:00 |
| C4 | pm1130 | pm windows to 11:30 | same, tf_sq05_1h15 traded to 11:30 | same, plus fewer minutes to the 11:55 flat |
| C5 | am0935 | am windows from 09:35 | first-five-minute 5m scores are built on one or two bars of the session | the earliest thrust is the edge (D4) |
| C6 | st1000 | stress entries from 10:00 | opening-crash entries 09:45–10:00 often reverse (D7: "SPY touch reverses half the days") | removes the best 2022 crash trades |
| C7 | st1300 | stress entries until 13:00 | afternoon stress shorts are EOD-reversal exposed (D3) | the stress window is +ve throughout |
| P1 | pdl15 | `pdl_5m.dist_low_pct ≥ −1.5` on am/pm windows | the −1.0 cut removed PF-1.23 trades (too tight); looser cut keeps more of them and only drops shorts far below yesterday's low (exhausted) | shorts below the prior low are the 2022 crash winners; same "cuts profit" result |
| P2 | pdl25 | ≥ −2.5 | same, a looser point on the dial | binds too rarely to matter |
| Q1 | vwbelow | `vwapd_1m.dist_pct ≤ 0` (short only below session VWAP) | a 5m thrust above VWAP fights the session | thrust shorts are already almost all below VWAP — never binds |
| Q2 | vwstretch | `vwapd_1m.dist_pct ≥ −1.0` (not over-stretched) | RQ-10: over-stretched shorts mean-revert | stretched names are the strong trend days (2022) |
| Q3 | rvol10 | `rvol_5m.rvol ≥ 1.0` (5m bar volume ≥ its 20-bar mean) | volume-confirmed thrust | VPIN already encodes flow; drops quiet-but-good trades |
| Q4 | rvolcap | `rvol_5m.rvol ≤ 3.0` | a climax bar is capitulation, not continuation | climaxes are the crash-day winners |
| Q5 | redday | `gap_5m.above_open ≤ 0` (name below today's open) | short only names red on the day | redundant with the 1h / 5m scores |
| X1 | peersred | `cross_1m.peers_red_frac ≥ 0.6` (≥ 2 of 3 peers red) | sector confirmation: shorts with the group | SPY band already forces flat-index days; peers-red then = sector selloff, which may reverse |
| X2 | peersgreen | `peers_red_frac ≤ 0.4` (name weak, peers not) | idiosyncratic weakness persists intraday | the complement of X1; one of the two must lose |
| X3 | spy15 | `cross_1m.index_ret_15m ≤ +0.05` (SPY not rising over the last 15 m) | shorts into a rising index tape get squeezed | the band already keeps SPY near flat; binds little |
| X4 | relweak | `pdl_5m.rel_close_pct ≤ 0` (name down vs SPY since prior close) | relative-strength persistence | names that gapped up and fade are good shorts too |
| T1 | t1h_am15 | thrust 1h ≤ −0.15 on am, −0.10 pm | frontier: most of the 1h dial's work is in the am thrust; keep pm volume | volume loss for the same PF (nb_t15) |
| T2 | t1h_pm15 | thrust 1h −0.10 am, −0.15 pm | the other half | pm 1h already found to matter (pm0 lost) |
| I1 | pmv18 | VPIN 0.18 on the pm windows only | the pm block runs at high PF; buy pm volume where trades are good | sub-floor population is PF 0.6 everywhere (D5) |

Pre-registered reading rule: a cell is interesting for **T** only if it *raises* P&L (paired vs row 25) — a pure cut that raises
PF while lowering P&L fails T by construction; a cell that adds trades is interesting for **A** only if the added trades
alone are PF ≥ 1.3 on train.

## 2. scaffold

`r2-entries/scaffold` re-declares all five windows unchanged and adds all four helper indicators at weight 0. On 2025h1 it
matches `base/row25` trade for trade: 41 / 41 trades, P&L diff +0, correlation 1.00. Every cell is the same scaffold with one
change.

## 3. ledger

This desk wrote 35 ledger lines under `r2-entries`: 2 base/row25 runs (2024h2_25h1 and train; shared cache), 1 scaffold run
(2025h1), 24 screens on 2024h2_25h1 and 8 train confirmations, one of which (`peersnotall_thr`) went straight to train without
a screen; see §5. That is **25 distinct non-scaffold cells** (26 specs including the scaffold) and 33 non-base runs. Charge
`cells_scanned` = 33 for this desk.
The `compare` runs do not write ledger lines.

## 4. all numbers (lab, research sizing)

### screen, 2024h2_25h1 (249 sessions)
| cell | P&L | n | PF | dd | worst day | sign-flip p | ex-best PF | per year (P&L/n/PF) |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| base__row25 | +863 | 89 | 1.86 | -217 | -168 | 0.048 | 1.87 | 24 +482/48/1.85 · 25 +380/41/1.87 |
| am0935 | +758 | 87 | 1.79 | -177 | -168 | 0.0537 | 1.70 | 24 +482/48/1.85 · 25 +276/39/1.70 |
| cut0950 | +828 | 90 | 1.80 | -217 | -168 | 0.056 | 1.87 | 24 +448/49/1.75 · 25 +380/41/1.87 |
| cut1010 | +863 | 89 | 1.86 | -217 | -168 | 0.048 | 1.87 | 24 +482/48/1.85 · 25 +380/41/1.87 |
| pdl15 | +916 | 64 | 3.00 | -174 | -77 | 0.0087 | 3.83 | 24 +486/36/2.58 · 25 +430/28/3.83 |
| pdl25 | +612 | 77 | 1.71 | -217 | -168 | 0.0903 | 1.53 | 24 +409/43/1.85 · 25 +203/34/1.53 |
| peersgreen | +699 | 36 | 4.56 | -61 | -58 | 0.0113 | 2.64 | 24 +210/16/2.64 · 25 +489/20/8.15 |
| peersgreen_am | +948 | 59 | 2.99 | -184 | -72 | 0.0157 | 2.09 | 24 +359/29/2.09 · 25 +589/30/5.04 |
| peersnotall_am | +988 | 74 | 2.34 | -134 | -77 | 0.01 | 1.94 | 24 +440/37/1.94 · 25 +548/37/3.04 |
| peersred | +356 | 70 | 1.39 | -248 | -168 | 0.1987 | 1.03 | 24 +342/37/1.68 · 25 +14/33/1.03 |
| pm1115 | +905 | 95 | 1.87 | -217 | -168 | 0.0403 | 1.78 | 24 +539/51/1.94 · 25 +367/44/1.78 |
| pm1130 | +844 | 98 | 1.77 | -217 | -168 | 0.048 | 1.78 | 24 +478/54/1.75 · 25 +367/44/1.78 |
| pmv18 | +830 | 90 | 1.81 | -217 | -168 | 0.0497 | 1.86 | 24 +453/49/1.77 · 25 +378/41/1.86 |
| redday | +823 | 88 | 1.82 | -217 | -168 | 0.0543 | 1.79 | 24 +482/48/1.85 · 25 +340/40/1.79 |
| relweak | +907 | 84 | 1.96 | -217 | -168 | 0.042 | 2.13 | 24 +475/47/1.84 · 25 +432/37/2.13 |
| relweak_pm1115 | +930 | 89 | 1.94 | -217 | -168 | 0.0327 | 2.01 | 24 +512/49/1.89 · 25 +418/40/2.01 |
| rvol10 | +444 | 62 | 1.57 | -213 | -168 | 0.1397 | 1.32 | 24 +132/33/1.32 · 25 +313/29/1.83 |
| rvolcap | +672 | 87 | 1.66 | -227 | -178 | 0.086 | 1.49 | 24 +453/47/1.78 · 25 +219/40/1.49 |
| spy15 | +859 | 83 | 1.89 | -217 | -168 | 0.0423 | 1.89 | 24 +476/43/1.89 · 25 +384/40/1.89 |
| st1000 | +863 | 89 | 1.86 | -217 | -168 | 0.048 | 1.87 | 24 +482/48/1.85 · 25 +380/41/1.87 |
| st1300 | +841 | 82 | 1.87 | -211 | -168 | 0.0457 | 1.89 | 24 +485/47/1.86 · 25 +356/35/1.89 |
| t1h_am15 | +761 | 81 | 1.77 | -217 | -168 | 0.0623 | 1.87 | 24 +385/41/1.69 · 25 +376/40/1.87 |
| t1h_pm15 | +727 | 85 | 1.70 | -217 | -168 | 0.072 | 1.80 | 24 +379/47/1.63 · 25 +349/38/1.80 |
| vwbelow | +874 | 87 | 1.88 | -217 | -168 | 0.0443 | 1.89 | 24 +486/46/1.86 · 25 +388/41/1.89 |
| vwstretch | +623 | 69 | 1.89 | -217 | -168 | 0.0677 | 1.43 | 24 +453/35/2.49 · 25 +170/34/1.43 |
### confirm, train 2022-01-03 → 2025-06-30 (874 sessions)
| cell | P&L | n | PF | dd | worst day | sign-flip p | ex-best PF | per year (P&L/n/PF) |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| base__row25 | +2610 | 320 | 1.75 | -238 | -168 | 0.0007 | 1.47 | 22 +1443/119/2.43 · 23 +136/71/1.15 · 24 +650/89/1.58 · 25 +380/41/1.87 |
| pdl15 | +2634 | 256 | 2.16 | -174 | -95 | 0.0 | 1.95 | 22 +1151/98/2.66 · 23 +330/60/1.48 · 24 +723/70/1.99 · 25 +430/28/3.83 |
| peersgreen | +2022 | 140 | 2.61 | -131 | -116 | 0.0 | 2.57 | 22 +968/68/2.65 · 23 +212/22/1.67 · 24 +353/30/2.23 · 25 +489/20/8.15 |
| peersgreen_am | +2585 | 224 | 2.27 | -184 | -116 | 0.0 | 2.12 | 22 +1187/94/2.50 · 23 +194/46/1.33 · 24 +614/54/2.19 · 25 +589/30/5.04 |
| peersnotall_am | +2771 | 273 | 1.99 | -260 | -119 | 0.0 | 1.72 | 22 +1398/109/2.56 · 23 +123/55/1.16 · 24 +702/72/1.79 · 25 +548/37/3.04 |
| peersnotall_thr | +2839 | 282 | 1.98 | -218 | -119 | 0.0 | 1.72 | 22 +1460/114/2.51 · 23 +98/56/1.12 · 24 +743/74/1.86 · 25 +538/38/2.93 |
| pm1115 | +2478 | 340 | 1.65 | -269 | -168 | 0.003 | 1.40 | 22 +1405/126/2.25 · 23 -8/75/0.99 · 24 +714/95/1.62 · 25 +367/44/1.78 |
| relweak | +2597 | 313 | 1.75 | -238 | -168 | 0.0007 | 1.50 | 22 +1386/117/2.36 · 23 +136/71/1.15 · 24 +643/88/1.57 · 25 +432/37/2.13 |
| relweak_pm1115 | +2410 | 331 | 1.64 | -269 | -168 | 0.002 | 1.40 | 22 +1348/124/2.19 · 23 -8/75/0.99 · 24 +652/92/1.56 · 25 +418/40/2.01 |

halves (train):
- base__row25: 22h1 +837/72 · 22h2 +606/47 · 23h1 -84/31 · 23h2 +221/40 · 24h1 +167/41 · 24h2 +482/48 · 25h1 +380/41
- pdl15: 22h1 +585/60 · 22h2 +566/38 · 23h1 +20/26 · 23h2 +309/34 · 24h1 +237/34 · 24h2 +486/36 · 25h1 +430/28
- peersgreen: 22h1 +489/46 · 22h2 +480/22 · 23h1 +186/12 · 23h2 +25/10 · 24h1 +144/14 · 24h2 +210/16 · 25h1 +489/20
- peersgreen_am: 22h1 +811/60 · 22h2 +377/34 · 23h1 +15/19 · 23h2 +180/27 · 24h1 +255/25 · 24h2 +359/29 · 25h1 +589/30
- peersnotall_am: 22h1 +932/66 · 22h2 +466/43 · 23h1 -74/26 · 23h2 +197/29 · 24h1 +262/35 · 24h2 +440/37 · 25h1 +548/37
- peersnotall_thr: 22h1 +874/68 · 22h2 +586/46 · 23h1 -70/26 · 23h2 +169/30 · 24h1 +262/35 · 24h2 +481/39 · 25h1 +538/38
- pm1115: 22h1 +850/74 · 22h2 +555/52 · 23h1 -118/32 · 23h2 +110/43 · 24h1 +175/44 · 24h2 +539/51 · 25h1 +367/44
- relweak: 22h1 +837/71 · 22h2 +549/46 · 23h1 -84/31 · 23h2 +221/40 · 24h1 +167/41 · 24h2 +475/47 · 25h1 +432/37
- relweak_pm1115: 22h1 +850/73 · 22h2 +498/51 · 23h1 -118/32 · 23h2 +110/43 · 24h1 +140/43 · 24h2 +512/49 · 25h1 +418/40

Paired day bootstrap vs `base/row25` on train (A − B; P(diff ≤ 0); daily corr):

| cell | P&L diff | P(diff ≤ 0) | corr | removed (P&L / n / PF) | added (P&L / n / PF) |
|---|---:|---:|---:|---|---|
| peersnotall_thr | +229 | 0.169 | 0.96 | +98 / 57 / 1.13 | +327 / 19 / 2.97 |
| peersnotall_am | +162 | 0.279 | 0.95 | +219 / 70 / 1.24 | +384 / 23 / 2.71 |
| pdl15 | +24 | 0.469 | 0.86 | −117 / 69 / 0.91 | −96 / 5 / 0.00 |
| relweak | −13 | 0.541 | 1.00 | +2 / 8 | −13 / 1 |
| peersgreen_am | −25 | 0.528 | 0.86 | +472 / 124 / 1.29 | +443 / 28 / 3.39 |
| pm1115 | −132 | 0.806 | 0.98 | — | −132 / 20 / 0.59 |
| relweak_pm1115 | −200 | 0.892 | 0.98 | +2 / 8 | −200 / 19 / 0.41 |
| peersgreen | −588 | 0.847 | 0.71 | +991 / 195 / 1.42 | +401 / 15 / 4.78 |

**No cell reaches the T bar, P(diff ≤ 0) < 0.05 against row 25.** The best is 0.169.

## 5. reading

**Clocks are flat or worse.**
- On the screen, `cut1010` and `st1000` are trade-for-trade identical to row 25, so those clocks never bind.
- `cut0950` adds 2 trades at −33.
- `am0935` moves 5 early-thrust winners (+248) into 3 later entries (+144). The first five minutes stay in.
- `st1300` drops 7 stress trades worth +24.
- Extending the pm window to 11:15 looked good on the screen (+42 / 6) but loses on train (added −132 / 20 / PF 0.59, 2023 −144).
  At 11:30 it is already negative on the screen. The 10:00–11:00 block does not extend: HISTORY D3 now reaches back to 11:00.
- Per-window 1h dials (`t1h_am15`, `t1h_pm15`) both cost P&L on the screen. In the am, −0.15 removes 11 thrusts worth +230 (NVDA +234).
- `pmv18` adds 4 pm trades at PF 0.56, the same sub-floor population as HISTORY D5.

**The prior-day-low dial is not monotone.** On the screen:
- ≥ −2.5 cuts 12 crash-day shorts worth +252 (NVDA 2025-01-27, 2025-04-04);
- ≥ −1.5 also cuts the 14 trades between −1.5 and −2.5 %, worth −336;
- frontier's ≥ −1.0 cut trades at PF 1.23.

So the signs alternate along the dial: −1.0…−1.5 positive, −1.5…−2.5 negative, below −2.5 positive. There is no smooth
"far below yesterday's low is exhausted" mechanism. **`pdl15` on train** is +2,634 / 256 / PF 2.16, ex-2022 PF 1.95,
dd −174, worst day −95, every half positive (23h1 +20 against the base's −84). P&L is the same as row 25 (+24, P 0.47).
What it removes is 2022 winners (+279 / 23) and post-2022 losers (2023 −202 / 12, 2024 −143 / 21, 2025h1 −50 / 13),
including 5 ScoreExit trades worth −416. It is a real post-2022 quality lever, but its cut point is chosen from three
non-monotone points, and at ~6 trades a month it misses the T target-window bar of 150 trades (≈ 135 expected).

**Quality metadata mostly removes winners.**
- `rvol10` cuts +379 / 40 (AMZN +295) and `rvolcap` cuts +322 / 10. Climax bars are the winners.
- `vwstretch` cuts +281 / 23 (NVDA +255); stretched shorts keep running.
- `vwbelow` (+11) and `redday` (−40) barely bind: thrust shorts are already below VWAP and below the open.
- `relweak` removes 8 trades for −13 on train. Neutral, because the 5m / 1h scores already encode relative weakness.
- `spy15` does nothing (+0 / 7 removed).

**Peers: the one lever with a P&L gain, and its mechanism is fragile.**
- On the screen, `peersred` (≥ 2 of 3 peers red) removes the best trades (+573 / 27). Its complement `peersgreen`
  splits by window: am thrusts with red peers lose (−111 / 28) and pm thrusts with red peers win (+235 / 17).
- Restricted to the am windows, the coarse cut (`peersgreen_am`, ≤ 1 peer red) is PF 2.27 on 224 trades but P&L −25.
- The neighbour cut "not all three peers red" (`peersnotall_am`, `peersnotall_thr` on the am thrust window only) is
  +162 / +229 on train with PF 1.98–1.99.
- `peersnotall_thr` was not screened. It went straight to train as a narrowing of `peersnotall_am`, because the screen
  diff showed the am strong-core removals were winners (+99 / 5). Count it as one more cell.
- The trade diff (§6) shows the gain does **not** come from the filter's direct cut.

**Selection caveat for the peer cells.** The idea and its form came from the 2024h2_25h1 screen. On the train years outside
the screen window, `peersnotall_thr` vs row 25 is 2022 +17, 2023 −38, 2024h1 +95, so ≈ +74 over 2.5 years. Most of the
+229 sits in the window it was chosen on (24h2 −1, 25h1 +158).

## 6. trade diffs for the confirmed cells (train, vs base/row25)

- **peersnotall_thr**: kept 263, removed 57 am thrusts at +98 / PF 1.13, added 19 at +327 / PF 2.97.
  - Removed, by year: 22 +201, 23 +133, 24 −78, 25h1 −158. By ticker: AAPL −98 / 7, AMZN −123 / 12, MSFT +185 / 15, NVDA +135 / 23.
    By exit: ScoreExit −372 / 6, BreakevenStop −89 / 17, MaxHold +560 / 34.
  - **All 19 added trades are same-day, same-ticker re-entries** after the blocked thrust, via a later am thrust (+242 / 12)
    or the pm thrust (+95 / 6). NVDA +321 / 11; 2022 +217 / 6, 2023 +95 / 5, 2024 +15 / 8.
  - So the cell works as a **delay** on all-peers-red mornings, not as a filter. HISTORY D4 says delayed entries lose in
    general, and 19 trades is a thin basis for the exception.
- **peersnotall_am**: the same 57, plus 13 am strong-core trades worth +121 removed. Added +384 / 23, all re-entries.
- **peersgreen_am**: removed 124 trades at +472 / PF 1.29 (AMZN −324, AAPL −185, MSFT +331, NVDA +650; 2025h1 −209 / 11),
  added 28 re-entries at +443 / PF 3.39.
- **pdl15**: removed 69 at −117 / PF 0.91 and added 5 at −96, all re-entries. Removed, by window: am thrust +53 / 39,
  pm thrust −33 / 20, am strong core −178 / 6, pm core +42 / 4. By ticker: AAPL −103, AMZN −106, MSFT +238, NVDA −145.
  By year: 22 +279, 23 −202, 24 −143, 25h1 −50.
- **pm1115**: added 20 pm trades at −132 / PF 0.59 (2023 −144 / 4; MaxHold −258 / 10, SessionClose +193 / 7). Nothing removed.
- **relweak**: ±0. **relweak_pm1115**: equals pm1115 plus relweak.
- **peersgreen**: removed 195 trades at +991 / PF 1.42. Too much volume lost.

## 7. dead ends

- Window clocks: cut-over 09:50 / 10:10, pm end 11:15 / 11:30, am start 09:35, stress clock 10:00 / 13:00.
- Per-window 1h dials; pm-only VPIN 0.18.
- `relative_volume` floor or cap; VWAP stretch or below-VWAP; red-on-the-day; SPY 15m; relative weakness vs SPY.
- Peers-red as a confirmation.
- The pdl dial as a smooth lever: its sign alternates along the dial.
- No cell in this theme adds trades at PF ≥ 1.3 (the A test). Every added population was ≤ 20 trades or lost money.

## 8. engine proposals (not implemented)

1. **Multiplicative sizing tiers on a second key**, e.g. ×0.5 when `pdl_5m.dist_low_pct` < −1.5. This turns the prior-day-low
   clause into a real size dial and keeps the 2022 crash winners at reduced size. Today one `indicator_tiered` action sizes on
   one key, and I did not verify how two sizing actions combine.
2. **Peer-condition parity check before any peer cell ships.** The backtest serves peers at the same minute timestamp, while
   live `cross_tracker` serves each peer's latest bar. The coordinator should run `peersnotall_thr` with `--cross-lag 1`
   before trusting it; the gain comes from minute-level re-entry timing, which is exactly what a one-bar lag can move.
3. **A per-window "re-arm" flag.** It would test the delay mechanism directly: block the first thrust on a condition and allow
   the next one.

## 9. nominations (best of 25 r2-entries cells / 33 runs; programme ledger at writing 352 lines)

1. **`research/lab/specs/r2/entries/peersnotall_thr.json`**: row 25 plus `cross_1m.peers_red_frac ≤ 0.7` on
   `window_5m_thrust_short_am` (no am thrust when all three other names are red on the session).
   - **Targets test T. It fails T's train paired test as it stands** (P 0.169 against < 0.05), so the coordinator should
     treat it as a near-miss.
   - Train: +2,839 / 282 / PF 1.98, dd −218, worst day −119, sign-flip p 0.000, ex-2022 PF 1.72.
     Years: 22 +1460 · 23 +98 · 24 +743 · 25h1 +538.
     Halves: 22h1 +874 · 22h2 +586 · 23h1 −70 · 23h2 +169 · 24h1 +262 · 24h2 +481 · 25h1 +538.
   - Paired vs row 25: +229, P 0.169, corr 0.96.
   - Why: it is the only cell with a P&L gain over row 25 and no volume loss below 250.
   - Against it: the gain is 19 same-day re-entries, and ≈ ⅔ of it sits in the screen window. Run the `--cross-lag 1` check first.
   - **Holdout expectation (before reveal):** within ±100 of row 25's holdout P&L, PF 1.9–2.1, ~105 trades. P(diff ≤ 0) vs
     row 25 ≈ 0.3–0.5; I expect it to pass T's "not worse" clause (< 0.25) only about one time in three.
2. **`research/lab/specs/r2/entries/pdl15.json`**: row 25 plus `pdl_5m.dist_low_pct ≥ −1.5` on the four am/pm windows.
   - **Targets neither T nor A cleanly.** It is the quality alternative: same P&L, much better PF and drawdown. It fails T's
     paired test by construction (P 0.47) and is at risk on the 150-trade target bar.
   - Train: +2,634 / 256 / PF 2.16, dd −174, worst day −95, p 0.000, ex-2022 PF 1.95. Every half positive (23h1 +20).
     Years: 22 +1151 · 23 +330 · 24 +723 · 25h1 +430.
   - Why: the one lever that makes 2023 a good year (PF 1.48, base 1.15).
   - Against it: chosen from a non-monotone three-point dial.
   - **Holdout expectation:** PF 2.0–2.5 on ~85–95 trades, P&L −150…+50 vs row 25.
3. No third nomination; no A cell exists from this desk.
   The untested interaction is `pdl15` + `peersnotall_thr`, whose removed sets barely overlap. It is the coordinator's call
   whether it is worth one more ledger line.
