# size-vpin26-x1.25

candidate: `research/sizing/size_vpin26_v1.patch.json` — disables `sizing_fixed` (fixed_fractional,
0.30) and replaces it with `sizing_tiered_vpin` (`indicator_tiered` on `vpin_1m.raw_vpin`):
`base_fraction 0.30`, tiers `[{min: 0.26, mult: 1.25}, {min: 0.217, mult: 1.0}]`, `fallback_mult
1.0` (anything below 0.217 never reaches sizing — the short entry windows already require
`vpin_1m.raw_vpin >= 0.217` to fire at all, so the fallback never applies in practice). raises
`session.max_position_pct` from 0.30 to 0.40 so the live book can actually realize the 1.25x tier
(0.30 x 1.25 = 0.375, under the new 0.40 cap). carries `_sweep_args: "--max-position-pct 0.45"` so
the five-year gate sweep (which always runs `--sizing-fraction 0.36 --max-position-pct 0.36`) is
not clamped back to the baseline: at 0.36 research sizing the top tier is 0.36 x 1.25 = 0.45,
exactly the override. no change to entries, exits, or any other window — this is a sizing-only
candidate (gate: `sizing-config`).

based on the earlier `research/volume/tiered.md` cap-0.45 follow-up study (`tier3_l0.12_cap45`),
which isolated the top (≥0.26, x1.25) tier's effect on v18's own trades: of the 305 tier-1 trades,
220 have `vpin_1m.raw_vpin >= 0.26` and are sized up, going from +1,269 at full size to +1,614
sized x1.25 — **+345 on the same trades** (tiered.md §4, "the three-tier cap-0.45 follow-up"). that
study's own `tier3_l0.12_cap45` cell also lowered the VPIN entry floor to 0.12 (a second, confounded
change not part of this candidate), so its headline +223 vs `iex_v18` mixes the +345 leverage effect
with a separate, net-negative lower-floor population; `tiered.md`'s verdict was "do not promote
tiered sizing *below* the VPIN floor." this candidate takes only the clean half of that follow-up —
the top-tier leverage on v18's existing, unchanged entries — leaving the 0.217 floor and every window
untouched, which is what the single-day check below confirms (identical entries, 1.25x size).

## single-day verification (promoted row 12, v18, vs the patch — no five-year sweep; the five-year
research lock was held by another running sweep at the time, so this check used the backtest
binary directly on two single days, exactly as `pipeline.py`'s own `run_backtest_day()` does)

```
./target/release/backtest --date <DATE> --lookback-days 8 --capital 10000 \
  --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars_iex \
  --cross-index SPY --config-id 12                                   # plain promoted run

./target/release/backtest --date <DATE> --lookback-days 8 --capital 10000 \
  --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars_iex \
  --cross-index SPY --config-id 12 \
  --patch-json research/sizing/size_vpin26_v1.patch.json --max-position-pct 0.45  # patched run
```

### 2025-04-04 (one trade, NVDA short)

| run | entry_time | direction | entry_price | exit_price | size | pnl |
|---|---|---|---|---|---|---|
| plain (row 12) | 2025-04-04T13:33 | Short | 98.1555 | 95.3536 | **30.00** | 84.06 |
| patched (sizing_tiered_vpin) | 2025-04-04T13:33 | Short | 98.1555 | 95.3536 | **37.00** | 103.67 |

37 / 30 = 1.233 (1.25x before whole-share flooring: 30 x 1.25 = 37.5 -> 37 shares). entry time,
entry price, exit time, exit price, exit reason and every score column are identical between the
two runs — only the position size (and the P&L that follows from it) differs.

### 2022-06-13 (one trade, AMZN short)

| run | entry_time | direction | entry_price | exit_price | size | pnl |
|---|---|---|---|---|---|---|
| plain (row 12) | 2022-06-13T13:50 | Short | 105.1135 | 103.2060 | **28.00** | 53.41 |
| patched (sizing_tiered_vpin) | 2022-06-13T13:50 | Short | 105.1135 | 103.2060 | **35.00** | 66.76 |

35 / 28 = 1.25 exactly. again, entries and exits are bit-for-bit identical; only size/pnl scale.

both days traded exactly one position each, both at the 1.25x tier (confirming `vpin_1m.raw_vpin`
was >= 0.26 at both entries, not just >= 0.217 — the 0.217 tier would have reproduced the plain
run's size exactly). no other ticker traded on either day in either run.

## status

proposed to the pipeline as `size-vpin26-x1.25` (kind `config`, gate `sizing-config`, base row 12).
see `docs/pipeline/status.md` / `pipeline.py show size-vpin26-x1.25` for the backtest gate once the
nightly runner (or a manual `pipeline.py backtest size-vpin26-x1.25`) runs the five-year sweep —
not done here, per the no-sweeps-while-another-agent-holds-the-research-lock constraint this study
ran under.
