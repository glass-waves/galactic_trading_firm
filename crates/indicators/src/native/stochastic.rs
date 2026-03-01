use std::collections::HashMap;

use ta::indicators::{FastStochastic, SlowStochastic};
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

// ── fast stochastic ──

#[allow(dead_code)]
pub struct FastStochasticIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl FastStochasticIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for FastStochasticIndicator {
    fn name(&self) -> &str {
        "stochastic_fast"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut stoch = FastStochastic::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last_value = Some(stoch.next(&item));
        }

        let raw = last_value?;
        let score = ((raw - 50.0) / 50.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn fast_stochastic_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(14) as usize;
    Box::new(FastStochasticIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}

// ── slow stochastic ──

#[allow(dead_code)]
pub struct SlowStochasticIndicator {
    stochastic_period: usize,
    ema_period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl SlowStochasticIndicator {
    pub fn new(stochastic_period: usize, ema_period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            stochastic_period,
            ema_period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for SlowStochasticIndicator {
    fn name(&self) -> &str {
        "stochastic_slow"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.stochastic_period + self.ema_period
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut stoch = SlowStochastic::new(self.stochastic_period, self.ema_period).ok()?;
        let mut last_value = None;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last_value = Some(stoch.next(&item));
        }

        let raw = last_value?;
        let score = ((raw - 50.0) / 50.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn slow_stochastic_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let stoch_period = config
        .params
        .get("stochastic_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(14) as usize;
    let ema_period = config
        .params
        .get("ema_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;
    Box::new(SlowStochasticIndicator::new(
        stoch_period,
        ema_period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
