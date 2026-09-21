//! cross-ticker context as an indicator (research, 2026-09-12). reads `MarketState.cross`,
//! which the backtest fills from the bar cache (`--cross-index SPY`) and live leaves `None`
//! for now — so a window condition on it can never fire live until the feed is wired.
//!
//! `field` selects the score: `index_session_ret` (default), `index_ret_5m`, `index_ret_15m`,
//! `peers_mean_session_ret` (each scaled by `scale_pct`, default 0.5 % → 1.0, clamped), or
//! `peers_red_frac` (mapped 0..1 → −1..+1, i.e. all peers red → −1). every field is also
//! exposed as metadata so one instance serves every window condition.
use std::collections::HashMap;

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
