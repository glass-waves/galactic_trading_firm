# per-ticker override testing plan

## goal

find per-ticker parameter overrides that improve overall strategy performance. each ticker inherits the base v95 config and can override specific knobs. we want to discover which tickers benefit from differentiated parameters vs which are fine with the base config.

## ticker characteristics

| ticker | type | spread | volatility | expected differences |
|--------|------|--------|------------|---------------------|
| SPY | broad ETF | ~$0.01 | low | tightest spreads, most liquid. base config likely near-optimal |
| QQQ | tech ETF | ~$0.01 | medium | correlated with SPY but more volatile. may benefit from slightly different thresholds |
| AAPL | mega-cap | ~$0.01 | medium | single stock momentum. may respond differently to OFI/momentum signals |
| MSFT | mega-cap | ~$0.01 | medium | similar profile to AAPL but lower beta |
| NVDA | mega-cap | ~$0.02 | high | highest volatility, widest spreads. most likely to need different stops/sizing |

## baseline: v95 config

current promoted config (v95):
- `entry_threshold`: 0.40
- `exit_threshold`: -0.05
- `no_new_entries_after`: "11:00"
- `avoid_first_minutes`: 30
- `stop_loss_pct`: 0.02 (fixed_pct_stop)
- `atr_multiplier`: 2.5 (atr_trailing_stop)
- `sizing`: volatility_scaled, base_fraction 0.05

---

## phase 1: per-ticker baselines (~30 min)

run each ticker individually on all 3 datasets (20-day IS, 99-day full, 40-day OOS) to establish per-ticker P&L profiles.

```bash
# 20-day per-ticker baseline
for ticker in SPY QQQ AAPL MSFT NVDA; do
    echo "=== $ticker ==="
    ./scripts/backtest_20days.sh --tickers $ticker
done

# 99-day per-ticker (single-ticker runs)
for ticker in SPY QQQ AAPL MSFT NVDA; do
    echo "=== $ticker ==="
    ./target/release/backtest --date 2026-03-02 --lookback-days 99 \
        --tickers $ticker --slippage-bps 2.0 --half-spread 0.005
done
```

**record in a table:**

| ticker | 20d P&L | 20d trades | 20d PF | 99d P&L | 99d trades | 99d PF | OOS P&L | notes |
|--------|---------|------------|--------|---------|------------|--------|---------|-------|
| SPY | | | | | | | | |
| QQQ | | | | | | | | |
| AAPL | | | | | | | | |
| MSFT | | | | | | | | |
| NVDA | | | | | | | | |

**decision gate**: if any ticker has PF < 1.0 on 99-day or negative OOS P&L, it's a priority target for overrides. if all tickers are profitable with PF > 1.5, overrides are optimization rather than repair.

---

## phase 2: entry threshold sweep (~1 hr)

the entry threshold is the most impactful knob. test per-ticker thresholds on the 20-day set.

```bash
# sweep entry_threshold per ticker
for ticker in SPY QQQ AAPL MSFT NVDA; do
    echo "=== $ticker ==="
    for thresh in 0.30 0.35 0.40 0.45 0.50; do
        echo -n "  threshold=$thresh: "
        ./scripts/backtest_20days.sh --tickers $ticker \
            --ticker-override "$ticker:entry_threshold=$thresh" 2>&1 | grep "total P&L"
    done
done
```

**what we're looking for**:
- does the optimal threshold differ by > 0.05 between tickers?
- does NVDA (high vol) need a higher threshold to avoid false entries?
- does SPY (low vol) benefit from a lower threshold to capture more trades?

**record results:**

| ticker | 0.30 | 0.35 | 0.40 (base) | 0.45 | 0.50 |
|--------|------|------|-------------|------|------|
| SPY | | | | | |
| QQQ | | | | | |
| AAPL | | | | | |
| MSFT | | | | | |
| NVDA | | | | | |

---

## phase 3: stop loss / ATR multiplier sweep (~1 hr)

volatile tickers may need wider stops to avoid getting stopped out on noise.

```bash
# stop_loss_pct sweep
for ticker in SPY QQQ AAPL MSFT NVDA; do
    echo "=== $ticker ==="
    for pct in 0.01 0.015 0.02 0.025 0.03 0.04; do
        echo -n "  stop=$pct: "
        ./scripts/backtest_20days.sh --tickers $ticker \
            --ticker-override "$ticker:stop_loss_pct=$pct" 2>&1 | grep "total P&L"
    done
done

# ATR multiplier sweep
for ticker in SPY QQQ AAPL MSFT NVDA; do
    echo "=== $ticker ==="
    for mult in 1.5 2.0 2.5 3.0 3.5; do
        echo -n "  atr_mult=$mult: "
        ./scripts/backtest_20days.sh --tickers $ticker \
            --ticker-override "$ticker:atr_multiplier=$mult" 2>&1 | grep "total P&L"
    done
done
```

