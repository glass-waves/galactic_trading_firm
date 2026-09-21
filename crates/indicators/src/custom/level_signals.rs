//! prior-day levels as entry context (research, 2026-09-12).
//!
//! `prior_day_levels`: where the last close sits relative to yesterday's regular-session
//! range, read from the 5-minute window (200 candles ≈ 2.5 sessions, so yesterday is
//! always present once the window is warm). inside the range → (−0.49, +0.49) linear;
//! below the prior low → −0.5 − overshoot/`scale_pct` (capped −1.0); above the prior high
//! mirrored. `None` until a full prior session is in the window.
//! metadata: `prior_high`, `prior_low`, `prior_close`, `dist_low_pct`, `dist_high_pct`,
//! `dist_close_pct` (signed % of price).
use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::US::Eastern;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

fn et_date(ts: DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&Eastern).date_naive()
}

/// (high, low, close) of the most recent complete eastern date before the last bar's date.
fn prior_session(candles: &[Candle]) -> Option<(f64, f64, f64)> {
    let last = candles.last()?;
    let today = et_date(last.timestamp);
    let end = candles.iter().rposition(|c| et_date(c.timestamp) != today)?;
    let prior_date = et_date(candles[end].timestamp);
    let start = candles[..=end]
        .iter()
        .rposition(|c| et_date(c.timestamp) != prior_date)
        .map(|i| i + 1)
        .unwrap_or(0);
    // require the prior session to be reasonably complete (≥ 60 five-minute candles of 78)
    if end + 1 - start < 60 {
        return None;
    }
    let bars = &candles[start..=end];
    let high = bars.iter().map(|c| c.high).fold(f64::MIN, f64::max);
    let low = bars.iter().map(|c| c.low).fold(f64::MAX, f64::min);
    Some((high, low, bars[bars.len() - 1].close))
}

pub struct PriorDayLevels {
    scale_pct: f64,
    timescale: Timescale,
}

impl Indicator for PriorDayLevels {
    fn name(&self) -> &str {
        "prior_day_levels"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        140
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        let (high, low, close_prev) = prior_session(candles)?;
        let px = market.last_price;
        let rng = (high - low).max(1e-9);
        let score = if px < low {
            (-0.5 - ((low - px) / px) / self.scale_pct).max(-1.0)
        } else if px > high {
            (0.5 + ((px - high) / px) / self.scale_pct).min(1.0)
        } else {
            ((px - low) / rng - 0.5) * 0.98
        };
        let mut metadata = HashMap::new();
        metadata.insert("prior_high".to_string(), high);
        metadata.insert("prior_low".to_string(), low);
        metadata.insert("prior_close".to_string(), close_prev);
        metadata.insert("dist_low_pct".to_string(), (px - low) / px * 100.0);
        metadata.insert("dist_high_pct".to_string(), (px - high) / px * 100.0);
        metadata.insert("dist_close_pct".to_string(), (px - close_prev) / px * 100.0);
        // name's return since its prior close minus the index's (the end-of-day loser
        // reversal signal); only when the cross context is present
        if let Some(ir) = market.cross.and_then(|x| x.index_ret_prior_close) {
            metadata.insert("rel_close_pct".to_string(), (px / close_prev - 1.0 - ir) * 100.0);
        }
        Some(IndicatorOutput { score, raw_value: px - low, metadata })
    }
}

pub fn prior_day_levels_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let scale_pct = config
        .params
        .get("scale_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.005);
    Box::new(PriorDayLevels { scale_pct, timescale: config.timescale })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn session(day: u32, base: f64, n: usize, start_i: i64) -> Vec<Candle> {
        (0..n)
            .map(|i| Candle {
                timestamp: Utc.with_ymd_and_hms(2026, 6, day, 13, 30, 0).unwrap() + Duration::minutes(5 * i as i64) + Duration::minutes(start_i),
                open: base,
                high: base + 1.0 + (i % 3) as f64,
                low: base - 1.0 - (i % 2) as f64,
                close: base + 0.5,
                volume: 100.0,
            })
            .collect()
    }
    fn ms(candles: Vec<Candle>, px: f64) -> MarketState {
        let mut m = HashMap::new();
        let t = candles.last().unwrap().timestamp;
        m.insert(Timescale::FiveMinute, candles);
        MarketState {
            last_price: px,
            bid: px,
            ask: px,
            timestamp: t,
            candles: m,
            spread: 0.0,
            session_vwap: px,
            session_volume: 0.0,
            position_context: None,
            session_progress: None,
            entries_blocked: false,
            total_deployed_capital: None,
            total_initial_capital: None,
            index_return: None,
            cross_ticker_correlation: None,
            cross: None,
        }
    }

    #[test]
    fn below_prior_low_is_strongly_negative_and_inside_is_bounded() {
        let mut c = session(1, 100.0, 78, 0); // prior: high 103, low 98
        c.extend(session(2, 100.0, 10, 0));
        let ind = PriorDayLevels { scale_pct: 0.005, timescale: Timescale::FiveMinute };
        let o = ind.compute(&ms(c.clone(), 97.0)).unwrap();
        assert!(o.score <= -0.5, "{}", o.score);
        assert!((o.metadata["prior_low"] - 98.0).abs() < 1e-9);
        let inside = ind.compute(&ms(c, 100.5)).unwrap();
        assert!(inside.score.abs() < 0.5);
    }

    #[test]
    fn none_without_a_prior_session() {
        let c = session(2, 100.0, 10, 0);
        let ind = PriorDayLevels { scale_pct: 0.005, timescale: Timescale::FiveMinute };
        assert!(ind.compute(&ms(c, 100.0)).is_none());
    }
}
