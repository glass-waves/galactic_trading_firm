use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};

use crate::indicator::IndicatorConfig;
use crate::market::{Candle, MarketState, Timescale};

/// create a candle with the given close price. open/high/low derived from close.
pub fn make_candle(close: f64, timestamp: DateTime<Utc>) -> Candle {
    Candle {
        timestamp,
        open: close - 0.5,
        high: close + 1.0,
        low: close - 1.0,
        close,
        volume: 100_000.0,
    }
}

/// create a candle from explicit OHLCV values.
pub fn make_candle_ohlcv(
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    timestamp: DateTime<Utc>,
) -> Candle {
    Candle {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    }
}

/// generate sequential timestamps starting from a fixed epoch.
pub fn sequential_timestamps(n: usize) -> Vec<DateTime<Utc>> {
    (0..n)
        .map(|i| Utc.timestamp_opt(1_700_000_000 + (i as i64) * 60, 0).unwrap())
        .collect()
}

/// build a MarketState from close prices for a given timescale.
/// last_price is set to the final close price.
pub fn make_market_state(timescale: Timescale, prices: &[f64]) -> MarketState {
    let timestamps = sequential_timestamps(prices.len());
    let candles: Vec<Candle> = prices
        .iter()
        .zip(timestamps.iter())
        .map(|(&p, &ts)| make_candle(p, ts))
        .collect();
    let last_price = prices.last().copied().unwrap_or(100.0);
    let last_ts = timestamps.last().copied().unwrap_or_else(Utc::now);

    let mut candle_map = HashMap::new();
    candle_map.insert(timescale, candles);

    MarketState {
        last_price,
        bid: last_price - 0.01,
        ask: last_price + 0.01,
        timestamp: last_ts,
        candles: candle_map,
        spread: 0.02,
        session_vwap: last_price,
        session_volume: 1_000_000.0,
    }
}

/// build a MarketState from explicit OHLCV tuples for a given timescale.
pub fn make_market_state_ohlcv(
    timescale: Timescale,
    ohlcv: &[(f64, f64, f64, f64, f64)],
) -> MarketState {
    let timestamps = sequential_timestamps(ohlcv.len());
    let candles: Vec<Candle> = ohlcv
        .iter()
        .zip(timestamps.iter())
        .map(|(&(o, h, l, c, v), &ts)| make_candle_ohlcv(o, h, l, c, v, ts))
        .collect();
    let last_price = ohlcv.last().map(|t| t.3).unwrap_or(100.0);
    let last_ts = timestamps.last().copied().unwrap_or_else(Utc::now);

    let mut candle_map = HashMap::new();
    candle_map.insert(timescale, candles);

    MarketState {
        last_price,
        bid: last_price - 0.01,
        ask: last_price + 0.01,
        timestamp: last_ts,
        candles: candle_map,
        spread: 0.02,
        session_vwap: last_price,
        session_volume: 1_000_000.0,
    }
}

/// generate n prices trending upward: base, base+step, base+2*step, ...
pub fn trending_up(n: usize, base: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| base + (i as f64) * step).collect()
}

/// generate n prices trending downward: base, base-step, base-2*step, ...
pub fn trending_down(n: usize, base: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| base - (i as f64) * step).collect()
}

/// generate n prices oscillating around base with given amplitude.
/// uses a sine-like pattern: base + amplitude * sin(i * pi/4).
pub fn ranging(n: usize, base: f64, amplitude: f64) -> Vec<f64> {
    (0..n)
        .map(|i| base + amplitude * (i as f64 * std::f64::consts::FRAC_PI_4).sin())
        .collect()
}

/// generate OHLCV tuples trending upward. high/low spread around close.
pub fn trending_up_ohlcv(n: usize, base: f64, step: f64) -> Vec<(f64, f64, f64, f64, f64)> {
    trending_up(n, base, step)
        .into_iter()
        .map(|c| (c - 0.5, c + 1.0, c - 1.0, c, 100_000.0))
        .collect()
}

/// generate OHLCV tuples trending downward.
pub fn trending_down_ohlcv(n: usize, base: f64, step: f64) -> Vec<(f64, f64, f64, f64, f64)> {
    trending_down(n, base, step)
        .into_iter()
        .map(|c| (c - 0.5, c + 1.0, c - 1.0, c, 100_000.0))
        .collect()
}

/// generate OHLCV tuples for ranging prices.
pub fn ranging_ohlcv(n: usize, base: f64, amplitude: f64) -> Vec<(f64, f64, f64, f64, f64)> {
    ranging(n, base, amplitude)
        .into_iter()
        .map(|c| (c - 0.5, c + 1.0, c - 1.0, c, 100_000.0))
        .collect()
}

/// shorthand to create an IndicatorConfig for tests.
pub fn make_indicator_config(
    indicator_type: &str,
    instance_id: &str,
    timescale: Timescale,
    weight: f64,
    params: Vec<(&str, serde_json::Value)>,
) -> IndicatorConfig {
    IndicatorConfig {
        indicator_type: indicator_type.to_string(),
        instance_id: instance_id.to_string(),
        timescale,
        enabled: true,
        weight,
        params: params
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_trending_up_is_monotonic() {
        let prices = trending_up(20, 100.0, 1.0);
        assert_eq!(prices.len(), 20);
        for i in 1..prices.len() {
            assert!(prices[i] > prices[i - 1]);
        }
        assert!((prices[0] - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trending_down_is_monotonic() {
        let prices = trending_down(20, 100.0, 1.0);
        assert_eq!(prices.len(), 20);
        for i in 1..prices.len() {
            assert!(prices[i] < prices[i - 1]);
        }
    }

    #[test]
    fn test_ranging_stays_in_bounds() {
        let prices = ranging(50, 100.0, 5.0);
        assert_eq!(prices.len(), 50);
        for &p in &prices {
            assert!(p >= 95.0 - f64::EPSILON, "price {p} below lower bound");
            assert!(p <= 105.0 + f64::EPSILON, "price {p} above upper bound");
        }
    }

    #[test]
    fn test_make_market_state_has_correct_candle_count() {
        let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 0.5));
        let candles = ms.candles.get(&Timescale::FiveMinute).unwrap();
        assert_eq!(candles.len(), 30);
        assert!((ms.last_price - 114.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_make_market_state_ohlcv() {
        let ohlcv = vec![(100.0, 102.0, 99.0, 101.0, 50_000.0)];
        let ms = make_market_state_ohlcv(Timescale::OneMinute, &ohlcv);
        let candles = ms.candles.get(&Timescale::OneMinute).unwrap();
        assert_eq!(candles.len(), 1);
        assert!((candles[0].open - 100.0).abs() < f64::EPSILON);
        assert!((candles[0].volume - 50_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_make_indicator_config() {
        let cfg = make_indicator_config(
            "rsi",
            "rsi_14",
            Timescale::FiveMinute,
            1.0,
            vec![("period", json!(14))],
        );
        assert_eq!(cfg.indicator_type, "rsi");
        assert_eq!(cfg.instance_id, "rsi_14");
        assert_eq!(cfg.timescale, Timescale::FiveMinute);
        assert!(cfg.enabled);
        assert!((cfg.weight - 1.0).abs() < f64::EPSILON);
        assert_eq!(cfg.params["period"], json!(14));
    }
}
