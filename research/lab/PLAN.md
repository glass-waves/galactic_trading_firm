# lab: find a config with PF > 1.4 on 2025-01-01 → today (pre-registered plan, 2026-10-31)

## where we start

- promoted config: row 12 (`v18`): short-only morning windows (5m thrust / strong core) on AMZN AAPL NVDA MSFT,
  SPY cross filter, VPIN floor, flat by 11:55 ET. house costs, research sizing (0.36 / 0.36).
- its record on the IEX cache: 22 +1089 (PF 2.36) · 23 −3 · 24 −17 · 25 +332 (PF 1.41, 80 trades) · 26 +429 (PF 1.67, 75 trades to 09-10).
- live paper since promotion (09-14 → 10-30): 17 window trades ≈ −$20, plus the 10-16/10-19 orphan incident (−$520).
  the live loop replays to the cent, so this is the strategy, not plumbing: ~9 trades/month, edge too thin to see.
- statistics audit (docs/pipeline.md): ≥ 2,100 configurations have been tried on this data; the best of a few hundred
  zero-edge cells reaches PF 1.4–1.5. an in-sample PF > 1.4 on 2025–26 is therefore not evidence. the plan below buys
  evidence with a holdout that no search touches.

## success criteria (fixed before any search)

a "strong contender" must satisfy all of:

| # | test | bar |
|---|------|-----|
| 1 | target window 2025-01-01 → 2026-10-30 | PF ≥ 1.4, ≥ 150 trades (prefer ≥ 200), P&L > 0 in 2025 and in 2026 |
| 2 | holdout 2025-07-01 → 2026-10-30 (never searched) | PF ≥ 1.3 and P&L > 0 |
| 3 | train window 2022-01-03 → 2025-06-30 | PF ≥ 1.3, sign-flip p < 0.05, no calendar year < −300, ex-best-year PF ≥ 1.2 |
| 4 | vs v18 on the target window, paired day bootstrap | P(diff ≤ 0) < 0.10 at the Šidák level for the number of finalists revealed on the holdout |
| 5 | risk | max drawdown on the target window ≤ $1,000 on $10k (≤ 10 %), largest single day ≥ −$300 |
| 6 | plumbing | `paper_trader --replay-bars` on one recent session reproduces the lab's trades for that day |

why holdout PF 1.3 and not 1.4: a true PF-1.4 strategy with ~160 trades shows PF < 1.4 about a third of the time;
1.3 keeps the test meaningful without rejecting the thing we are looking for. the full-window PF ≥ 1.4 (test 1)
is still required.

## protocol

