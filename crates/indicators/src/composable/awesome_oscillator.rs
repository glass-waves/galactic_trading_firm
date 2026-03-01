use std::collections::HashMap;

use ta::indicators::SimpleMovingAverage;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// awesome oscillator: SMA(5) of midpoint - SMA(34) of midpoint
#[allow(dead_code)]
pub struct AwesomeOscillatorIndicator {
    fast_period: usize,
    slow_period: usize,
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl AwesomeOscillatorIndicator {
    pub fn new(fast_period: usize, slow_period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            fast_period,
            slow_period,
            timescale,
            instance_id,
            scale_factor: 2.0,
        }
    }
}

impl Indicator for AwesomeOscillatorIndicator {
    fn name(&self) -> &str {
        "awesome_oscillator"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.slow_period
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut fast_sma = SimpleMovingAverage::new(self.fast_period).ok()?;
        let mut slow_sma = SimpleMovingAverage::new(self.slow_period).ok()?;

        let mut last_ao = 0.0;
        for candle in candles {
            let midpoint = (candle.high + candle.low) / 2.0;
            let fast_val = fast_sma.next(midpoint);
            let slow_val = slow_sma.next(midpoint);
            last_ao = fast_val - slow_val;
        }

        let score = (last_ao / self.scale_factor).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: last_ao,
            metadata: HashMap::new(),
        })
    }
}

pub fn awesome_oscillator_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let fast = config.params.get("fast_period").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let slow = config.params.get("slow_period").and_then(|v| v.as_u64()).unwrap_or(34) as usize;
    Box::new(AwesomeOscillatorIndicator::new(fast, slow, config.timescale, config.instance_id.clone()))
}
