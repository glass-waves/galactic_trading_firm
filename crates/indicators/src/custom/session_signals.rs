//! session-anchored signals for entry research: they read *today's* bars rather than
//! yesterday's state, which is the failure mode of the v11–v13 long windows.
//!
//! all four are 1-minute / hourly indicators meant to be added at weight 0 so they
//! don't touch the composite; entry windows reference them via `indicator_min` /
//! `indicator_max` conditions (metadata is exposed as `{instance_id}.{key}`).

use std::collections::HashMap;

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use chrono_tz::US::Eastern;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

const SESSION_OPEN_MIN: u32 = 9 * 60 + 30;
const SESSION_LEN_MIN: f64 = 390.0;

fn et_date(ts: DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&Eastern).date_naive()
}

fn et_minutes(ts: DateTime<Utc>) -> u32 {
    let l = ts.with_timezone(&Eastern);
    l.hour() * 60 + l.minute()
}

/// bars belonging to the same eastern date as the last bar, from 09:30 ET on.
fn todays_session_bars(candles: &[Candle]) -> &[Candle] {
    let Some(last) = candles.last() else { return &[] };
    let today = et_date(last.timestamp);
    let start = candles
        .iter()
        .rposition(|c| et_date(c.timestamp) != today)
        .map(|i| i + 1)
        .unwrap_or(0);
    let bars = &candles[start..];
    // skip anything before the open (extended-hours bars are filtered upstream, but be safe)
    let first_rth = bars
        .iter()
        .position(|c| et_minutes(c.timestamp) >= SESSION_OPEN_MIN)
        .unwrap_or(bars.len());
    &bars[first_rth..]
}

// ── opening range ──────────────────────────────────────────────────────────────

/// position of the last close relative to the opening range (first `range_minutes`
/// of the session). inside the range → (−0.49, +0.49); above the range high →
/// 0.5 + overshoot/range (capped 1.0); below the low → mirrored negative.
/// `None` until the range is complete, and on any bar without a session.
pub struct OpeningRange {
    range_minutes: usize,
    timescale: Timescale,
}

impl Indicator for OpeningRange {
    fn name(&self) -> &str {
        "opening_range"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        self.range_minutes
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        let bars = todays_session_bars(candles);
        if bars.len() < self.range_minutes {
            return None;
        }
        let range = &bars[..self.range_minutes];
        let or_high = range.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let or_low = range.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        let width = or_high - or_low;
        if width <= 0.0 {
            return None;
        }
        let close = bars.last()?.close;
        let score = if close > or_high {
            (0.5 + (close - or_high) / width).min(1.0)
        } else if close < or_low {
            (-0.5 - (or_low - close) / width).max(-1.0)
        } else {
            let mid = (or_high + or_low) / 2.0;
            ((close - mid) / (width / 2.0)) * 0.49
        };
        let mut metadata = HashMap::new();
        metadata.insert("or_high".to_string(), or_high);
        metadata.insert("or_low".to_string(), or_low);
        metadata.insert("range_pct".to_string(), 100.0 * width / or_low);
        metadata.insert("bars_today".to_string(), bars.len() as f64);
        Some(IndicatorOutput { score, raw_value: close - or_high, metadata })
    }
}

pub fn opening_range_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let range_minutes = config
        .params
        .get("range_minutes")
        .and_then(|v| v.as_u64())
        .unwrap_or(15) as usize;
    Box::new(OpeningRange { range_minutes: range_minutes.max(2), timescale: config.timescale })
}

// ── gap ────────────────────────────────────────────────────────────────────────

/// overnight gap: today's first session open vs the previous session's last close,
/// as a fraction of `scale_pct` (default 1 % → score 1.0). metadata `above_open`
/// is the last close vs today's open in percent — "is the gap holding".
pub struct Gap {
    scale_pct: f64,
    timescale: Timescale,
}

