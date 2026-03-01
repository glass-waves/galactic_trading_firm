use std::collections::HashMap;

use chrono::{DateTime, Timelike, Utc};
use types::market::{Candle, Timescale};

/// event emitted when a higher-timeframe candle completes.
#[derive(Debug, Clone)]
pub enum AggregationEvent {
    /// no higher-timeframe candle completed on this tick.
    None,
    /// a 5-minute candle completed.
    FiveMinComplete(Candle),
    /// an hourly candle completed.
    HourlyComplete(Candle),
    /// both a 5-minute and hourly candle completed (on the hour boundary).
    BothComplete { five_min: Candle, hourly: Candle },
}

/// builds 5-min and hourly candles from a 1-min candle stream.
/// uses clock-aligned boundaries (5-min at :00, :05, :10, ...; hourly at :00).
pub struct CandleAggregator {
    /// completed 1-min candles (rolling window).
    one_min_candles: Vec<Candle>,
    /// completed 5-min candles.
    five_min_candles: Vec<Candle>,
    /// completed hourly candles.
    hourly_candles: Vec<Candle>,
    /// accumulator for the current 5-min bar being built.
    five_min_acc: Option<CandleAccumulator>,
    /// accumulator for the current hourly bar being built.
    hourly_acc: Option<CandleAccumulator>,
    /// max candles to keep per timescale.
    max_window: usize,
}

/// accumulates OHLCV data for an in-progress candle.
#[derive(Debug, Clone)]
struct CandleAccumulator {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    start_time: DateTime<Utc>,
    count: usize,
}

impl CandleAccumulator {
    fn new(candle: &Candle, start_time: DateTime<Utc>) -> Self {
        Self {
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume,
            start_time,
            count: 1,
        }
    }

    fn update(&mut self, candle: &Candle) {
        self.high = self.high.max(candle.high);
        self.low = self.low.min(candle.low);
        self.close = candle.close;
        self.volume += candle.volume;
        self.count += 1;
    }

