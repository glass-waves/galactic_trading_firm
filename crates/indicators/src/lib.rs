pub mod aggregation;
pub mod helpers;
pub mod native;
pub mod composable;
pub mod custom;

use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig};
use types::registry::IndicatorRegistry;

/// create a registry pre-populated with all native + composable indicator factories.
pub fn default_indicator_registry() -> IndicatorRegistry {
    let mut reg = IndicatorRegistry::new();
    // native
    reg.register("rsi", native::rsi::rsi_factory);
    reg.register("ema", native::ema::ema_factory);
    reg.register("sma", native::sma::sma_factory);
    reg.register("macd", native::macd::macd_factory);
    reg.register("bollinger", native::bollinger::bollinger_factory);
    reg.register("atr", native::atr::atr_factory);
    reg.register("keltner", native::keltner::keltner_factory);
    reg.register("stochastic_fast", native::stochastic::fast_stochastic_factory);
    reg.register("stochastic_slow", native::stochastic::slow_stochastic_factory);
    reg.register("cci", native::cci::cci_factory);
    reg.register("mfi", native::mfi::mfi_factory);
    reg.register("roc", native::roc::roc_factory);
    reg.register("obv", native::obv::obv_factory);
    // composable
    reg.register("bollinger_pct_b", composable::bollinger_pct_b::bollinger_pct_b_factory);
    reg.register("bollinger_bandwidth", composable::bollinger_bandwidth::bollinger_bandwidth_factory);
    reg.register("adx", composable::adx::adx_factory);
    reg.register("supertrend", composable::supertrend::supertrend_factory);
    reg.register("vwap_distance", composable::vwap_distance::vwap_distance_factory);
    reg.register("stochastic_rsi", composable::stochastic_rsi::stochastic_rsi_factory);
    reg.register("williams_r", composable::williams_r::williams_r_factory);
    reg.register("donchian", composable::donchian::donchian_factory);
    reg.register("dema", composable::dema::dema_factory);
    reg.register("ttm_squeeze", composable::ttm_squeeze::ttm_squeeze_factory);
    reg.register("awesome_oscillator", composable::awesome_oscillator::awesome_oscillator_factory);
    // composable (additional)
    reg.register("momentum_persistence", composable::momentum_persistence::momentum_persistence_factory);
    // custom
    reg.register("ofi", custom::ofi::ofi_factory);
    reg.register("vpin", custom::vpin::vpin_factory);
    reg.register("position_direction", custom::position_context::position_direction_factory);
    reg.register("unrealized_pnl", custom::position_context::unrealized_pnl_factory);
    reg.register("hold_duration", custom::position_context::hold_duration_factory);
    reg.register("session_remaining", custom::position_context::session_remaining_factory);
    reg.register("relative_volume", custom::rvol::rvol_factory);
    reg.register("market_breadth", custom::market_breadth::market_breadth_factory);
    reg.register("cross_ticker_correlation", custom::cross_correlation::cross_correlation_factory);
    reg
}

/// build indicator instances from configs using the given registry.
/// returns a map of instance_id → indicator.
/// disabled indicators are skipped. unknown types produce an error.
pub fn build_indicators(
    configs: &[IndicatorConfig],
    registry: &IndicatorRegistry,
) -> Result<HashMap<String, Box<dyn Indicator>>, String> {
    let mut indicators = HashMap::new();
    for cfg in configs {
        if !cfg.enabled {
            continue;
        }
        let factory = registry
            .factories
            .get(&cfg.indicator_type)
            .ok_or_else(|| format!("unknown indicator type: '{}'", cfg.indicator_type))?;
        let indicator = factory(cfg);
        indicators.insert(cfg.instance_id.clone(), indicator);
    }
    Ok(indicators)
}
