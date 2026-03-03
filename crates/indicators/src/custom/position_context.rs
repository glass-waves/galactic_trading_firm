use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// meta-indicator that exposes position direction to the scoring pipeline.
/// +1.0 = long, -1.0 = short, 0.0 = flat (no position).
pub struct PositionDirectionIndicator {
    timescale: Timescale,
    #[allow(dead_code)]
    instance_id: String,
}

impl Indicator for PositionDirectionIndicator {
    fn name(&self) -> &str {
        "position_direction"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        0
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let ctx = market.position_context.as_ref()?;
        Some(IndicatorOutput {
            score: ctx.direction,
            raw_value: ctx.direction,
            metadata: HashMap::new(),
        })
    }
}

pub fn position_direction_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(PositionDirectionIndicator {
        timescale: config.timescale,
        instance_id: config.instance_id.clone(),
    })
}

/// meta-indicator that exposes unrealized P&L percentage.
/// score = (unrealized_pnl_pct * scale).clamp(-1.0, 1.0)
pub struct UnrealizedPnlIndicator {
    timescale: Timescale,
    #[allow(dead_code)]
    instance_id: String,
    scale: f64,
}

impl Indicator for UnrealizedPnlIndicator {
    fn name(&self) -> &str {
        "unrealized_pnl"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        0
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let ctx = market.position_context.as_ref()?;
        let score = (ctx.unrealized_pnl_pct * self.scale).clamp(-1.0, 1.0);
        Some(IndicatorOutput {
            score,
            raw_value: ctx.unrealized_pnl_pct,
            metadata: HashMap::new(),
        })
    }
}

pub fn unrealized_pnl_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let scale = config
        .params
        .get("scale")
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0); // 1% pnl → score 1.0
    Box::new(UnrealizedPnlIndicator {
        timescale: config.timescale,
        instance_id: config.instance_id.clone(),
        scale,
    })
}

/// meta-indicator for hold duration relative to max hold time.
/// score approaches 1.0 as hold_duration → max_hold.
/// 0.0 when just entered, ~1.0 near timeout.
pub struct HoldDurationIndicator {
    timescale: Timescale,
    #[allow(dead_code)]
    instance_id: String,
}

impl Indicator for HoldDurationIndicator {
    fn name(&self) -> &str {
        "hold_duration"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        0
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let ctx = market.position_context.as_ref()?;
        if ctx.max_hold_ms <= 0 {
            return None;
        }
        let ratio = (ctx.hold_duration_ms as f64 / ctx.max_hold_ms as f64).clamp(0.0, 1.0);
        Some(IndicatorOutput {
            score: ratio,
            raw_value: ctx.hold_duration_ms as f64,
            metadata: HashMap::new(),
        })
    }
}

pub fn hold_duration_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(HoldDurationIndicator {
        timescale: config.timescale,
        instance_id: config.instance_id.clone(),
    })
}

/// meta-indicator for session remaining time.
/// score = 1.0 - session_progress (high early, low near close).
pub struct SessionRemainingIndicator {
    timescale: Timescale,
    #[allow(dead_code)]
    instance_id: String,
}

impl Indicator for SessionRemainingIndicator {
    fn name(&self) -> &str {
        "session_remaining"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        0
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let progress = market.session_progress?;
        let remaining = (1.0 - progress).clamp(0.0, 1.0);
        Some(IndicatorOutput {
            score: remaining,
            raw_value: progress,
            metadata: HashMap::new(),
        })
    }
}

pub fn session_remaining_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(SessionRemainingIndicator {
        timescale: config.timescale,
        instance_id: config.instance_id.clone(),
    })
}
