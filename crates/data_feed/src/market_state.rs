use types::market::{Candle, MarketState};

use crate::candle_aggregator::CandleAggregator;

/// builds MarketState from live candle data.
/// tracks session VWAP and cumulative volume.
pub struct MarketStateBuilder {
    aggregator: CandleAggregator,
    /// cumulative price*volume for VWAP calculation.
    cumulative_pv: f64,
    /// cumulative volume for VWAP calculation.
    cumulative_volume: f64,
}

impl MarketStateBuilder {
    pub fn new(max_candle_window: usize) -> Self {
        Self {
            aggregator: CandleAggregator::new(max_candle_window),
            cumulative_pv: 0.0,
            cumulative_volume: 0.0,
        }
    }

    /// process a new 1-minute bar and build a MarketState snapshot.
    /// bid/ask are approximated from the candle close ± half-spread.
    pub fn on_bar(&mut self, candle: Candle, bid: f64, ask: f64) -> MarketState {
        // update VWAP: use typical price (H+L+C)/3 * volume
        let typical_price = (candle.high + candle.low + candle.close) / 3.0;
        self.cumulative_pv += typical_price * candle.volume;
        self.cumulative_volume += candle.volume;

        let session_vwap = if self.cumulative_volume > 0.0 {
            self.cumulative_pv / self.cumulative_volume
        } else {
            candle.close
        };

        let last_price = candle.close;
        let timestamp = candle.timestamp;

        // feed candle to aggregator for multi-timescale windows
        let _event = self.aggregator.on_candle(candle);

        let candles = self.aggregator.candle_windows();
        let spread = ask - bid;

        MarketState {
            last_price,
            bid,
            ask,
            timestamp,
            candles,
            spread,
            session_vwap,
            session_volume: self.cumulative_volume,
            position_context: None,
            session_progress: None,
        }
    }

    /// reset for a new trading session (new day).
    pub fn reset_session(&mut self) {
        self.aggregator.reset_session();
        self.cumulative_pv = 0.0;
        self.cumulative_volume = 0.0;
    }

    /// seed with historical candles to bootstrap indicator lookback.
    pub fn seed(&mut self, candles: Vec<Candle>) {
        for candle in candles {
            let typical = (candle.high + candle.low + candle.close) / 3.0;
            self.cumulative_pv += typical * candle.volume;
            self.cumulative_volume += candle.volume;
            let _event = self.aggregator.on_candle(candle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use types::market::Timescale;

    fn make_candle(hour: u32, minute: u32, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(2024, 6, 3, hour, minute, 0).unwrap(),
            open: close - 0.5,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume,
        }
    }

    #[test]
    fn single_bar_produces_valid_market_state() {
        let mut builder = MarketStateBuilder::new(100);
        let candle = make_candle(9, 30, 450.0, 50000.0);
        let ms = builder.on_bar(candle, 449.99, 450.01);

        assert!((ms.last_price - 450.0).abs() < f64::EPSILON);
        assert!((ms.bid - 449.99).abs() < f64::EPSILON);
        assert!((ms.ask - 450.01).abs() < f64::EPSILON);
        assert!((ms.spread - 0.02).abs() < 1e-10);
        assert!(ms.session_volume > 0.0);
        assert!(ms.candles.contains_key(&Timescale::OneMinute));
    }

    #[test]
    fn vwap_correctness_single_bar() {
        let mut builder = MarketStateBuilder::new(100);
        // candle: high=451, low=449, close=450 → typical = (451+449+450)/3 = 450.0
        let candle = make_candle(9, 30, 450.0, 10000.0);
        let ms = builder.on_bar(candle, 449.99, 450.01);

        let expected_vwap = 450.0; // typical = 450.0 with this candle helper
        assert!((ms.session_vwap - expected_vwap).abs() < 0.01);
    }

    #[test]
    fn vwap_correctness_multiple_bars() {
        let mut builder = MarketStateBuilder::new(100);

        // bar 1: typical = (101+99+100)/3 = 100.0, vol=10000
        let c1 = make_candle(9, 30, 100.0, 10000.0);
        builder.on_bar(c1, 99.99, 100.01);

        // bar 2: typical = (201+199+200)/3 = 200.0, vol=20000
        let c2 = make_candle(9, 31, 200.0, 20000.0);
        let ms = builder.on_bar(c2, 199.99, 200.01);

        // VWAP = (100*10000 + 200*20000) / (10000+20000) = 5_000_000 / 30000 = 166.67
        let expected_vwap = (100.0 * 10000.0 + 200.0 * 20000.0) / 30000.0;
        assert!((ms.session_vwap - expected_vwap).abs() < 0.01);
    }

    #[test]
    fn session_volume_accumulates() {
        let mut builder = MarketStateBuilder::new(100);

        let c1 = make_candle(9, 30, 100.0, 5000.0);
        builder.on_bar(c1, 99.99, 100.01);

        let c2 = make_candle(9, 31, 101.0, 7000.0);
        let ms = builder.on_bar(c2, 100.99, 101.01);

        assert!((ms.session_volume - 12000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn candle_windows_populated() {
        let mut builder = MarketStateBuilder::new(100);

        for m in 0..10 {
            let c = make_candle(9, 30 + m, 100.0 + m as f64, 1000.0);
            builder.on_bar(c, 99.99, 100.01);
        }

        let ms = builder.on_bar(make_candle(9, 40, 110.0, 1000.0), 109.99, 110.01);
        assert!(ms.candles.contains_key(&Timescale::OneMinute));
        assert!(ms.candles.contains_key(&Timescale::FiveMinute));
    }

    #[test]
    fn session_reset_clears_vwap() {
        let mut builder = MarketStateBuilder::new(100);

        let c1 = make_candle(9, 30, 100.0, 10000.0);
        builder.on_bar(c1, 99.99, 100.01);

        builder.reset_session();

        let c2 = make_candle(9, 30, 200.0, 5000.0);
        let ms = builder.on_bar(c2, 199.99, 200.01);

        // VWAP should be based only on the new bar
        assert!((ms.session_volume - 5000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn seed_builds_historical_windows() {
        let mut builder = MarketStateBuilder::new(100);
        let candles: Vec<Candle> = (0..20)
            .map(|m| make_candle(9, 30 + m, 100.0 + m as f64, 1000.0))
            .collect();
        builder.seed(candles);

        // after seeding, volume should be accumulated
        assert!((builder.cumulative_volume - 20000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn timestamp_from_candle() {
        let mut builder = MarketStateBuilder::new(100);
        let candle = make_candle(14, 30, 450.0, 50000.0);
        let expected_ts = candle.timestamp;
        let ms = builder.on_bar(candle, 449.99, 450.01);
        assert_eq!(ms.timestamp, expected_ts);
    }
}
