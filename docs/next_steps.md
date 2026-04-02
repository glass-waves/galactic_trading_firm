# next steps: entry quality improvements

**date:** 2026-03-29
**context:** exit strategy tuning complete (v9 tiered hold, +3.9% across 4yr). remaining edge is on the entry side — ScoreExit is still 40-50% of trades at 28-47% win rate, meaning nearly half of entries are marginal. improving entry filtering concentrates capital on good trades.

**testing approach:** all backtests exclude NVDA (anomalous concentration at 54% of P&L). run on SPY, QQQ, AAPL, MSFT with v9 tiered hold config as baseline. use `scripts/backtest_year.sh` with `--tickers SPY,QQQ,AAPL,MSFT` and existing CLI overrides.

---

## 1. new entry windows — candle pattern triggers

**goal:** add candle-pattern-based entry windows that capture concrete price action events day traders rely on. the existing engulfing indicator (w=0.05) contributes to the composite score but doesn't gate entries. a dedicated candle pattern window would only enter when a specific pattern fires with confluence.

**priority patterns (from research):**

| pattern | type | evidence | why it works on 5m |
|---|---|---|---|
| **hammer / shooting star** | 1-candle reversal | 55-60% WR daily, wick rejection = real order flow | long wick = institutional rejection at a level. with VWAP + volume confluence, filters noise |
| **evening star** | 3-candle exhaustion | 72% reversal (bulkowski), better intraday than morning star | captures momentum exhaustion via doji middle candle. morning star is ~52% intraday — skip it |
| **confirmed engulfing** | 3-candle (engulfing + follow-through) | 69-75% reversal, three outside up/down | existing engulfing + confirmation candle. higher conviction but lower frequency |

**implementation plan:**

1. extend `candle_pattern.rs` to detect hammer/shooting star and evening star alongside engulfing. each pattern gets its own base score. the existing confluence system (volume, VWAP location, trend) applies to all patterns unchanged.
   - hammer/shooting star: wick >= 2x body (or wick >= 60% of range, body <= 30%). base score 0.60
   - evening star: large directional candle + small body/doji + reversal candle closing past midpoint of first. base score 0.65
   - confirmed engulfing (three outside): existing engulfing + third candle closing beyond. base score 0.75

2. create new entry window `W5: candle_reversal` with conditions:
   - `indicator_min` on candle pattern instance_id >= 0.40 (pattern fired with decent confluence)
   - `timescale_min` on 1h >= 0.20 (hourly trend supports the reversal direction)
   - `composite_min` >= 0.20 (relaxed vs W1/W4 since the pattern itself is the signal)

3. backtest the new window alongside W1 + W4. sweep the `indicator_min` threshold (0.30, 0.40, 0.50) and the composite_min (0.15, 0.20, 0.30) to find the best settings.

**key design choices:**
- mean reversion mode stays on — bearish patterns at tops trigger long entries (71-76% WR on SPY per bulkowski)
- affirmative_only stays on — patterns can only boost/trigger, never block
- the candle window is additive to W1/W4, not a replacement. it captures a different entry signal (price action rejection vs indicator momentum/strength)

**what to skip:** inside bars (evidence is daily-only, 5m inside bars are noise), morning star (~52% intraday WR), three white soldiers/crows (too stringent for 5m noise), standalone doji

---

## 2. re-tune existing windows without NVDA

**goal:** all prior W1/W4 threshold sweeps were done with NVDA inflating results. re-sweep on the 4-ticker universe to find potentially different optima.

**parameters to sweep:**

| param | current | sweep range | CLI flag |
|---|---|---|---|
| W1 composite_min | 0.35 | 0.25, 0.30, 0.35, 0.40 | `--w1-composite-min` |
| W1 5m_min | 0.50 | 0.40, 0.45, 0.50, 0.55 | `--w1-5m-min` |
| W1 lead_by | 0.15 | 0.10, 0.15, 0.20 | `--w1-lead-by` |
| W4 composite_min | 0.35 | 0.25, 0.30, 0.35, 0.40 | `--w4-composite-min` |
| W4 5m_min | 0.40 | 0.30, 0.35, 0.40, 0.45 | `--w4-5m-min` |
| W4 1h_min | 0.30 | 0.20, 0.25, 0.30, 0.35 | `--w4-1h-min` |

**approach:** run sweeps on 2024 first (most recent full year). validate winners on 2022, 2023, 2025. can batch these — test the new candle window at its best params alongside each existing window param sweep so we get interaction effects for free.

**efficiency:** with ~4 values per param and 6 params, a full grid is 4096 runs. instead, sweep one param at a time holding others at current values (24 runs), identify movers, then do a focused grid on the 2-3 params that matter most.

---

## 3. per-ticker entry overrides

**goal:** AAPL at 67% WR vs MSFT at 57% WR suggests different instruments respond differently to the same thresholds. per-ticker overrides can capture this without overfitting.

**approach:**
- start with `entry_threshold` per ticker (the simplest lever). current global is 0.60 via score_threshold_entry. some tickers may want tighter (fewer but better trades) or looser (more exposure to a consistently profitable instrument).
- then consider per-ticker `stop_loss_pct` and `atr_multiplier` — different volatility profiles warrant different stop distances.
- use `--ticker-override "AAPL:entry_threshold=0.55"` CLI flag (already implemented).

**methodology:**
1. run the optimized config (from steps 1+2) on each ticker individually to see per-ticker metrics
2. for tickers with <55% WR, try tightening entry_threshold by 0.05-0.10
3. for tickers with >65% WR and low trade count, try loosening by 0.05
4. validate that per-ticker overrides improve the aggregate, not just individual tickers (avoid robbing peter to pay paul)

**constraint:** keep the number of per-ticker overrides small (1-2 params max). the strategy should work globally — overrides are corrections, not a per-ticker strategy.

---

## execution order

1. **candle patterns + new window** — implement hammer/shooting star + evening star, create W5 window, backtest
2. **existing window re-tune** — sweep W1/W4 params on 4-ticker universe, combine with best W5 settings
3. **per-ticker overrides** — after optimal windows are found, fine-tune per ticker

each step validates on 2024 first, then full 4-year cross-validation before adopting.
