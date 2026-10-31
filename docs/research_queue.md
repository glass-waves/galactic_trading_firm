# research queue

proposals from the evening routine; results written back by `scripts/research_runner.sh` on the
desktop. the human approves an entry by changing `status: proposed` to `status: approved`.
format and rules: `docs/routines/eod-report.md` §6. append only. `status:` must sit on the `### RQ-N` header line;
`status: approved from YYYY-MM-DD` runs on the first nightly run on or after that date (one entry per night keeps
the 20:00 runner clear of the 20:30 pipeline on the shared lock).

### RQ-1 plateau cell: SPY ±0.3 % / VPIN ≥ 0.18 as v19 candidate   status: done
proposed: 2026-09-21 by human (from plan doc §11.6 / §13.5)
hypothesis: widening the two v17 filters to the validated plateau cell raises trade count ~39 % while keeping PF > 1.3 and every year positive.
why now: v18 live rate 2 trades in 8 sessions vs 0.4/day expected; the grid was pre-computed.
command: (already run as tag e_grid_b0.6_v0.18)
accept if: PF ≥ 1.3 pooled, positive in 5 of 5 years.
result: 652 trades / +3,068 / PF 1.47; 2022 +1,667 · 2023 +138 · 2024 +252 · 2025 +314 · 2026 +697. passes. held as the fallback if the week of 09-22 ends without trades.

### RQ-2 runner smoke test: single-day replay   status: done
proposed: 2026-09-23 by human
hypothesis: the research runner executes an approved command and writes the result back.
why now: first wiring of the wheel.
command: ./target/release/backtest --date 2026-09-23 --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars --cross-index SPY | grep -c '^trade,'
accept if: result shows the trade count for 2026-09-23 (expected 1).
result: ran 2026-09-23 20:28 PT, 0 min
```
1
```
stderr: DATA_QUALITY:AMZN:2730:8
DATA_QUALITY:AAPL:2730:8
DATA_QUALITY:NVDA:2730:8
DATA_QUALITY:MSFT:2730:8

### RQ-3 VPIN on IEX volume: does the v17 filter survive the live feed?   status: done
proposed: 2026-09-24 by eod-routine
hypothesis: recomputing the five-year v18 replay with VPIN fed IEX-volume bars (what live actually sees) keeps PF ≥ 1.4 with the 0.217 floor, or finds a floor that does; if not, the live VPIN filter is not the one that was validated.
why now: 2026-09-24 live vs replay mismatch — AMZN live raw_vpin 0.07–0.08 vs replay 0.21–0.23 at 09:30–09:35, MSFT live passed VPIN at 09:35 while replay stayed 0.04–0.18 until 09:49. live stream is IEX (`alpaca_feed.rs:120`), warmup and bar cache are SIP.
command: needs code — add `--feed iex|sip` to `backtest --fetch-bars` (pass `apca::data::v2::Feed::IEX` to the bar request, write to a separate dir, e.g. `data/bars_iex`), and a `--vpin-bars-dir DIR` option so VPIN (only) reads volume/closes from that cache while prices, fills and every other indicator keep using SIP `--bars-dir`. then run the v18 honest five-year replay (`data/e_v17_lag1.args` with the v18 cap) twice: VPIN on SIP (baseline, must reproduce ~469 trades / PF 1.64) and VPIN on IEX; also a raw_vpin floor sweep 0.12–0.30 on IEX. a parallel live fix (warm up on IEX so the window is one feed) is a separate small PR the human can choose.
accept if: IEX-VPIN run at some floor gives PF ≥ 1.4 pooled, ≥ 350 trades, positive in ≥ 4 of 5 years — then that floor with an IEX-consistent live warmup is the v19 candidate; otherwise flag the VPIN filter as unvalidated live.
result: ran 2026-09-24 evening (full IEX cache `data/bars_iex`, whole replay on IEX not just VPIN; plan doc §14.1). v18 cell on IEX: 407 / +2,154 / PF 1.48 / 3 pos years. VPIN 0.26 at ±0.2 %: 306 / +1,952 / PF 1.58 / 4 pos years (2023 −97) — the only cell meeting PF ≥ 1.4 with ≥ 4 years, but 306 < 350 trades. no-VPIN rows PF 1.20, so the filter is real on IEX too. verdict: partially accepted — filter validated, floor 0.217 kept, wider cells rejected; live warm-up moved to IEX (4d81b14).

