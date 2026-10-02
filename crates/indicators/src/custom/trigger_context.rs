//! trigger context (entry-trigger study, 2026-10-02): a weight-0 1-minute indicator that exposes
//! two things the entry windows cannot see from scores alone.
//!
//! 1. **pullback after a thrust** — over today's last `lookback` 1-minute bars (never reaching
//!    into the prior session): the lowest low, the highest high *before* it (where the thrust
//!    started) and how far the current close has come back.
//!    - `drop_pct`      — (high_before / low − 1) · 100: size of the thrust down
//!    - `bars_since_low` — 0 when the current bar made the low
//!    - `retrace`       — (close − low) / (high_before − low): 0 at the low, 1 back at the start
//!    - `bounce_pct`    — (close / low − 1) · 100
//!    - `last_ret_pct`  — the current bar's close vs the previous close, percent (resumption = < 0)
//! 2. **VPIN slope** — raw VPIN (same defaults as the `vpin` indicator: sigma 20, 20 lookback
//!    bars) now and `k` bars ago, recomputed on the window without its last `k` candles, which
//!    is exactly the value the `vpin` indicator reported `k` bars ago (stateless, so a live
//!    restart and the replay agree):
//!    - `vpin_now`, `vpin_slope_3` (now − 3 bars ago), `vpin_slope_5` (now − 5 bars ago)
//!
//! score = 2 · retrace − 1, clamped (−1 at the low, +1 fully retraced). meant for weight 0;
//! conditions read the metadata as `{instance_id}.{key}`.
use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Timelike, Utc};
use chrono_tz::US::Eastern;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

use super::vpin::VpinIndicator;

const SESSION_OPEN_MIN: u32 = 9 * 60 + 30;

fn et_date(ts: DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&Eastern).date_naive()
}

fn et_minutes(ts: DateTime<Utc>) -> u32 {
    let l = ts.with_timezone(&Eastern);
    l.hour() * 60 + l.minute()
}

/// today's regular-session bars (same eastern date as the last bar, from 09:30 ET on).
fn todays_bars(candles: &[Candle]) -> &[Candle] {
    let Some(last) = candles.last() else { return &[] };
    let today = et_date(last.timestamp);
    let start = candles
        .iter()
        .rposition(|c| et_date(c.timestamp) != today)
        .map(|i| i + 1)
        .unwrap_or(0);
    let bars = &candles[start..];
    let first = bars
        .iter()
        .position(|c| et_minutes(c.timestamp) >= SESSION_OPEN_MIN)
        .unwrap_or(bars.len());
    &bars[first..]
}

pub struct TriggerContextIndicator {
    lookback: usize,
    timescale: Timescale,
    vpin: VpinIndicator,
}

impl TriggerContextIndicator {
    pub fn new(lookback: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            lookback: lookback.max(2),
            timescale,
            vpin: VpinIndicator::new(20, 0.2, 20, timescale, instance_id),
        }
    }

    /// raw VPIN on the window without its last `k` candles.
    fn vpin_lagged(&self, market: &MarketState, k: usize) -> Option<f64> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() <= k {
            return None;
        }
        let mut m = market.clone();
        m.candles
            .insert(self.timescale, candles[..candles.len() - k].to_vec());
        self.vpin.compute(&m).map(|o| o.raw_value)
    }
}

impl Indicator for TriggerContextIndicator {
    fn name(&self) -> &str {
        "trigger_context"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        2
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        let mut metadata = HashMap::new();
        let mut score = None;

        // 1. pullback geometry over today's last `lookback` bars
        let today = todays_bars(candles);
        let bars = &today[today.len().saturating_sub(self.lookback)..];
        if bars.len() >= 2 {
            let mut i_low = 0;
            for (i, c) in bars.iter().enumerate() {
                if c.low <= bars[i_low].low {
                    i_low = i; // latest bar at the low
                }
            }
            let low = bars[i_low].low;
            let high_before = bars[..=i_low].iter().map(|c| c.high).fold(f64::MIN, f64::max);
            let close = bars[bars.len() - 1].close;
            let prev = bars[bars.len() - 2].close;
            if low > 0.0 && prev > 0.0 {
                let span = high_before - low;
                let retrace = if span > 0.0 { (close - low) / span } else { 0.0 };
                metadata.insert("drop_pct".to_string(), (high_before / low - 1.0) * 100.0);
                metadata.insert("bars_since_low".to_string(), (bars.len() - 1 - i_low) as f64);
                metadata.insert("retrace".to_string(), retrace);
                metadata.insert("bounce_pct".to_string(), (close / low - 1.0) * 100.0);
                metadata.insert("last_ret_pct".to_string(), (close / prev - 1.0) * 100.0);
                score = Some((2.0 * retrace - 1.0).clamp(-1.0, 1.0));
            }
        }

        // 2. VPIN level and slope
        if let Some(now) = self.vpin.compute(market).map(|o| o.raw_value) {
            metadata.insert("vpin_now".to_string(), now);
            for k in [3usize, 5] {
                if let Some(then) = self.vpin_lagged(market, k) {
                    metadata.insert(format!("vpin_slope_{k}"), now - then);
                }
            }
        }

        if metadata.is_empty() {
            return None;
        }
        Some(IndicatorOutput {
            score: score.unwrap_or(0.0),
            raw_value: metadata.get("retrace").copied().unwrap_or(0.0),
            metadata,
        })
    }
}

pub fn trigger_context_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let lookback = config
        .params
        .get("lookback")
        .and_then(|v| v.as_u64())
        .unwrap_or(15) as usize;
    Box::new(TriggerContextIndicator::new(
        lookback,
        config.timescale,
        config.instance_id.clone(),
    ))
}
