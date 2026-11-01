# desk: tom-event — calendar-event standalone books (2026-11-01)

**verdict: no nomination recommended.** The turn-of-month "session +2 long" reproduces in the engine (with costs, the 2.5 %
stop, whole shares) almost exactly as the pretest found it. But on the train window it is the single best of 25
session-of-month slots, the spread across slots looks like pure noise, and the required shifted-calendar twin (k=5)
is **not** flat: it earns +24.5 bps/trade against k=2's +30.1. The basket's 1,400 trades are 42 independent days, one bet
per month. In the engine the k=2 effect cannot be told apart from the session-of-month noise floor. Optional specs for closing RQ-11
are listed at the end, with a warning that the holdout is not clean for this effect.

All numbers come from the lab on `train` (2022-01-03 → 2025-06-30, 874 sessions; k=2 fires on 42 of them). No holdout run was made, and
nothing after 2025-06-30 was read. `LAB_AGENT=tom-event`.

## setup

- generator: `research/lab/patches/tom-event/gen.py` writes every patch and spec. The calendar is the k-th NYSE session of each month,
  taken from `data/bars_iex/SPY.csv` sessions (the lab's own list); k<0 counts from the month end. Its k=2 dates are identical to
  `research/tom/tom_long.patch.json`.
- patch (on row 12): adds `event_calendar` `tom_cal` (weight 0) and one `entry_window` "tom long"
  (`indicator_max tom_cal ≤ 0`, entry 09:30–09:35, `exit_overrides` force_exit_by 15:55, max_hold 6.5 h, score exit −2 = never).
  It disables both short windows, both 1m reject gates, `breakeven` and `trailing_stop_atr`, and keeps `hard_stop` at 2.5 %.
  `exit_session` is set explicitly to 15:55. The session block sets no_new_entries_after = window end, force_exit_by = exit.
  The 5-minute entry window is the one-entry-per-day guard, since the engine has no per-day entry limiter. A stop-out can only re-enter inside
  those 5 minutes (it never happened).
- **session / sizing, set deliberately:** replay runs tickers independently, so the live `max_concurrent_positions` cap is
  not simulated. A basket that enters every name at 09:31 would be truncated to 3 names live. Every spec therefore sets
  `max_concurrent_positions = number of names`, `max_capital_deployed_pct 0.95`, and per-name sizing of `0.90 / N` of the book
  (`--sizing-fraction` = `--max-position-pct`). Values used: screen = 41 names at 2 % of a 100k book (so every name gets whole shares;
  AMZN pre-split still skips 6 days); `b_idx3_k2` = 0.30 × 3 on 10k; `b_pool34_k2` = 0.0265 × 34 on a **100k** book (divide its
  dollars by 10 for a 10k book; on 10k, names above ~$265 would round to zero shares).
- runner (`research/lab/patches/tom-event/evrun.py`; subsets via `subsets.py`): for speed, cells were run with a thin wrapper around `lab.py` (`run_spec` on only the calendar-flagged days, then
  `summarize` over the full window's session list and a normal ledger line, note tagged `[event-days-only runner]`). Every other
  day is provably empty because the only enabled entry window requires the flag. Parity check: `b_idx3_k2` on `2025h1`
  through `lab.py run` (all 121 days) and through the wrapper gives identical output (+484 / 18 / PF 34.65, p 0.014). The per-day cache
  keys are lab.py's own, so the coordinator can re-run any spec with `lab.py run` for free on the event days.

## cells (train)

Screen = 41 names (SPY QQQ IWM, XLK XLF XLE SMH, the 34-name pool), 2 %/name on 100k. "bps" = net bps per trade after house
costs. Subsets are read from the same trades file and count as cells for multiplicity. p = sign-flip on the subset's daily
equal-weight basket return (42 days).

| cell | spec | ALL41 | POOL34 | MEGA4 | IDX3 | SPY | QQQ |
|---|---|---|---|---|---|---|---|
| k=2 open→15:55, stop 2.5 % | `s_k2_open_close` | +30.1 bps / 1714 / PF 1.55, p 0.038 | +32.0 / 1420 / 1.55, p 0.034 | +39.8 / 162 / 1.82, p 0.031 | +24.2 / 126 / 1.75, p 0.075 | +18.5 / 42 / 1.74, p 0.09 | +30.7 / 42 / 2.03, p 0.053 |
| k=2, no stop (20 %) | `s_k2_open_close_nostop` | +35.1 / 1714 / 1.68, p 0.030 | +37.7 / 1420 / 1.69, p 0.027 | +41.2 / 162 / 1.87 | +24.0 / 126 / 1.74 | +18.5 | +29.9 |
| k=2, gap ≤ 0 only | `s_k2_gapdown` | +35.9 / 913 / 1.71, p 0.049 | +38.9 / 756 / 1.73 | +63.9 / 88 / 2.76 | +24.2 / 72 / 1.77 | +13.2 / 25 | +45.6 / 24 |
| k=1 (first session) | `s_k1_open_close` | −8.8 / 1668 / 0.88 | −7.6 | −4.6 | −11.6 | −9.5 | −5.0 |
| k=3 | `s_k3_open_close` | −4.1 / 1673 / 0.94 | −2.9 | +5.8 | −6.6 | −5.4 | +1.7 |
| last session (k=−1) | `s_klast_open_close` | +11.8 / 1670 / 1.19, p 0.39 | +12.4 | +0.9 | +6.7 | +9.2 | +9.6 |
| **control k=5** (shifted twin) | `s_k5_ctrl` | **+24.5 / 1711 / 1.44, p 0.077** | +24.8 / 1.42 | +27.0 / 1.47 | +14.6 / 1.41 | +14.8 / 1.50 | +17.9 / 1.45 |
| control k=10 (shifted twin) | `s_k10_ctrl` | −14.1 / 1713 / 0.80, p 0.83 | −12.7 | −21.4 | −23.6 | −21.3 | −18.8 |

Per year, ALL41 bps (22 / 23 / 24 / 25h1): k=2 +58.5 / −13.0 / +16.9 / +86.9; k=5 +12.7 / +34.4 / +15.3 / +46.1;
k=−1 +15.1 / +22.2 / −28.5 / +58.8.

Standalone book specs (train):

| spec | P&L / trades / PF | dd | worst day | sign-flip p | ex-best-year | per year 22 / 23 / 24 / 25h1 | top-name share |
|---|---|---|---|---|---|---|---|
| `b_idx3_k2` (10k) | +872 / 126 / 1.75 | −344 | −208 | 0.071 | PF 1.34 ex-2025 | +386 / −63 / +65 / +484 (18 trades, 17 winners) | QQQ 42 % |
| `b_pool34_k2` (100k; ÷10 for 10k) | +11,784 / 1,422 / 1.57 (≈ +1,178 on 10k) | −3,494 (≈ −349) | −1,834 (≈ −183) | 0.032 | PF 1.38 ex-2022 | +6,588 / −1,204 / +1,947 / +4,453 | COIN 14 % |

Exits for `b_pool34_k2`: 177 hard stops −11.8k, 1,245 session closes +23.6k. The stop costs ~5 bps per trade (compare the no-stop cell).

## reading

1. **The engine reproduces the pretest.** SPY +18.5 bps on 42 train days (pretest 5 y: +17.7), QQQ +30.7 (+23.9), mega-caps
   +39.8 (+37.7), pool +32.0 (+30.9). Plumbing, costs and the stop do not change the story.
2. **It is one bet per month.** The basket's daily return is SPY's day plus noise: the top 3 of 42 days carry 62 % of the
   ALL41 bps (2022-06-02 +310, 2022-07-05 +254, 2025-04-02 +228). Without them the mean is +12 bps/day. 28 of 42 days are green.
   Basket breadth adds trades but no independent observations, so the sign-flip p of 0.03–0.08 is the honest strength, and that
   is before multiplicity.
3. **The session-of-month profile is noise.** A bars-only profile (09:31 open → 15:56 open, minus 6 bps + $0.01/share, the same window
   and costs as the engine, no new ledger cells, `research/lab/patches/tom-event/kprofile.py`) over all 25 slots (k=1…20, k=−1…−5), ALL41,
   train: k=2 has z = +1.92 against the all-session mean of −3.5 bps, the **maximum of 25 slots**. k=4 is −35.8 (z −1.67), k=9 +27.4
   (z +1.60), k=5 +24.0 (z +1.42). The SD of z across slots is 0.86 (1.0 = pure noise), and 1 of 25 slots has |z| ≥ 1.8. The expected maximum
   of 25 standard normals is ≈ 1.97. The random one-session-per-month control gives p = 0.027 for ALL41 k=2 (SPY 0.061, QQQ
   0.037, MEGA4 0.032). That p is a single-slot p, and k=2 was picked as the best slot ("round 1b day +2 discovery").
4. **The required control fails.** The k=5 twin with the identical window is clearly positive (+24.5 bps, PF 1.44, p 0.077). The k=10
   twin is negative (−14.1). Neither looks like "flat around a k=2 bump". The neighbours k=1 (−8.8) and k=3 (−4.1) are negative,
   so there is no turn-of-month *window*, only an isolated day. That is the signature of a noise maximum, not of the
   Ariel / Lakonishok–Smidt effect, which is close-to-close over d−1…d+3 and mostly overnight (pretest c02: intraday d−1, d+1..3
   PF 1.22, a multi-day product).
5. **Mechanism variants are tuning on that maximum.** No-stop (+35 bps) and gap-down-only (+36 bps, half the trades, QQQ n = 24) look
   better, but they slice the same 42 days, so I did not pursue them. The planned entry-time (09:45 / 10:30), exit (12:00 / 11:55)
   and trail cells were deliberately **not run**: once the calendar slot itself is indistinguishable from noise, every timing cell only adds
   to the programme's denominator. (Specs were deleted before running; they are not in the ledger.)
6. **Other calendar effects:** pretest c01 pre-FOMC / FOMC-day is dead (09:30→14:00 SPY −2.5 bps, PF 0.77). OpEx is dead. Last-session
   (k=−1) is +11.8 bps, PF 1.19, p 0.39 in-engine and z +0.87 in the profile, so it is not live. No FOMC-afternoon cell was run: the pretest
   shows no live signal to justify it.
7. Overlap with v18: v18 had 16 trades, +89, on the 42 k=2 days in train, so the books would barely interact.

Cells: 10 distinct specs / 12 ledger runs (8 screen + 2 book specs, with one 2025h1 parity re-run each through the wrapper and lab.py),
each read on 7 subsets. Add the 25-slot bars-only profile.

## nominations

**None recommended as a strong contender.**
- The train evidence for k=2 is best-of-25 at z 1.9, and the shifted-calendar twin fails.
- Plan test 1 (≥ 150 trades on the target) is out of reach for any index-only form: ≈ 66 trades over 22 months for SPY/QQQ/IWM.
- A pool form reaches the trade count only by multiplying one daily bet by 34.
- **The holdout is not clean for this effect.** The pretest cells c11 and c15 that selected k=2 were computed on 2022-01 → 2026-10, so the
  holdout months already informed the choice of k=2. HISTORY §B10 quotes their 2025/2026 rows. A reveal would partly re-measure
  data that picked the rule.

If the coordinator still wants RQ-11 closed in-engine, use these optional specs (hosting-ready):

| spec | what | holdout expectation (written before any reveal) |
|---|---|---|
| `research/lab/specs/tom-event/b_idx3_k2.json` | SPY/QQQ/IWM long, session 2, 09:31 open → 15:56, stop 2.5 %, 0.30 each on 10k | ≈ 48 trades. Honest prior: unconditional open→close drift (−5…+5 bps), PF 0.8–1.2. Because the pretest already saw these months positive, a PF 1.2–1.8 print would **not** be evidence. It cannot pass test 1 (trade count) by construction. |
| `research/lab/specs/tom-event/b_pool34_k2.json` | the 34-name pool, same rule, 0.0265 each on a 100k book (÷10 for 10k dollars) | ≈ 540 trades on ≈ 16 days. Same prior. Day-level p on 16 days will be > 0.1 whatever happens. Expect worst-day drawdown ≈ −1.5…−2 % of the book on a red k=2 day (−183 on 10k in train). |

Hosting (either spec): a `books` row with its own tickers (IDX3: SPY QQQ IWM; pool: the 34 names) and a new
`config_versions` row materialised from the patch on row 12.
- Before hosting, write `sizing_fixed.fraction` and `session.max_position_pct` (0.30 or 0.0265) into the patch; the lab passes them
  as CLI flags. Set `max_concurrent_positions` = N, which the patch already does.
- Warm-up: nothing in the rule needs history (`event_calendar` and `fixed_pct_stop` are stateless). The default 8 days is fine,
  and the book adds no SPY warm-up demand.
- The calendar list ends 2026-12; `gen.py` needs `FUTURE_HOLIDAYS` extended before then.
- IWM's IEX cache ends 2026-10-13, so top it up if replaying later dates.
- The book trades ≈ 12 sessions a year and is idle otherwise, so shadow verdicts would take years.

## engine notes (proposals only, nothing implemented)

- A per-window `max_entries_per_day` (or one-shot) flag would replace the 5-minute entry-window hack for event books.
- If event books are ever pursued, the real turn-of-month effect is close-to-close across d−1…d+3, which needs an overnight-hold product.
  That is outside this engine's intraday-only constraint.
