# testing strategy

standardized process for adding, testing, and validating changes to the trading engine.

---

## testing layers

### layer 1: unit tests (rust)

every indicator and action has unit tests in its respective crate's test directory. these validate correctness of the implementation in isolation.

**when to run:** after any code change, before committing.

```bash
# single crate
cargo test -p indicators -- candle
cargo test -p actions -- trailing

# full workspace (326+ tests)
cargo test --workspace

# lint (must pass before merge)
cargo clippy --workspace -- -D warnings
```

**what they cover:**
- indicator detection logic (pattern fires / doesn't fire under specific conditions)
- scoring modes (mean reversion, affirmative only, confluence multipliers)
- action behavior (entry/exit/monitor/sizing signals under different market states)
- factory registration (config JSON → correct indicator/action construction)
- engine integration (tick loop, position management, scoring pipeline)

**test locations:**
| crate | test directory | count |
|-------|---------------|-------|
| types | `crates/types/src/` (inline) | 18 |
| indicators | `crates/indicators/tests/` | 157 |
| actions | `crates/actions/tests/` | 33 |
| engine | `crates/engine/tests/` | 40 |
| backtest | `crates/backtest/tests/` | 40 |
| data_feed | `crates/data_feed/tests/` | 54 |

### layer 2: single-day backtest (spot check)

quick smoke test on one trading day to verify the feature runs end-to-end without errors.

```bash
cargo run -p backtest --release -- --date 2024-06-15 \
    --slippage-bps 3.0 --half-spread 0.005 \
    --use-entry-windows --add-candle-pattern 0.05 --add-momentum-persistence 0.05 \
    --tickers SPY,QQQ,AAPL,MSFT
```

**what to check:**
- no panics or errors in output
- trades are generated (non-zero trade count)
- P&L is reasonable (not obviously broken)
- new feature's trades appear (e.g., W5 entries show up in window:candle_reversal reason)

### layer 3: single-year backtest (primary validation)

run on 2024 first (most recent full year, most representative of current market conditions).

```bash
./scripts/backtest_year.sh 2024 \
    --use-entry-windows --add-candle-pattern 0.05 --add-momentum-persistence 0.05 \
    --tickers SPY,QQQ,AAPL,MSFT \
    > data/feature_name_2024_trades.csv 2> logs/feature_name_2024.log
```

**key metrics to evaluate:**
- **P&L**: total profit/loss (absolute)
- **profit factor (PF)**: gross wins / gross losses. target >= 2.0
- **win rate**: percentage of winning trades. target >= 50%
- **trade count**: more trades = more statistical significance. <50 is suspect
- **W/L ratio**: average winner / average loser. higher = better risk/reward
- **avg $/trade**: P&L / trades. expectancy per trade

### layer 4: parameter sweep (optimization)

sweep one parameter at a time while holding others constant. avoids combinatorial explosion.

```bash
./scripts/sweep_params.sh \
    --param "--w5-indicator-min" \
    --values "0.30 0.40 0.50" \
    --year 2024
```

produces a comparison table:

```
value     trades        P&L   win%    PF     avg/t
0.30        142    $234.56  48.6%  2.12  $1.65
0.40        108    $256.78  52.3%  2.45  $2.38   ← best
0.50         67    $198.12  55.2%  2.67  $2.96
```

**sweep guidelines:**
- sweep 1 param at a time (24 runs for 6 params × 4 values each)
- identify the 2-3 params that move metrics the most
- then do a focused grid on those movers (if needed)
- max 2 concurrent backtests (alpaca API rate limit)

### layer 5: cross-year validation (gold standard)

validate winning params across all 4 years. this is THE test for adoption.

```bash
./scripts/run_baseline.sh --tag feature_v1 \
    --extra "--use-entry-windows --add-candle-pattern 0.05 --add-momentum-persistence 0.05 --w5-indicator-min 0.40 --tickers SPY,QQQ,AAPL,MSFT"
```

**acceptance criteria:**
- positive P&L in every year (2022-2025)
- no year worse than baseline by more than 10%
- aggregate 4-year PF >= 2.0
- no single year's PF < 1.5

**red flags:**
- great in one year, bad in others → overfitting
- trade count drops dramatically in some years → filter too aggressive
- win rate varies wildly across years → regime-dependent, not robust

---

## process: adding a new indicator

1. **implement** in `crates/indicators/src/{native|composable|custom}/my_indicator.rs`
   - implement `Indicator` trait (name, timescale, min_lookback, compute)
   - output score normalized to [-1.0, +1.0]

2. **register** in `crates/indicators/src/lib.rs` → `default_indicator_registry()`

3. **unit test** in `crates/indicators/tests/`
   - detection: does it fire correctly?
   - edge cases: zero data, missing fields, boundary values
   - scoring: correct sign, magnitude, clamping
   - factory: config JSON → correct construction

4. **add CLI flag** in `crates/backtest/src/main.rs`
   - add `add_my_indicator: Option<f64>` to `ConfigOverrides`
   - parse `--add-my-indicator <weight>` in `parse_overrides()`
   - push indicator config in `apply()` with the weight
   - update `any_active()` check

5. **single-day smoke test** to verify it runs

6. **sweep weight** on 2024 to find optimal contribution

7. **cross-year validation** with best weight

---

## process: adding a new entry window

1. **define conditions** — what combination of indicator scores, timescale scores, and composite score defines this entry regime?

2. **add CLI flags** for each sweepable threshold in `ConfigOverrides`

3. **add window to `apply()`** in the windows vec with appropriate priority

4. **single-day smoke test** — verify window triggers entries

5. **sweep thresholds** on 2024 — find optimal condition values

6. **cross-year validation** with best thresholds

---

## process: adding a new exit action

1. **implement** in `crates/actions/src/exit/my_exit.rs`
   - implement `Action` trait with `ActionPhase::Exit`
   - return `ActionSignal::Exit { reason }` when condition met

2. **register** in `crates/actions/src/lib.rs` → `default_action_registry()`

3. **unit test** in `crates/actions/tests/`
   - trigger conditions: does it fire at the right time?
   - no-trigger conditions: does it hold when it should?
   - edge cases: no position, extreme values

4. **add CLI flag** in `crates/backtest/src/main.rs`
   - add override field to `ConfigOverrides`
   - parse CLI flag in `parse_overrides()`
   - patch action config in `apply()`

5. **compare vs baseline** — run 4-year test with and without the new exit

6. **key exit metrics to watch:**
   - exit reason distribution (how many trades use this exit vs others?)
   - average P&L by exit reason (is this exit helping or hurting?)
   - trade count change (exits that cause re-entry churn are harmful)

---

## process: modifying existing behavior

1. **add CLI override** that enables the change (don't modify defaults)
2. **A/B test**: run baseline vs modified on 2024
3. **if promising**: validate on all 4 years
4. **if adopted**: update `baseline_config.json` and promote to DB

---

## backtesting cost model

all backtests use a pessimistic cost model to avoid overfitting to gross returns:

| parameter | default | flag |
|-----------|---------|------|
| slippage | 3.0 bps | `--slippage-bps` |
| half-spread | $0.005 | `--half-spread` |
| capital | $10,000 | set in script |

these costs are applied per trade in `crates/backtest/src/replay.rs`.

---

## ticker universe

**standard test set:** SPY, QQQ, AAPL, MSFT (4 tickers)

NVDA is excluded from all testing — it accounts for 54% of historical P&L from 38% of trades, making it an anomalous outlier that masks true strategy performance on normal instruments.

the `--tickers SPY,QQQ,AAPL,MSFT` flag is included in `sweep_params.sh` BASE_EXTRA automatically.

---

## scripts reference

| script | purpose | usage |
|--------|---------|-------|
| `scripts/backtest_year.sh` | run all trading days in one year, CSV output | `./scripts/backtest_year.sh 2024 [args...] > data/out.csv` |
| `scripts/run_baseline.sh` | gold standard 4-year test with summary table | `./scripts/run_baseline.sh --tag v10 --extra "..."` |
| `scripts/sweep_params.sh` | sweep one param, produce comparison table | `./scripts/sweep_params.sh --param "--w5-indicator-min" --values "0.30 0.40 0.50"` |
| `scripts/summarize_backtest.sh` | summarize/compare existing CSV results | `./scripts/summarize_backtest.sh --compare baseline v10` |

---

## common pitfalls

- **overfitting to one year**: always validate across 2022-2025. 2024-only wins often fail cross-year.
- **re-entry churn**: an exit change that causes more frequent exits + re-entries can destroy P&L even if each individual trade looks fine. watch trade count.
- **NVDA concentration**: always exclude NVDA from test runs. it inflates results.
- **dual entry_threshold**: the engine reads action's `params.entry_threshold`, not `scoring.entry_threshold`. CLI `--entry-threshold` updates both. manual config edits may not.
- **build during backtest**: don't run `cargo build` while backtests are running — it can invalidate the binary mid-execution. use `cargo test/clippy -p <crate>` only.
- **max 2 concurrent backtests**: alpaca API rate limiting. `run_baseline.sh --parallel` respects this (runs 2 at a time).
