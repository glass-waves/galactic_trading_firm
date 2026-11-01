# desk brief (common to every lab desk)

Read first: `research/lab/PLAN.md` (goal, success criteria, protocol), `research/lab/HISTORY.md` (everything tried; §D is
the do-not-repeat list), then the study README under `research/<topic>/` closest to your angle. You are one desk in a
parallel programme; the coordinator combines results. Work in the repo at /home/dylmet/Projects/galactic_trading_firm.

## rules

1. **Never touch the holdout.** Run only `--window train`, `--window 2024h2_25h1`, `--window 2025h1`, `--window 2023_25h1`
   or any explicit `YYYY-MM-DD:YYYY-MM-DD` range that ends on or before 2025-06-30. The lab refuses later dates; do not set
   `LAB_REVEAL`. Do not read `data/*_2025_trades.csv` rows after 2025-06-30 or any `*_2026_trades.csv` for cells you
   design (the meta desk's scoring of *existing* tags on train windows is fine).
2. **The binary is frozen** (`target/release/backtest`, parity-verified against live). No edits under `crates/`. If your best
   idea needs an engine change, write it up as a proposal in your desk report; do not implement it.
3. Do not touch the database, the pipeline (`scripts/pipeline/`), the trader, systemd, `docs/research_queue.md`, or any
   promoted row. Do not `git commit`; the coordinator commits.
4. Every cell is a spec file under `research/lab/specs/<desk>/<cell>.json`, patches under `research/lab/patches/<desk>/`.
   A spec is reproducible from the repo alone. Give cells short descriptive names and a `note`.
5. `export LAB_AGENT=<desk>` before running, so the ledger attributes your cells. Every run is a ledger line; the ledger
   count is the multiple-comparison denominator for the whole programme, so **do not run cells you do not need**.
6. Screen cheaply, confirm properly: coarse grids on `2024h2_25h1` (≈ 250 sessions, the most recent in-sample regime) or
   `2025h1` (121 sessions), then the best ≤ 8 cells of your desk on `train` (874 sessions). A cell that is only good on
   the screen window and flat on `train` is a regime fit — say so, do not nominate it.
7. The box is shared (12 cores, machine-wide slot semaphore): one `train` run takes ~80 s alone and several minutes when
   other desks are running. Write all your specs first, then run them in a bash loop. Results are cached per (spec, day):
   re-running the same spec, or a wider window that overlaps, is free.
8. Keep `research/lab/desks/<desk>.md` current as you go: a table of every cell (name, what it changes, window, P&L /
   trades / PF / dd / sign-flip p / per-year), your reading, and dead ends. Finish with a **nominations** section: ≤ 3
   cells with spec paths, why each, and what you expect on the holdout (write the expectation down before anyone reveals it).

## tools

```
python3 scripts/lab/lab.py run research/lab/specs/<desk>/<cell>.json --window 2024h2_25h1   # verdict block
python3 scripts/lab/lab.py run ... --json                                                   # machine-readable
python3 scripts/lab/lab.py compare SPEC_A.json SPEC_B.json --window train                   # paired day bootstrap A−B
python3 scripts/lab/lab.py compare SPEC.json research/lab/specs/base/v18.json --window train  # vs the baseline
python3 scripts/lab/lab.py score <tag> [--brief] --window train                             # an existing data/<tag>_* sweep
python3 scripts/lab/lab.py top --agent <desk> --window train                                # your leaderboard
```
Per-run artifacts: `research/lab/runs/<name>.<window>.json` and `.trades.csv` (every trade with entry/exit scores,
entry_reason, exit_reason — use them to diagnose *why* a cell wins or loses, not just whether).

Spec format:
```json
{"name": "<desk>/<cell>", "config_id": 12,
 "patches": ["research/lab/patches/<desk>/<cell>.json"],
 "args": ["--entry-threshold", "0.6"],           // any backtest CLI flag (see `backtest --help` and crates/backtest/src/main.rs get_arg list)
 "tickers": "AMZN,AAPL,NVDA,MSFT",               // optional override
 "note": "one line on what this tests"}
```
Defaults (do not change unless the cell is about them): config row 12, `data/bars_iex`, lookback 8, capital 10,000,
house costs, `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`.

Patch JSON keys: `disable` [instance_ids], `indicators` [IndicatorConfig…], `actions` [ActionConfig…], `session` {…},
`tickers` […]. An indicator/action whose `instance_id` already exists is replaced in place; new ids are appended.
Baseline instance ids: indicators rsi_7_1min stoch_fast_14_1min roc_12_1min macd_fast_1min rsi_14_5min ema_20_5min
bb_20_5min macd_5min stoch_rsi_5min candle_5min vwap_dist_1hr supertrend_1hr ema_20_1hr bb_bw_20_1hr adx_14_1hr cross_1m
vpin_1m; actions reject_1m_noise(_short) window_5m_thrust(_short) window_candle_reversal window_strong_core(_short)
trailing_stop_atr hard_stop max_hold exit_session breakeven sizing_fixed sizing_vol. Dump the full blob with
`./scripts/psql.sh -tA -c "select config_blob from config_versions where id=12" | python3 -m json.tool`.

Entry-window vocabulary (`crates/actions/src/entry/conditions.rs`, `entry_window.rs`): conditions composite_min/max,
timescale_min/max/range/lead/lag/all_min/all_max/spread_max (timescale 1m|5m|1h|1d), indicator_min/max/range on an
instance_id or `instance_id.metadata_key` (e.g. `vpin_1m.raw_vpin`); window params name, direction, conditions,
entry_after, entry_before, exit_overrides {score_exit_threshold, max_hold_ms, force_exit_by}. Session keys:
no_new_entries_after, force_exit_by, avoid_first_minutes, max_concurrent_positions, max_capital_deployed_pct,
entry_cooldown_ms, max_daily_loss_pct, max_position_pct, warmup_days. Indicator types: rsi ema sma macd bollinger atr keltner
stochastic_fast/slow cci mfi roc obv bollinger_pct_b bollinger_bandwidth adx supertrend vwap_distance stochastic_rsi
williams_r donchian dema ttm_squeeze awesome_oscillator momentum_persistence ofi vpin position_direction unrealized_pnl
hold_duration session_remaining relative_volume opening_range gap hourly_trend session_clock candle_sequence
prior_day_levels cross_context event_calendar trigger_context noise_area market_breadth cross_ticker_correlation
candle_pattern (params: read the source under crates/indicators/src/). Action types: score_threshold_entry entry_window
entry_reject_gate atr_trailing_stop fixed_pct_stop session_close max_hold_timeout profit_trailing_stop vwap_stop
breakeven_stop fixed_fractional score_scaled indicator_tiered volatility_scaled.

Scaffold rule: when a cell needs to re-create the promoted windows (e.g. to change their clock), first prove the scaffold
reproduces `base/v18` trade for trade on `2025h1` (`lab.py compare` should show P&L diff 0, identical counts) before
trusting any cell built on it. The `research/timefilter/*.patch.json` files are working examples.

## what a good desk report looks like

- numbers from the lab, not from memory; per-year and per-half rows for every nominated cell;
- the mechanism: which trades a change adds or removes (diff the `.trades.csv` files), by ticker / entry window / exit;
- a control where selection could flatter you (shuffled dates, a neighbouring parameter, the no-filter twin);
- honest about cell count: "best of N" is stated for every nomination.