**hypothesis**: NVDA needs wider stops (higher stop_loss_pct, higher ATR multiplier) due to higher intraday volatility. SPY can use tighter stops.

---

## phase 4: sizing fraction sweep (~30 min)

the vol-scaled sizing already adjusts for volatility, but per-ticker sizing caps may further improve risk management.

```bash
for ticker in SPY QQQ AAPL MSFT NVDA; do
    echo "=== $ticker ==="
    for frac in 0.03 0.04 0.05 0.06 0.08; do
        echo -n "  sizing=$frac: "
        ./scripts/backtest_20days.sh --tickers $ticker \
            --ticker-override "$ticker:sizing_fraction=$frac" 2>&1 | grep "total P&L"
    done
done
```

**hypothesis**: NVDA should have smaller position sizes (lower fraction) since it's more volatile and has wider spreads. SPY/QQQ can take larger positions.

---

## phase 5: indicator weight overrides (~1 hr, optional)

only attempt this if phase 1 baselines show specific indicators underperforming for certain tickers. the most likely candidates:

- **OFI** (`ofi_5min`): microstructure signal. may be more useful for high-volume ETFs (SPY, QQQ) than single stocks
- **momentum_persistence** (`mom_persist_5min`): may be more useful for trending names (NVDA) than mean-reverting ones (SPY)

```bash
# test disabling OFI for NVDA (weight 0 = disabled)
./scripts/backtest_20days.sh --tickers NVDA \
    --ticker-override "NVDA:weight:ofi_5min=0.00"

# test higher momentum_persistence weight for NVDA
./scripts/backtest_20days.sh --tickers NVDA \
    --ticker-override "NVDA:weight:mom_persist_5min=0.10"
```

---

## phase 6: combine winners and validate (~1 hr)

take per-ticker winners from phases 2-5 and combine.

### step 1: 20-day combined test

```bash
# example: if NVDA wants threshold 0.45 and wider stops, SPY wants threshold 0.35
./scripts/backtest_20days.sh \
    --ticker-override "SPY:entry_threshold=0.35" \
    --ticker-override "NVDA:entry_threshold=0.45,stop_loss_pct=0.03"
```

compare combined P&L vs base v95 P&L. must be strictly better.

### step 2: 99-day validation

run the combined overrides on the full 99-day set. if 99-day P&L improves, proceed.

### step 3: OOS validation

run on the 40-day hold-out set. this is the real test. if OOS degrades, the per-ticker overrides are overfit to the IS data.

### step 4: full validation suite

```bash
./scripts/validate_strategy.sh --quick \
    --ticker-override "SPY:entry_threshold=0.35" \
    --ticker-override "NVDA:entry_threshold=0.45,stop_loss_pct=0.03"
```

**promotion criteria (all must pass)**:
- IS P&L >= v95 baseline (+$37,719)
- OOS P&L >= v95 baseline (+$16,602)
- OOS/IS ratio >= 50%
- all tickers individually profitable on OOS
- report card >= 4/4

### step 5: promote to DB

```bash
# use update_config.sh with jq to add ticker_overrides
./scripts/update_config.sh \
    '.ticker_overrides = {"SPY": {"entry_threshold": 0.35}, "NVDA": {"entry_threshold": 0.45, "stop_loss_pct": 0.03}}' \
    'v96: per-ticker overrides — SPY lower entry, NVDA wider stops'
```

---

## rules of engagement

1. **one knob at a time** per ticker per phase. don't test `entry_threshold` and `stop_loss_pct` simultaneously until phase 6.
2. **20-day first, always**. only graduate to 99-day if 20-day shows improvement.
3. **OOS is the real test**. any override that improves IS but degrades OOS is overfit. discard it.
4. **no override is a valid outcome**. if the base config is already optimal for a ticker, don't force an override.
5. **record everything** in the tuning log. even negative results are valuable — they tell us the config is robust.
6. **watch for cross-ticker effects**. changing NVDA's sizing might affect available capital for other tickers if running multi-ticker. test combined, not just individual.

## expected timeline

| phase | effort | what |
|-------|--------|------|
| 1 | 30 min | per-ticker baselines (3 datasets × 5 tickers) |
| 2 | 1 hr | entry threshold sweep (5 values × 5 tickers) |
| 3 | 1 hr | stop loss + ATR sweep (6+5 values × 5 tickers) |
| 4 | 30 min | sizing fraction sweep (5 values × 5 tickers) |
| 5 | 1 hr | indicator weight overrides (optional, targeted) |
| 6 | 1 hr | combine, validate, promote |
| **total** | **~5 hr** | |
