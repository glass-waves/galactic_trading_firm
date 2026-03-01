use std::collections::HashMap;

use ta::indicators::{BollingerBands, KeltnerChannel};
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// TTM squeeze: detects when Bollinger Bands contract inside Keltner Channel.
/// squeeze state + momentum direction.
#[allow(dead_code)]
pub struct TtmSqueezeIndicator {
    period: usize,
    bb_std: f64,
    kc_mult: f64,
    timescale: Timescale,
    instance_id: String,
}

impl TtmSqueezeIndicator {
    pub fn new(period: usize, bb_std: f64, kc_mult: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            bb_std,
            kc_mult,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for TtmSqueezeIndicator {
    fn name(&self) -> &str {
        "ttm_squeeze"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut bb = BollingerBands::new(self.period, self.bb_std).ok()?;
        let mut kc = KeltnerChannel::new(self.period, self.kc_mult).ok()?;

        let mut last_bb = None;
        let mut last_kc = None;

        for candle in candles {
            last_bb = Some(bb.next(candle.close));
            let item: ta::DataItem = candle.into();
            last_kc = Some(kc.next(&item));
        }

        let bb_out = last_bb?;
        let kc_out = last_kc?;

        let squeeze_on = bb_out.lower > kc_out.lower && bb_out.upper < kc_out.upper;

        // momentum: close relative to midline of donchian-like range
        let close = candles.last()?.close;
        let momentum = close - bb_out.average;
        let bb_width = bb_out.upper - bb_out.lower;
        let norm_momentum = if bb_width.abs() > f64::EPSILON {
            momentum / (bb_width / 2.0)
        } else {
            0.0
        };

        // score: direction from momentum, boosted if squeeze is on (potential breakout)
        let base_score = norm_momentum.clamp(-1.0, 1.0);
        let score = if squeeze_on {
            // during squeeze, amplify signal (market is coiling)
            (base_score * 1.5).clamp(-1.0, 1.0)
        } else {
            base_score * 0.7
        };

        let mut metadata = HashMap::new();
        metadata.insert("squeeze_on".to_string(), if squeeze_on { 1.0 } else { 0.0 });
        metadata.insert("momentum".to_string(), momentum);

        Some(IndicatorOutput {
            score,
            raw_value: norm_momentum,
            metadata,
        })
    }
}

pub fn ttm_squeeze_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let bb_std = config.params.get("bb_std").and_then(|v| v.as_f64()).unwrap_or(2.0);
    let kc_mult = config.params.get("kc_mult").and_then(|v| v.as_f64()).unwrap_or(1.5);
    Box::new(TtmSqueezeIndicator::new(period, bb_std, kc_mult, config.timescale, config.instance_id.clone()))
}