- data: `data/bars_iex` (the live feed's cache, 2022-01-03 → 2026-10-30, 41 symbols), house costs
  (3 bps + $0.005), capital 10,000, lookback 8 days, `--cross-index SPY`, research sizing 0.36 / 0.36.
- windows: `train` = 2022-01-03 → 2025-06-30 (≈ 875 sessions); `holdout` = 2025-07-01 → 2026-10-30 (≈ 335 sessions);
  `target` = 2025-01-01 → 2026-10-30. agents may also run any sub-window inside `train` (e.g. `2025h1`).
- the lab refuses to run `holdout` / `target` / `all` unless `LAB_REVEAL=1`. only the coordinator sets it, once,
  for the finalists. every run is appended to `research/lab/ledger.jsonl` (agent, spec hash, window, metrics):
  the ledger is the `cells_scanned` count.
- candidate = spec JSON: base config row (12 unless stated) + patch files + CLI overrides + tickers. patches
  live in `research/lab/patches/<desk>/`. a spec is reproducible from the repo alone.
- finalists: each desk nominates ≤ 3; the coordinator picks ≤ 8 by train-window strength and diversity,
  reveals the holdout once, applies tests 1–6, and writes `research/lab/REPORT.md`. the winner(s) go to the
  pipeline as `proposed` candidates with `cells_scanned` = ledger count, and a shadow book. promotion stays human.

## harness (`scripts/lab/lab.py`)

- one process per session day (the parity-exact replay path), fanned over all cores with a machine-wide
  slot semaphore (`/tmp/lab_slots`), so several agents share the box without oversubscribing it.
- per-(spec, day) result cache in `data/lab_cache/` keyed by a hash of the binary, the bars dir, the config,
  the patch contents and the args: re-running a spec, or a window that overlaps an earlier one, is free.
- one command gives the verdict block: P&L / trades / PF per year and per window, ex-best-year PF, sign-flip p,
  bootstrap CIs, drawdown, worst day, by ticker / entry window / exit reason / month, trades per month.
- `lab.py compare A B` = paired day bootstrap (reuses `scripts/pipeline/metrics.py`).
- `lab.py score <tag>` scores any existing `data/<tag>_<year>_trades.csv` set on the windows (373 tags exist).

## desks (parallel agents, each with its own angle and budget)

| desk | angle | budget |
|------|-------|--------|
| meta | re-score the 373 existing sweep tags and 49 pipeline candidates on train / 2025h1; what structural features (window, hold, filter, ticker) go with recent-regime edge | no backtests |
| universe | the v18 engine on each of the other 37 cached names, one at a time, then baskets; which names carry the short-morning edge recently | ≈ 60 cells |
| long-side | re-enable the long windows with regime filters (SPY above/below trend, VPIN, time) — 2025–26 is a bull tape | ≈ 60 cells |
| session | afternoon / full-day windows, later force-exit, max-hold / loss-reduction grid, entry-after-11:30 | ≈ 60 cells |
| exits | trailing ATR multiplier, hard stop, breakeven (loses by design), score exit threshold, profit extension | ≈ 60 cells |
| indicators | the 2026-10-29 knobs: standard supertrend, signed ADX, MACD pct-normalised; 1h gate strengths | ≈ 40 cells |
| regime | SPY-trend / realised-vol / VIX-proxy / day-of-week / turn-of-month gates on top of v18 and on top of the best other desks' cells | ≈ 60 cells |
| signals | fresh entry families on the 41-name cache: ORB fade, VWAP reclaim/fade, gap-and-go/fade, previous-day-level breaks (the earlier `sig_*` attempts are the prior) | ≈ 80 cells |

each desk: start from the ledger and the meta report, run on `train` first (coarse grids on `2025h1` are allowed
as a screen), keep a running `research/lab/desks/<desk>.md` with every cell and verdict, nominate ≤ 3 finalists
with the exact spec files. desks do not touch the holdout, the promoted row, the pipeline or the trader.

## phases

0. build the lab, baseline v18 on train (coordinator) — tonight.
1. desks in parallel (compute is the limit: ~1 s per session-day per core; a full train run ≈ 80 s with the box
   to itself, longer when shared — screen on `2025h1`, confirm on `train`).
2. synthesis: ledger review, finalists chosen on train strength + diversity + simplicity.
3. holdout reveal, tests 1–6, REPORT.md, pipeline candidate(s) + shadow book(s).

## finalists (fixed 2026-11-01 before the holdout reveal; ledger at this point: ~230 runs / ~195 distinct specs)

1. `frontier/na_bundle` — noADX row-21 core + stress-s15-core window + ×1.25 tier at VPIN ≥ 0.26 (frontier nomination 1)
2. `frontier/nb_v15` — the same at VPIN 0.15 (volume alternative)
3. `indicators/r21_v020_noadx_t10` — the core alone, no sizing levers (indicators nomination 1)
4. `universe/nt10_b3` — the core without AAPL (universe's single nomination; NVDA-heavy)
5. `frontier/na_pdl10` — the core + prior-day-low exclusion (quality alternative, low volume)
6. `frontier/na_bundle` + `meta/qqq_noise_pm_dm07` — two books (diversification value; QQQ book needs warm-up 22)

not revealed: tom-event (no nomination), signals-new (no nomination), long-2026 afternoon long (3 trades/month, warm-up 22 — kept for a later round).