impl Indicator for Gap {
    fn name(&self) -> &str {
        "gap"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        2
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        let bars = todays_session_bars(candles);
        if bars.is_empty() {
            return None;
        }
        let today_start = candles.len() - bars.len();
        // previous session's last bar = the bar right before today's first bar
        let prev_close = candles[..today_start].last()?.close;
        let today_open = bars.first()?.open;
        let close = bars.last()?.close;
        if prev_close <= 0.0 || today_open <= 0.0 {
            return None;
        }
        let gap_pct = 100.0 * (today_open / prev_close - 1.0);
        let score = (gap_pct / self.scale_pct).clamp(-1.0, 1.0);
        let mut metadata = HashMap::new();
        metadata.insert("gap_pct".to_string(), gap_pct);
        metadata.insert("above_open".to_string(), 100.0 * (close / today_open - 1.0));
        Some(IndicatorOutput { score, raw_value: gap_pct, metadata })
    }
}

pub fn gap_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let scale_pct = config.params.get("scale_pct").and_then(|v| v.as_f64()).unwrap_or(1.0);
    Box::new(Gap { scale_pct: scale_pct.max(0.05), timescale: config.timescale })
}

// ── hourly trend (regime) ──────────────────────────────────────────────────────

/// last close vs the mean close of the previous `lookback` hourly candles, as a
/// fraction of `scale_pct` (default 3 % → score 1.0). a slow regime gate.
pub struct HourlyTrend {
    lookback: usize,
    scale_pct: f64,
    timescale: Timescale,
}

impl Indicator for HourlyTrend {
    fn name(&self) -> &str {
        "hourly_trend"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        self.lookback + 1
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.lookback + 1 {
            return None;
        }
        let close = candles.last()?.close;
        let window = &candles[candles.len() - 1 - self.lookback..candles.len() - 1];
        let mean = window.iter().map(|c| c.close).sum::<f64>() / window.len() as f64;
        if mean <= 0.0 {
            return None;
        }
        let pct = 100.0 * (close / mean - 1.0);
        let score = (pct / self.scale_pct).clamp(-1.0, 1.0);
        let mut metadata = HashMap::new();
        metadata.insert("pct_vs_mean".to_string(), pct);
        Some(IndicatorOutput { score, raw_value: pct, metadata })
    }
}

pub fn hourly_trend_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let lookback = config.params.get("lookback").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
    let scale_pct = config.params.get("scale_pct").and_then(|v| v.as_f64()).unwrap_or(3.0);
    Box::new(HourlyTrend { lookback: lookback.max(2), scale_pct: scale_pct.max(0.1), timescale: config.timescale })
}

// ── session clock ──────────────────────────────────────────────────────────────

/// minutes since the 09:30 ET open as a fraction of the 390-minute session
/// (0.0 at the open, ~0.077 at 10:00, 0.5 at 12:45). lets a window say "not before
/// HH:MM" with an `indicator_min` condition and no new condition types.
/// `None` before the open. metadata `minutes` is the raw count.
pub struct SessionClock {
    timescale: Timescale,
}

impl Indicator for SessionClock {
    fn name(&self) -> &str {
        "session_clock"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        1
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let m = et_minutes(market.timestamp);
        if m < SESSION_OPEN_MIN {
            return None;
        }
        let elapsed = (m - SESSION_OPEN_MIN) as f64;
        let mut metadata = HashMap::new();
        metadata.insert("minutes".to_string(), elapsed);
        metadata.insert("weekday".to_string(), market.timestamp.with_timezone(&Eastern).weekday().num_days_from_monday() as f64);
        Some(IndicatorOutput { score: (elapsed / SESSION_LEN_MIN).clamp(0.0, 1.0), raw_value: elapsed, metadata })
    }
}