### RQ-4 Tradier as the live bar feed (parked 2026-10-04; re-prioritised 2026-10-07 after §16.1: SIP replay PF 1.56 / 510 trades vs IEX 1.37 / 436)   status: parked
note: parked by the owner (needs a sandbox token); RQ-9 answers the feed question from cached data first.
why: Tradier docs say its equities feed is consolidated (all exchanges) at $10/mo Pro vs Alpaca's $99 SIP plan; would fix the VPIN volume gap on the free IEX stream. real-time is gated to a brokerage account; the websocket is per-trade ticks (adapter must bucket minutes); REST 1m history only 10–20 days (Alpaca stays the history source). research: docs/research/2026-10-04_tradier_data_quality.md.
command: put a free sandbox token in .env as TRADIER_TOKEN (TRADIER_BASE=https://sandbox.tradier.com), then `set -a; source .env; set +a; python3 scripts/analysis/tradier_volume_check.py 2026-10-02` — the decisive test, 10 minutes.
accept if: Tradier/SIP volume ratio ≈ 1.0 (± 0.05) with minute correlation > 0.95 on all five symbols → then scope a tradier_feed.rs adapter (2–3 days) and run it as a parallel feed for a week before anything depends on it. ratio well below 1 → close.

### RQ-5 scale-free MACD (found 2026-10-14 by the split-adjustment restatement)   status: superseded by RQ-8
why: `macd_5min` and `macd_fast_1min` score the MACD histogram in DOLLARS (clamped to ±1), so the composite's sensitivity depends on a name's price level — NVDA at $120 vs $1,200 scores differently for the same percentage move, and so do AAPL ($335) vs MSFT ($520). a hidden per-name parameter; also why a plain replay of split-adjusted history moves ~80 trades (docs/research/2026-10-14_split_adjustment.md).
command: add a `normalize_by_price: true` option to the MACD indicator (histogram ÷ close, scaled so the ±1 clamp binds at the same percentage move), then propose a config candidate with both MACD instances switched on, gate quality-config — the pipeline judges it like any other change.
accept if: PF ≥ baseline + 0.05 with ≥ 4 of 5 years and ≥ 0.6 × baseline trades (quality-config), or trades ≥ baseline with PF ≥ 1.3 (volume-config); either way it must also be positive in every year on the point-in-time record.

### RQ-6 spend the sqrt-band books' PF headroom on volume   status: done — gate pass, but noise-sized vs its parent (see verdict)
proposed: 2026-10-23 by claude (weekly audit; owner goal "increase volume while keeping PF > 1.4")
hypothesis: lowering the VPIN floor on the two time-consistent SPY-band candidates (#44 row 20 PF 1.46 / 460, #45 row 21 PF 1.70 / 309) buys ≥ 480 trades at PF ≥ 1.40.
why now: the VPIN-floor × SPY-band grid was only ever run on thrust-1h15 (research/qualvol) and on v18 before the VWAP parity fix; on thrust-1h15, 0.217 → 0.15 cost ≈ 0.14 PF for +112 trades — a prior, not a result. conditions/6 of the thrust windows and conditions/5 of the core windows are the `vpin_1m.raw_vpin` min_score (checked against rows 20/21 on 2026-10-23).
cost: 7 five-year sweeps ≈ 30 min.
command: for c in "20 0.20" "20 0.18" "20 0.15" "21 0.217" "21 0.20" "21 0.18" "21 0.15"; do set -- $c; BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq6_r$1_v$2 --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --config-id $1 --set-action-param window_5m_thrust_short_am:conditions/6/min_score=$2 --set-action-param window_strong_core_short_am:conditions/5/min_score=$2 --set-action-param window_5m_thrust_short_pm:conditions/6/min_score=$2 --set-action-param window_strong_core_short_pm:conditions/5/min_score=$2; done; python3 research/entries/summarize.py cand_44 cand_45 rq6_r20_v0.20 rq6_r20_v0.18 rq6_r20_v0.15 rq6_r21_v0.217 rq6_r21_v0.20 rq6_r21_v0.18 rq6_r21_v0.15
accept if: `rq6_r21_v0.217` reproduces `cand_45` (+2,044 / 309; otherwise the override did not apply — stop). a cell passes with PF ≥ 1.40, ≥ 480 trades, ≥ 4/5 years positive, no year < −300, and the same floor chosen by the volume-constrained leave-one-year-out check (`python3 research/timefilter/analyze.py loyo_vol <tags>`).
next: edit the four floors in the #44 or #45 patch (data/cand_44.args / cand_45.args name the patch files), then `python3 scripts/pipeline/pipeline.py propose --gate volume-config --cells-scanned 24 --study research/timefilter/2026-10-07_time_consistent_filter.md ...` — four shadow slots are free since MAX_BOOKS went to 12.
result: ran 2026-10-26 20:00 PT, 31 min
```
== rq6_r$1_v$2: 5y         —
   2022         —
   2023         —
   2024         —
   2025         —
   2026         —
```
verdict (2026-10-29, statistics audit): the cell clears the volume-config gate and is candidate #49 (shadow from 10-30), but a paired day-bootstrap against its parent cand_44 says the improvement is noise: P&L +149, 90 % CI [−291, +603], P(≤ 0) 0.30; PF −0.009 [−0.13, +0.11]. the two share 409 trades; RQ-6 adds 83 (+266, PF 1.33) and drops 49 (+117, PF 1.28), and the "every year positive" comes from 2024 (+355) at 2025's expense (−205). with cand_44 in the LOYO candidate set the pick is 3 of 5, not 4 of 5, and the out-of-sample sum (+2,107) is below cand_44's own. NVDA is 73 % of the P&L. read #49's shadow as a parity trial of a 492-trade variant, not as an edge upgrade; ex-2022 PF 1.28 for both.

### RQ-7 stress-core trigger neighbourhood (generated, never swept)   status: done
proposed: 2026-10-23 by claude (owner: "make sure we aren't tossing aside ideas too quickly")
hypothesis: a different trigger on the core-only stress window adds more than #6 stress-s15-core's ≈ +275 / 23 trades while leaving the base book untouched.
why now: research/stress/make_patches.py already writes {s10,s20,c10n15,c05n15,n15,n25}_core but only s15_core was ever swept (stress report §2); §4 says the 1h ≤ −0.4 requirement "arrives late".
cost: 9 five-year sweeps ≈ 35 min. the generated JSONs are pruned afterwards (repo hygiene) except the winner's.
command: python3 research/stress/make_patches.py >/dev/null && for c in am_only s15_core s10_core c10n15_core c05n15_core n15_core n25_core s20_core s10_core_novpin; do BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh st2_$c --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --no-new-entries-after 15:30 --force-exit-by 15:55 --patch-json research/stress/$c.json; done; python3 research/stress/analyze.py --grid st2_am_only st2_s15_core st2_s10_core st2_c10n15_core st2_c05n15_core st2_n15_core st2_n25_core st2_s20_core st2_s10_core_novpin
accept if: sanity first — st2_am_only = 434 trades and st2_s15_core = cand_6 (+2,003 / 459). a cell passes the additive-config clauses (base unchanged within ±2 % P&L / ±1 % trades; added ≥ 15 trades, net positive, PF ≥ 1.3, no year < −100, no day < −300, combined years not worse) AND added P&L ≥ #6's AND ≥ 35 added trades.
next: `python3 research/stress/make_patches.py --pipeline` for the winner, `pipeline.py propose --gate additive-config --cells-scanned 26`; the human decides whether #6 stays in shadow. phase 2 (agent edits the generator): core 1h ceiling at −0.3 / −0.2.
result: ran 2026-10-27 20:00 PT, 43 min
```
== st2_$c: 5y         —
   2022         —
   2023         —
   2024         —
   2025         —
   2026         —
```
### RQ-8 scale-free MACD, runnable form of RQ-5   status: done — invariance confirmed, strategy fails; phase B is the real test
proposed: 2026-10-23 by claude (RQ-5 needed code; the indicator already has a price-scaled mode — normalization_factor ≤ 0 divides the histogram by 1 % of the close, crates/indicators/src/native/macd.rs)
hypothesis: a price-scaled MACD makes the adjusted and point-in-time replays agree (today 360 vs 434 trades, plan §16.3) without losing PF.
why now: the fixed $1 norm is 0.2–0.9 % of price across the four names and years (cache medians: MSFT $266→$480, AAPL $153→$297), so the existing 1 %-of-price mode damps MACD 1.1–5×: treat it as an invariance test and a lower bracket, not the fair test. patch: research/macd/k01.patch.json (replaces both MACD instances in place — patches replace by instance_id since 2026-10-23).
cost: 2 five-year sweeps ≈ 8 min. patch verified 2026-10-23 on 10-22: scores match hist/(0.01·close) to 1e-4; mean |macd_5min| falls from 0.21 → 0.04 (MSFT), so this bracket mostly switches the MACD off — read it as the invariance test, phase B supplies the fair divisor.
command: BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq8_k01_pit --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/macd/k01.patch.json; BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq8_k01_adj --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/macd/k01.patch.json --splits none; python3 research/entries/summarize.py iex_v18 rq8_k01_pit rq8_k01_adj
accept if: invariance — pit and adj trade counts within 1 % of each other. as a strategy RQ-5's own clause: PF ≥ baseline + 0.05 with ≥ 4/5 years and ≥ 0.6 × baseline trades (quality-config) or trades ≥ baseline with PF ≥ 1.3 (volume-config), positive every year on the point-in-time record.
next: phase B is a code PR — a `normalization_pct` param so the clamp binds at the same percentage move (sweep k ∈ {0.003, 0.004, 0.005, 0.007}); phase C re-screens the universe near-misses (SHOP PF 1.30, AMD 1.28 / META 1.33 on 1h15, CVX 1.43 n < 60, CAT 1.26) whose dollar MACD made each a different strategy — their cooldowns run to 2027-03/04, re-proposing is the human's call.
result: ran 2026-10-29 20:00 PT, 9 min
```
== rq8_k01_pit: 5y     +421 n= 231 win=  35% PF=1.15 dd=  -838
   2022     +666 n=  62 win=  45% PF=1.86 dd=  -287
   2023     -363 n=  47 win=  26% PF=0.50 dd=  -471
   2024     -100 n=  51 win=  35% PF=0.82 dd=  -187
   2025     +136 n=  42 win=  36% PF=1.25 dd=  -198
   2026      +83 n=  29 win=  31% PF=1.31 dd=   -68
== rq8_k01_adj: 5y     +724 n= 233 win=  37% PF=1.25 dd=  -845
   2022     +895 n=  60 win=  50% PF=2.28 dd=  -364
   2023     -266 n=  50 win=  30% PF=0.66 dd=  -436
   2024     -124 n=  52 win=  35% PF=0.80 dd=  -210
   2025     +136 n=  42 win=  36% PF=1.25 dd=  -198
   2026      +83 n=  29 win=  31% PF=1.31 dd=   -68
```
verdict (2026-10-29): invariance holds — 231 vs 233 trades (0.9 %) on the point-in-time vs adjusted replays, so a price-scaled MACD removes the split sensitivity. as a strategy the 1 %-of-close bracket fails every clause (+421 / 231 / PF 1.15, 2023 −363): it mostly switches the MACD off, exactly as the 10-22 dump predicted. next = phase B, a `normalization_pct` knob so the ±1 clamp binds at the same percentage move as today's $1 does at ~$300 (k ≈ 0.003–0.007), then the same two-sweep invariance check plus the quality-config clause.

### RQ-9 feed gap: volume or price path?   status: done
proposed: 2026-10-23 by claude (the SIP-vs-IEX decision, answered from cached data before paying for anything)
hypothesis: most of the SIP-vs-IEX gap comes from the VPIN gate (volume), not from the price path.
why now: phase 1 is already computed from the iex_v18 / sip_v18_vwapfix trade CSVs (2026-10-23): the +1,006 gap splits into price path and fills +53 (182 identical entries), timing +342 (337 shared ticker-days, re-timed entries, median −1 min), selection +611 (SIP-only ticker-days +424 vs IEX-only −187). ≈ 95 % of the gap is which bar or day passes the gates.
cost: 2 dump sweeps ≈ 15 min (delete data/rq9_*_ticks.csv afterwards), then ≈ 1 h of agent analysis.
command: DUMP_TICKS=1 BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq9_iex --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --dump-window-only; DUMP_TICKS=1 BARS_DIR=data/bars scripts/run_cached_sweep.sh rq9_sip --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --dump-window-only
accept if: (pre-registered) for each non-shared entry find the condition that flipped on the other feed at that bar. "volume-driven" if bars where only VPIN flips carry ≥ 60 % of the +953 timing+selection gap; "price-driven" if SPY-band and score flips carry ≥ 60 % (`vwap_dist_1hr` is volume-weighted — tag it separately); otherwise mixed.
next: volume-driven → an IEX-native VPIN neighbourhood (`bucket_divisor` {0.2, 0.4, 0.8} × `lookback_buckets` {20, 40}, never swept) with floors set by matching v18's pass rate, before spending $99/mo. price-driven → nothing on IEX can fix it and the SIP decision rests on PF 1.57 / 510 vs 1.40 / 434.
result: ran 2026-10-30 20:00 PT, 10 min
```
== rq9_iex: 5y    +1829 n= 434 win=  39% PF=1.40 dd=  -469
   2022    +1089 n=  87 win=  45% PF=2.36 dd=  -291
   2023       -3 n=  91 win=  33% PF=1.00 dd=  -250
   2024      -17 n= 101 win=  39% PF=0.99 dd=  -331
   2025     +332 n=  80 win=  41% PF=1.41 dd=  -179
   2026     +429 n=  75 win=  39% PF=1.67 dd=  -124
== rq9_sip: 5y    +2835 n= 510 win=  38% PF=1.57 dd=  -369
   2022    +1259 n= 116 win=  46% PF=2.12 dd=  -335
   2023     +328 n= 104 win=  38% PF=1.32 dd=  -248
   2024     +113 n= 108 win=  32% PF=1.09 dd=  -369
   2025     +410 n=  91 win=  33% PF=1.51 dd=  -211
   2026     +726 n=  91 win=  40% PF=1.93 dd=  -154
--- command output ---
[rq9_iex] done: 260 days, 0 skipped
[rq9_iex] done: 260 days, 0 skipped
[rq9_iex] done: 262 days, 0 skipped
[rq9_iex] done: 261 days, 0 skipped
[rq9_iex] done: 181 days, 0 skipped
[rq9_sip] done: 260 days, 0 skipped
[rq9_sip] done: 260 days, 0 skipped
[rq9_sip] done: 262 days, 0 skipped
[rq9_sip] done: 261 days, 0 skipped
[rq9_sip] done: 181 days, 0 skipped
```
### RQ-10 pretest cell c14: VWAP-fade neighbourhood   status: in pretest (cell c14_vwap_fade_pm added 2026-10-23; first verdict fragile)
proposed: 2026-10-23 by claude
why: c12 is labelled dead, but several of its points are net-positive (pretest summary, c12 section: the 4 names at 12:30 +19.8 bps / PF 1.75 / 3 of 5 years, n 57; the 34-name pool at z 1.0 PF 1.46 / 3 of 5). under the README's own vocabulary that is sub-cost, not dead, and it is a fade mechanism, not the thrust trigger that "afternoons never paid" refers to.
method: c14 appended to research/pretest/cells.py (c12 untouched): t {150, 180, 210} min × hold {45, 60, 90} × z {0.75, 1.0, 1.25}, 4 names and the 34-name pool, same kill bar and random-session control (p < 0.05). ≈ 20 s inside the nightly pretest.
accept if: a neighbourhood (not a point) clears the pretest pass bar with p < 0.05.
first run 2026-10-23: fragile — best point pool / 12:30 / hold 45 / z 1.0: n 119, +16.6 bps net, PF 1.69, 4/5 years, z=0 control p 0.00; but only 2 of 54 cells clear the bar and leave-one-year-out gives 3/5, so it is a grid pick (without the LOYO rule it would read pass). the nightly pretest tracks it from here.
next: an engine study, which needs a normalized-stretch metadata key on vwap_distance (small code).

### RQ-11 turn-of-month day +2 long in the engine   status: proposed (needs the gate decision below)
proposed: 2026-10-23 by claude (backup decoupled channel if #48 qqq-noise-pm-vol ends insufficient_activity, which its trade rate predicts)
why: passed the pretest with the random-session control (SPY PF 1.77 5/5 p 0.03, QQQ PF 1.73 4/5 p 0.037) and has never been run in the engine.
method: research/tom/tom_long.patch.json — `event_calendar` date list (60 dates, session 2 of each month, research/tom/dates.py), a long entry_window 09:30–10:30 with exit_overrides (force_exit_by 15:55, max hold 6.5 h, score exit off), breakeven monitor, ATR trailing stop, both 1m-noise reject gates and both short windows disabled (the strategy stands alone; the 2.5 % hard stop stays), tickers SPY, QQQ, IWM (IEX cache). verified 2026-10-23: 3 trades on 2026-10-02 (session 2), none on 10-01 / 10-05. one five-year sweep; ≈ 170–180 trades expected, which only just clears the standalone gate's ≥ 150.
command: BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq11_tom --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/tom/tom_long.patch.json; python3 research/entries/summarize.py rq11_tom
accept if: net bps within 3 bps of the pretest, PF > 1.3, ≥ 4/5 years.
caveat (human decision before any proposal): the standalone-strategy gate's `combined_active_days ≥ 1.25 × base` needs ≈ 81 extra active days and this rule adds at most ≈ 58, so it fails by construction; either an `event-strategy` gate variant without that clause, or run it as a shadow ticker-set on its own.

### RQ-12 scale-free MACD phase B: `normalization_pct` sweep   status: approved from 2026-11-02
proposed: 2026-10-29 by claude (engine-numerics audit: on 2022-05-05 AMZN at $2,338 had macd_5min pinned at ±1 on 75 % of bars; pre-split AMZN/NVDA are 116 of 470 trades but 48 % of the e_v17_lag1 P&L, $10.7/trade vs $3.8 for the rest — the live regime looks like the $3.8 half)
hypothesis: scoring the 5m/1m histogram against a fraction of price (`normalization_pct`, added 2026-10-29, default off) keeps the record's edge at today's price levels, i.e. PF ≥ 1.4 ex-2022 is not reached but the ex-best-year PF does not fall below cand_44's 1.27, and the pit/adj replays agree within 1 %.
why now: RQ-8 showed the 1 %-of-close bracket switches the MACD off (|score| 0.21 → 0.04). at ~$300 the $1 norm equals 0.33 % of price, so the fair bracket is 0.3–0.7 %.
cost: 4 five-year sweeps ≈ 20 min. patch files research/macd/pct_<k>.patch.json replace both MACD instances in place (same ids, timescales, weights as row 12) with `normalization_pct` k; an agent writes them from k01.patch.json before the run (one-line edits).
command: for k in 0.003 0.004 0.005 0.007; do BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq12_pct$k --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/macd/pct_$k.patch.json; done; python3 research/entries/summarize.py iex_v18 rq12_pct0.003 rq12_pct0.004 rq12_pct0.005 rq12_pct0.007
accept if: the k closest to today's effective scale (0.0033) reproduces 2026's trade count within 15 % of iex_v18's 75 AND its 2022 share of P&L falls below 50 % AND ex-best-year PF ≥ 1.25; then the adj-vs-pit invariance check (RQ-8's two-sweep form) on that k.
next: propose the winner with gate quality-config and `--cells-scanned 4`; it is a candidate for the RQ-14 restatement baseline.

### RQ-13 a directional 1h sum: standard supertrend + signed ADX   status: approved from 2026-11-03
proposed: 2026-10-29 by claude (engine-numerics audit: `supertrend` never turns bearish — is_bullish = 1 on 100 % of 10.7k dumped bars incl. −2.8 % and −3.5 % days — and `adx` is scored direction-free, so 60 % of the 1h weight is a +0.125 offset; the short windows' `s1h ≤ 0` gate passes 40.5 % of bars where the directional part alone would pass 59.9 %)
hypothesis: with `supertrend.standard: true` and `adx.signed: true` (both added 2026-10-29, defaults unchanged) the 1h gate admits more genuine down-trends; trades rise toward 500 with PF ≥ 1.4 after the thresholds are re-fitted.
cost: 3 five-year sweeps ≈ 15 min (patch alone; then the 1h threshold neighbourhood).
command: BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq13_dir1h --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/dir1h/dir1h.patch.json; BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq13_dir1h_m04 --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/dir1h/dir1h.patch.json --set-action-param window_5m_thrust_short:conditions/3/max_score=-0.4; BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rq13_dir1h_p02 --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY --patch-json research/dir1h/dir1h.patch.json --set-action-param window_5m_thrust_short:conditions/3/max_score=0.2; python3 research/entries/summarize.py iex_v18 rq13_dir1h rq13_dir1h_m04 rq13_dir1h_p02
accept if: any cell with PF ≥ 1.40, trades ≥ 434, ex-best-year PF ≥ 1.25 and the paired day-bootstrap lower 90 % bound of the P&L difference vs iex_v18 > 0.
next: an agent writes research/dir1h/dir1h.patch.json before the run (replace the 1h supertrend and adx instances of row 12 in place with the two flags; verify on one day with --dump-ticks that is_bullish takes both values). the windows' `timescale_max OneHour` condition index (3 for the thrust window) must be checked against row 12 first.

### RQ-14 restatement bundle (human decision)   status: proposed
proposed: 2026-10-29 by claude
why: three parity/robustness fixes each move the record a little and must land together with a re-run baseline and a regate of every open candidate, not one at a time: (a) the backtest's max-hold clock starts at the fill bar, live at the signal bar (one bar late in every replay); (b) `warmup_days` 8 → 12 so the 1h ADX exists from the open after holiday weekends; (c) the VPIN sigma window straddles the overnight gap and the lookback carries yesterday's last 20 bars into 09:30–09:49 (the 0.217 gate is 30 % pass in the first 20 minutes and 6 % in the next 20 — it is largely a time-of-day filter; 70 % of trades enter in the first 20 minutes). (c) needs a `session_sigma: true` option in vpin.rs (not written yet) and a re-fit of 0.217.
method: one evening — fix (a) and (b) in code, add (c) behind a flag, rebuild, re-run `iex_v18` into data/, `pipeline.py regate` every shadow candidate against the new baseline, restate docs/pipeline/status.md and plan §16; parity check = `paper_trader --replay-bars` on a traded day must now match the live exit bars exactly.
accept if: the restated baseline is within ±5 % P&L of +1,829 / 434 (if not, the plan §16 record is rewritten with the explanation, as on 10-14); every open candidate keeps its gate verdict or the change is explained per candidate.
