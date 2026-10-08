//! cross-ticker context as an indicator (research, 2026-09-12). reads `MarketState.cross`,
//! which the backtest fills from the bar cache (`--cross-index SPY`) and live leaves `None`
//! for now — so a window condition on it can never fire live until the feed is wired.
//!
//! `field` selects the score: `index_session_ret` (default), `index_ret_5m`, `index_ret_15m`,
//! `peers_mean_session_ret` (each scaled by `scale_pct`, default 0.5 % → 1.0, clamped), or
//! `peers_red_frac` (mapped 0..1 → −1..+1, i.e. all peers red → −1). every field is also
//! exposed as metadata so one instance serves every window condition.
//!
//! metadata `index_session_ret_per_sqrt_min` (research, 2026-10-07): the index session return in
//! percent divided by sqrt(minutes elapsed since 09:30 ET, counting the current bar). a fixed band
//! on the raw session return is effectively a clock (almost always inside ±0.2 % at 09:35, rarely by
//! 10:30); dividing by sqrt(time) — a random walk's typical displacement — gives the band the same
//! meaning at any time of day. ±0.05 equals ±0.2 % at 09:45 (16 min).
use std::collections::HashMap;

use chrono::Timelike;
use chrono_tz::US::Eastern;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

pub struct CrossContextIndicator {
    field: String,
    scale_pct: f64,
    timescale: Timescale,
}

impl Indicator for CrossContextIndicator {
    fn name(&self) -> &str {
        "cross_context"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        1
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let x = market.cross?;
        let scaled = |v: f64| (v / self.scale_pct).clamp(-1.0, 1.0);
        let score = match self.field.as_str() {
            "index_ret_5m" => scaled(x.index_ret_5m),
            "index_ret_15m" => scaled(x.index_ret_15m),
            "index_ret_prior_close" => scaled(x.index_ret_prior_close.unwrap_or(0.0)),
            "peers_mean_session_ret" => scaled(x.peers_mean_session_ret),
            "peers_red_frac" => 1.0 - 2.0 * x.peers_red_frac,
            _ => scaled(x.index_session_ret),
        };
        let mut metadata = HashMap::new();
        metadata.insert("index_session_ret".to_string(), x.index_session_ret * 100.0);
        metadata.insert(
            "index_session_ret_per_sqrt_min".to_string(),
            x.index_session_ret * 100.0 / minutes_elapsed(market).sqrt(),
        );
        metadata.insert("index_ret_5m".to_string(), x.index_ret_5m * 100.0);
        metadata.insert("index_ret_15m".to_string(), x.index_ret_15m * 100.0);
        if let Some(r) = x.index_ret_prior_close {
            metadata.insert("index_ret_prior_close".to_string(), r * 100.0);
        }
        metadata.insert("peers_red_frac".to_string(), x.peers_red_frac);
        metadata.insert("peers_mean_session_ret".to_string(), x.peers_mean_session_ret * 100.0);
        Some(IndicatorOutput { score, raw_value: score, metadata })
    }
}

/// minutes since 09:30 ET including the current 1m bar (the 09:30 bar → 1), clamped to 1..=390.
fn minutes_elapsed(market: &MarketState) -> f64 {
    let t = market.timestamp.with_timezone(&Eastern);
    let m = (t.hour() * 60 + t.minute()) as i64 - 570 + 1;
    m.clamp(1, 390) as f64
}

pub fn cross_context_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let field = config
        .params
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("index_session_ret")
        .to_string();
    let scale_pct = config.params.get("scale_pct").and_then(|v| v.as_f64()).unwrap_or(0.005);
    Box::new(CrossContextIndicator { field, scale_pct, timescale: config.timescale })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use types::market::CrossContext;
    use types::test_fixtures::make_market_state;

    fn state(utc_h: u32, utc_m: u32, session_ret: f64) -> MarketState {
        let mut ms = make_market_state(Timescale::OneMinute, &[100.0, 100.5, 101.0]);
        // 2024-06-03: EDT, 09:30 ET = 13:30 UTC
        ms.timestamp = Utc.with_ymd_and_hms(2024, 6, 3, utc_h, utc_m, 0).unwrap();
        ms.cross = Some(CrossContext {
            index_session_ret: session_ret,
            index_ret_prior_close: None,
            index_ret_5m: 0.0,
            index_ret_15m: 0.0,
            peers_red_frac: 0.5,
            peers_mean_session_ret: 0.0,
        });
        ms
    }

    fn per_sqrt(ms: &MarketState) -> f64 {
        let ind = CrossContextIndicator { field: "index_session_ret".into(), scale_pct: 0.005, timescale: Timescale::OneMinute };
        ind.compute(ms).unwrap().metadata["index_session_ret_per_sqrt_min"]
    }

    #[test]
    fn per_sqrt_min_scales_by_elapsed_minutes() {
        // 09:45 ET bar = 16th minute: 0.2 % / 4 = 0.05
        assert!((per_sqrt(&state(13, 45, 0.002)) - 0.05).abs() < 1e-12);
        // 10:33 ET = 64th minute: -0.4 % / 8 = -0.05 — same band, twice the distance an hour in
        assert!((per_sqrt(&state(14, 33, -0.004)) + 0.05).abs() < 1e-12);
        // the opening bar counts as one minute
        assert!((per_sqrt(&state(13, 30, 0.001)) - 0.1).abs() < 1e-12);
    }

    #[test]
    fn per_sqrt_min_clamps_outside_regular_hours() {
        // pre-market → 1 minute; after the close → 390 minutes
        assert!((per_sqrt(&state(13, 0, 0.001)) - 0.1).abs() < 1e-12);
        assert!((per_sqrt(&state(20, 30, 0.0039)) - 0.39 / 390f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn per_sqrt_min_handles_standard_time() {
        // 2024-12-02: EST, 09:30 ET = 14:30 UTC; 09:38 ET = 9th minute: 0.3 % / 3 = 0.1
        let mut ms = state(14, 38, 0.003);
        ms.timestamp = Utc.with_ymd_and_hms(2024, 12, 2, 14, 38, 0).unwrap();
        assert!((per_sqrt(&ms) - 0.1).abs() < 1e-12);
    }

    #[test]
    fn score_unchanged_by_new_metadata() {
        let ms = state(13, 45, 0.002);
        let ind = CrossContextIndicator { field: "index_session_ret".into(), scale_pct: 0.005, timescale: Timescale::OneMinute };
        let out = ind.compute(&ms).unwrap();
        assert!((out.score - 0.4).abs() < 1e-12);
        assert!((out.metadata["index_session_ret"] - 0.2).abs() < 1e-12);
    }
}