pub fn session_clock_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(SessionClock { timescale: config.timescale })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashMap as Map;

    fn bar(y: i32, mo: u32, d: u32, h: u32, mi: u32, o: f64, hi: f64, lo: f64, c: f64) -> Candle {
        Candle { timestamp: Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap(), open: o, high: hi, low: lo, close: c, volume: 1000.0 }
    }
    fn ms(candles: Vec<Candle>) -> MarketState {
        let last = candles.last().unwrap().clone();
        let mut m = Map::new();
        m.insert(Timescale::OneMinute, candles.clone());
        m.insert(Timescale::OneHour, candles);
        MarketState {
            last_price: last.close, bid: last.close, ask: last.close, timestamp: last.timestamp, candles: m,
            spread: 0.0, session_vwap: last.close, session_volume: 0.0, position_context: None, session_progress: None,
            entries_blocked: false, total_deployed_capital: None, total_initial_capital: None, index_return: None, cross_ticker_correlation: None,
        }
    }

    #[test]
    fn opening_range_needs_full_range_then_scores_breakout() {
        // 2024-06-03: 13:30 UTC = 09:30 EDT. yesterday's bar first, then today's session.
        let mut c = vec![bar(2024, 5, 31, 19, 59, 100.0, 100.5, 99.5, 100.0)];
        for i in 0..3 { c.push(bar(2024, 6, 3, 13, 30 + i, 100.0, 101.0, 99.0, 100.0)); }
        let ind = OpeningRange { range_minutes: 3, timescale: Timescale::OneMinute };
        assert!(ind.compute(&ms(c[..3].to_vec())).is_none(), "range not complete");
        let inside = ind.compute(&ms(c.clone())).unwrap();
        assert!(inside.score.abs() < 0.5, "inside the range: {}", inside.score);
        c.push(bar(2024, 6, 3, 13, 33, 101.0, 102.0, 100.9, 102.0)); // breakout above 101
        let out = ind.compute(&ms(c.clone())).unwrap();
        assert!(out.score >= 0.5, "breakout: {}", out.score);
        assert!((out.metadata["or_high"] - 101.0).abs() < 1e-9);
        c.push(bar(2024, 6, 3, 13, 34, 98.0, 98.5, 97.0, 97.5)); // breakdown below 99
        assert!(ind.compute(&ms(c)).unwrap().score <= -0.5);
    }

    #[test]
    fn gap_uses_previous_session_close_and_reports_hold() {
        let c = vec![
            bar(2024, 5, 31, 19, 59, 100.0, 100.5, 99.5, 100.0),
            bar(2024, 6, 3, 13, 30, 101.0, 101.5, 100.5, 101.5),
            bar(2024, 6, 3, 13, 31, 101.5, 102.0, 101.0, 100.5),
        ];
        let ind = Gap { scale_pct: 1.0, timescale: Timescale::OneMinute };
        let o = ind.compute(&ms(c)).unwrap();
        assert!((o.metadata["gap_pct"] - 1.0).abs() < 1e-9);
        assert!((o.score - 1.0).abs() < 1e-9);
        assert!(o.metadata["above_open"] < 0.0, "closed below today's open");
    }

    #[test]
    fn hourly_trend_sign_and_scale() {
        let mut c: Vec<Candle> = (0..10).map(|i| bar(2024, 6, 3, 10 + i, 0, 100.0, 100.0, 100.0, 100.0)).collect();
        c.push(bar(2024, 6, 3, 20, 0, 103.0, 103.0, 103.0, 103.0));
        let ind = HourlyTrend { lookback: 10, scale_pct: 3.0, timescale: Timescale::OneHour };
        let o = ind.compute(&ms(c)).unwrap();
        assert!((o.score - 1.0).abs() < 1e-9, "3% above a flat mean → 1.0, got {}", o.score);
    }

    #[test]
    fn session_clock_fraction_and_preopen_none() {
        let ind = SessionClock { timescale: Timescale::OneMinute };
        let pre = ms(vec![bar(2024, 6, 3, 13, 0, 1.0, 1.0, 1.0, 1.0)]); // 09:00 EDT
        assert!(ind.compute(&pre).is_none());
        let ten = ms(vec![bar(2024, 6, 3, 14, 0, 1.0, 1.0, 1.0, 1.0)]); // 10:00 EDT
        let o = ind.compute(&ten).unwrap();
        assert!((o.metadata["minutes"] - 30.0).abs() < 1e-9);
        assert!((o.score - 30.0 / 390.0).abs() < 1e-9);
    }
}