    fn to_candle(&self) -> Candle {
        Candle {
            timestamp: self.start_time,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }
}

/// compute the clock-aligned 5-minute boundary for a given timestamp.
/// e.g., 09:03 → 09:00, 09:07 → 09:05, 09:12 → 09:10.
fn five_min_boundary(ts: DateTime<Utc>) -> DateTime<Utc> {
    let minute = ts.minute();
    let aligned_minute = (minute / 5) * 5;
    ts.with_minute(aligned_minute)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

/// compute the clock-aligned hourly boundary for a given timestamp.
fn hourly_boundary(ts: DateTime<Utc>) -> DateTime<Utc> {
    ts.with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

impl CandleAggregator {
    pub fn new(max_window: usize) -> Self {
        Self {
            one_min_candles: Vec::new(),
            five_min_candles: Vec::new(),
            hourly_candles: Vec::new(),
            five_min_acc: None,
            hourly_acc: None,
            max_window,
        }
    }

    /// process a new 1-minute candle. returns an event indicating which
    /// higher-timeframe candles completed, if any.
    pub fn on_candle(&mut self, candle: Candle) -> AggregationEvent {
        // store the 1-min candle
        self.one_min_candles.push(candle.clone());
        if self.one_min_candles.len() > self.max_window {
            self.one_min_candles.remove(0);
        }

        let five_min_bound = five_min_boundary(candle.timestamp);
        let hourly_bound = hourly_boundary(candle.timestamp);

        // update 5-min accumulator
        let five_min_completed = self.update_five_min(&candle, five_min_bound);

        // update hourly accumulator
        let hourly_completed = self.update_hourly(&candle, hourly_bound);

        match (five_min_completed, hourly_completed) {
            (Some(five), Some(hour)) => AggregationEvent::BothComplete {
                five_min: five,
                hourly: hour,
            },
            (Some(five), None) => AggregationEvent::FiveMinComplete(five),
            (None, Some(hour)) => AggregationEvent::HourlyComplete(hour),
            (None, None) => AggregationEvent::None,
        }
    }

    fn update_five_min(
        &mut self,
        candle: &Candle,
        current_boundary: DateTime<Utc>,
    ) -> Option<Candle> {
        let mut completed = None;

        match &mut self.five_min_acc {
            Some(acc) if acc.start_time == current_boundary => {
                // same 5-min window — accumulate
                acc.update(candle);
            }
            Some(acc) => {
                // new boundary — finalize previous candle
                completed = Some(acc.to_candle());
                self.five_min_candles.push(completed.clone().unwrap());
                if self.five_min_candles.len() > self.max_window {
                    self.five_min_candles.remove(0);
                }
                // start new accumulator
                self.five_min_acc = Some(CandleAccumulator::new(candle, current_boundary));
            }
            None => {
                // first candle ever
                self.five_min_acc = Some(CandleAccumulator::new(candle, current_boundary));
            }
        }

        completed
    }

    fn update_hourly(
        &mut self,
        candle: &Candle,
        current_boundary: DateTime<Utc>,
    ) -> Option<Candle> {
        let mut completed = None;

        match &mut self.hourly_acc {
            Some(acc) if acc.start_time == current_boundary => {
                acc.update(candle);
            }
            Some(acc) => {
                completed = Some(acc.to_candle());
                self.hourly_candles.push(completed.clone().unwrap());
                if self.hourly_candles.len() > self.max_window {
                    self.hourly_candles.remove(0);
                }
                self.hourly_acc = Some(CandleAccumulator::new(candle, current_boundary));
            }
            None => {
                self.hourly_acc = Some(CandleAccumulator::new(candle, current_boundary));
            }
        }

        completed
    }

    /// get current candle windows for all timescales, suitable for building MarketState.
    /// includes the in-progress higher-timeframe candles as the latest entry.
    pub fn candle_windows(&self) -> HashMap<Timescale, Vec<Candle>> {
        let mut map = HashMap::new();

        if !self.one_min_candles.is_empty() {
            map.insert(Timescale::OneMinute, self.one_min_candles.clone());
        }

        // 5-min: completed candles + current in-progress bar
        let mut five_min = self.five_min_candles.clone();
        if let Some(acc) = &self.five_min_acc {
            five_min.push(acc.to_candle());
        }
        if !five_min.is_empty() {
            map.insert(Timescale::FiveMinute, five_min);
        }

        // hourly: completed candles + current in-progress bar
        let mut hourly = self.hourly_candles.clone();
        if let Some(acc) = &self.hourly_acc {
            hourly.push(acc.to_candle());
        }
        if !hourly.is_empty() {
            map.insert(Timescale::OneHour, hourly);
        }

        map
    }

    /// reset all state for a new trading session.
    pub fn reset_session(&mut self) {
        self.one_min_candles.clear();
        self.five_min_candles.clear();
        self.hourly_candles.clear();
        self.five_min_acc = None;
        self.hourly_acc = None;
    }

    /// seed with historical candles (e.g., from alpaca historical bars).
    pub fn seed_one_min(&mut self, candles: Vec<Candle>) {
        for candle in candles {
            self.on_candle(candle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_1min_candle(
        hour: u32,
        minute: u32,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(2024, 6, 3, hour, minute, 0).unwrap(),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    #[test]
    fn single_candle_no_completion() {
        let mut agg = CandleAggregator::new(100);
        let event = agg.on_candle(make_1min_candle(9, 30, 100.0, 101.0, 99.0, 100.5, 1000.0));
        assert!(matches!(event, AggregationEvent::None));
        assert_eq!(agg.candle_windows().get(&Timescale::OneMinute).unwrap().len(), 1);
    }

    #[test]
    fn five_candles_produce_one_five_min_bar() {
        let mut agg = CandleAggregator::new(100);
        // 5 candles: 09:30-09:34 (all in the 09:30 5-min bucket)
        for m in 30..35 {
            agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        // 6th candle at 09:35 triggers completion of 09:30 bar
        let event = agg.on_candle(make_1min_candle(9, 35, 101.0, 103.0, 100.0, 102.0, 1500.0));
        assert!(matches!(event, AggregationEvent::FiveMinComplete(_)));

        let windows = agg.candle_windows();
        let five_min = windows.get(&Timescale::FiveMinute).unwrap();
        // 1 completed + 1 in-progress
        assert_eq!(five_min.len(), 2);
        // completed bar should have aggregated OHLCV
        assert!((five_min[0].open - 100.0).abs() < f64::EPSILON);
        assert!((five_min[0].volume - 5000.0).abs() < f64::EPSILON); // 5 * 1000
    }

    #[test]
    fn five_min_ohlcv_correctness() {
        let mut agg = CandleAggregator::new(100);
        // varying OHLCV within a 5-min bucket
        agg.on_candle(make_1min_candle(9, 30, 100.0, 102.0, 99.0, 101.0, 1000.0));
        agg.on_candle(make_1min_candle(9, 31, 101.0, 105.0, 100.0, 104.0, 2000.0));
        agg.on_candle(make_1min_candle(9, 32, 104.0, 106.0, 103.0, 103.0, 1500.0));
        agg.on_candle(make_1min_candle(9, 33, 103.0, 104.0, 97.0, 98.0, 3000.0));
        agg.on_candle(make_1min_candle(9, 34, 98.0, 100.0, 96.0, 99.0, 500.0));

        // trigger completion
        let event = agg.on_candle(make_1min_candle(9, 35, 99.0, 100.0, 98.0, 99.5, 800.0));
        if let AggregationEvent::FiveMinComplete(bar) = event {
            assert!((bar.open - 100.0).abs() < f64::EPSILON); // first candle's open
            assert!((bar.high - 106.0).abs() < f64::EPSILON); // max high
            assert!((bar.low - 96.0).abs() < f64::EPSILON); // min low
            assert!((bar.close - 99.0).abs() < f64::EPSILON); // last candle's close
            assert!((bar.volume - 8000.0).abs() < f64::EPSILON); // sum
        } else {
            panic!("expected FiveMinComplete");
        }
    }

    #[test]
    fn sixty_candles_produce_twelve_five_min_and_one_hourly() {
        let mut agg = CandleAggregator::new(200);
        let mut five_min_count = 0;
        let mut hourly_count = 0;

        // 60 candles: 09:00 through 09:59
        for m in 0..60 {
            let event = agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
            match event {
                AggregationEvent::FiveMinComplete(_) => five_min_count += 1,
                AggregationEvent::BothComplete { .. } => {
                    five_min_count += 1;
                    hourly_count += 1;
                }
                AggregationEvent::HourlyComplete(_) => hourly_count += 1,
                AggregationEvent::None => {}
            }
        }

        // trigger the next hour boundary
        let event = agg.on_candle(make_1min_candle(10, 0, 101.0, 103.0, 100.0, 102.0, 1200.0));
        match event {
            AggregationEvent::BothComplete { .. } => {
                five_min_count += 1;
                hourly_count += 1;
            }
            AggregationEvent::FiveMinComplete(_) => five_min_count += 1,
            AggregationEvent::HourlyComplete(_) => hourly_count += 1,
            AggregationEvent::None => {}
        }

        // 60 minutes / 5 = 12 five-min bars completed when 10:00 arrives
        assert_eq!(five_min_count, 12);
        // 1 hourly bar completed (09:00-09:59) when 10:00 arrives
        assert_eq!(hourly_count, 1);
    }

    #[test]
    fn boundary_alignment_five_min() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 3, 9, 37, 15).unwrap();
        let bound = five_min_boundary(ts);
        assert_eq!(bound.minute(), 35);
        assert_eq!(bound.second(), 0);
    }

    #[test]
    fn boundary_alignment_hourly() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 3, 14, 45, 30).unwrap();
        let bound = hourly_boundary(ts);
        assert_eq!(bound.minute(), 0);
        assert_eq!(bound.second(), 0);
        assert_eq!(bound.hour(), 14);
    }

    #[test]
    fn boundary_on_exact_five_min() {
        let ts = Utc.with_ymd_and_hms(2024, 6, 3, 10, 15, 0).unwrap();
        let bound = five_min_boundary(ts);
        assert_eq!(bound.minute(), 15);
    }

    #[test]
    fn session_reset_clears_all() {
        let mut agg = CandleAggregator::new(100);
        for m in 30..40 {
            agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        assert!(!agg.candle_windows().is_empty());

        agg.reset_session();
        assert!(agg.candle_windows().is_empty());
    }

    #[test]
    fn empty_state_returns_empty_windows() {
        let agg = CandleAggregator::new(100);
        assert!(agg.candle_windows().is_empty());
    }

    #[test]
    fn max_window_limits_stored_candles() {
        let mut agg = CandleAggregator::new(5);
        for m in 0..10 {
            agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        let windows = agg.candle_windows();
        let one_min = windows.get(&Timescale::OneMinute).unwrap();
        assert_eq!(one_min.len(), 5);
    }

    #[test]
    fn in_progress_bar_included_in_windows() {
        let mut agg = CandleAggregator::new(100);
        // only 2 candles in the 09:30 bucket — no completed 5-min bar yet
        agg.on_candle(make_1min_candle(9, 30, 100.0, 102.0, 99.0, 101.0, 1000.0));
        agg.on_candle(make_1min_candle(9, 31, 101.0, 103.0, 100.0, 102.0, 1500.0));

        let windows = agg.candle_windows();
        let five_min = windows.get(&Timescale::FiveMinute).unwrap();
        // should have the in-progress bar
        assert_eq!(five_min.len(), 1);
        assert!((five_min[0].open - 100.0).abs() < f64::EPSILON);
        assert!((five_min[0].close - 102.0).abs() < f64::EPSILON);
        assert!((five_min[0].high - 103.0).abs() < f64::EPSILON);
        assert!((five_min[0].low - 99.0).abs() < f64::EPSILON);
        assert!((five_min[0].volume - 2500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn seed_processes_historical_candles() {
        let mut agg = CandleAggregator::new(100);
        let candles: Vec<Candle> = (0..10)
            .map(|m| make_1min_candle(9, 30 + m, 100.0, 102.0, 99.0, 101.0, 1000.0))
            .collect();
        agg.seed_one_min(candles);
        assert_eq!(agg.candle_windows().get(&Timescale::OneMinute).unwrap().len(), 10);
    }

    #[test]
    fn both_complete_on_hour_boundary() {
        let mut agg = CandleAggregator::new(200);
        // fill 09:55 through 09:59 (last 5-min bucket of the hour)
        for m in 55..60 {
            agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        // need to also have started the hourly accumulator earlier
        // reset and do a full hour for a cleaner test
        agg.reset_session();
        for m in 0..60 {
            agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        // 10:00 should trigger both
        let event = agg.on_candle(make_1min_candle(10, 0, 101.0, 103.0, 100.0, 102.0, 1200.0));
        assert!(matches!(event, AggregationEvent::BothComplete { .. }));
    }

    #[test]
    fn multiple_five_min_bars_accumulate() {
        let mut agg = CandleAggregator::new(100);
        // 15 candles: 3 complete 5-min bars
        for m in 0..15 {
            agg.on_candle(make_1min_candle(9, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        // trigger the third bar completion
        let _event = agg.on_candle(make_1min_candle(9, 15, 101.0, 103.0, 100.0, 102.0, 1200.0));

        let windows = agg.candle_windows();
        let five_min = windows.get(&Timescale::FiveMinute).unwrap();
        // 3 completed + 1 in-progress
        assert_eq!(five_min.len(), 4);
    }
}
