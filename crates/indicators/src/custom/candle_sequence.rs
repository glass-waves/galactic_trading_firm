//! multi-bar candlestick sequences as entry triggers (research, 2026-09-12).
//!
//! one indicator type, one `pattern` per instance, so every pattern is a swappable
//! module: add an instance at weight 0 and reference it from an entry window with
//! `indicator_max <= -0.5` (bearish fired) or `indicator_min >= 0.5` (bullish fired).
//!
//! patterns are evaluated on **closed** candles only: for 1-minute the last candle in the
//! window is the bar that just closed; for 5-minute / hourly the last candle is the one
//! still forming, so it is excluded. the score therefore holds for the life of the next
//! candle (up to 5 bars on 5m), which is what an entry window with a cooldown wants.
//!
//! score: −1.0 bearish pattern completed on the latest closed candle, +1.0 bullish,
//! 0.0 otherwise. `direction` restricts to one side. metadata: `pattern_ts` (epoch
//! seconds of the completing candle), `body_pct` (its body as a % of close ×100).
use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Engulfing,
    Star,          // evening star (bearish) / morning star (bullish)
    ThreeSoldiers, // three black crows (bearish) / three white soldiers (bullish)
    PinBar,        // shooting star (bearish) / hammer (bullish), after a move
    CloudCover,    // dark cloud cover (bearish) / piercing line (bullish)
    Harami,
    InsideBreak,   // inside bar then break of its low (bearish) / high (bullish)
    ThreeBarReversal,
    FirstReversal, // first red after N green (bearish) / first green after N red
}

impl Pattern {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "engulfing" => Self::Engulfing,
            "star" => Self::Star,
            "three_soldiers" | "three_crows" => Self::ThreeSoldiers,
            "pin_bar" | "shooting_star" | "hammer" => Self::PinBar,
            "cloud_cover" | "piercing" => Self::CloudCover,
            "harami" => Self::Harami,
            "inside_break" => Self::InsideBreak,
            "three_bar_reversal" => Self::ThreeBarReversal,
            "first_reversal" => Self::FirstReversal,
            _ => return None,
        })
    }

    fn lookback(self, run_len: usize) -> usize {
        match self {
            Self::Engulfing | Self::CloudCover | Self::Harami => 2,
            Self::Star | Self::ThreeSoldiers | Self::InsideBreak | Self::ThreeBarReversal => 3,
            Self::PinBar => 4,
            Self::FirstReversal => run_len + 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Bearish,
    Bullish,
    Both,
}

pub struct CandleSequence {
    pattern: Pattern,
    direction: Direction,
    timescale: Timescale,
    /// green/red run length for `first_reversal`
    run_len: usize,
    /// minimum body of the completing candle as a fraction of its close (filters doji noise)
    min_body_pct: f64,
    #[allow(dead_code)]
    instance_id: String,
}

#[inline]
fn body(c: &Candle) -> f64 {
    (c.close - c.open).abs()
}
#[inline]
fn green(c: &Candle) -> bool {
    c.close > c.open
}
#[inline]
fn red(c: &Candle) -> bool {
    c.close < c.open
}
#[inline]
fn range(c: &Candle) -> f64 {
    (c.high - c.low).max(1e-9)
}
#[inline]
fn mid(c: &Candle) -> f64 {
    (c.open + c.close) / 2.0
}

/// bearish variants. `w` holds at least `lookback` closed candles, last = completing candle.
fn bearish(p: Pattern, w: &[Candle], run_len: usize) -> bool {
    let n = w.len();
    let c = &w[n - 1];
    match p {
        Pattern::Engulfing => {
            let p1 = &w[n - 2];
            green(p1) && red(c) && c.open >= p1.close && c.close <= p1.open && body(c) > body(p1)
        }
        Pattern::Star => {
            let (a, b) = (&w[n - 3], &w[n - 2]);
            green(a)
                && body(b) < 0.3 * body(a)
                && mid(b) > a.close
                && red(c)
                && c.close < mid(a)
        }
        Pattern::ThreeSoldiers => {
            let (a, b) = (&w[n - 3], &w[n - 2]);
            [a, b, c].iter().all(|x| red(x) && body(x) > 0.5 * range(x))
                && b.close < a.close
                && c.close < b.close
                && b.open <= a.open
                && b.open >= a.close
                && c.open <= b.open
                && c.open >= b.close
        }
        Pattern::PinBar => {
            // shooting star: long upper wick, small body near the low, after an up move
            let upper = c.high - c.open.max(c.close);
            let lower = c.open.min(c.close) - c.low;
            let b = body(c).max(1e-9);
            let up_move = w[n - 2].close > w[n - 4].close;
            upper >= 2.0 * b && lower <= b && up_move
        }
        Pattern::CloudCover => {
            let p1 = &w[n - 2];
            green(p1) && red(c) && c.open > p1.high && c.close < mid(p1) && c.close > p1.open
        }
        Pattern::Harami => {
            let p1 = &w[n - 2];
            green(p1)
                && red(c)
                && body(p1) > 0.5 * range(p1)
                && c.open <= p1.close
                && c.close >= p1.open
                && body(c) < 0.5 * body(p1)
        }
        Pattern::InsideBreak => {
            let (a, b) = (&w[n - 3], &w[n - 2]);
            b.high <= a.high && b.low >= a.low && red(c) && c.close < b.low
        }
        Pattern::ThreeBarReversal => {
            let (a, b) = (&w[n - 3], &w[n - 2]);
            b.high > a.high && red(c) && c.close < b.low && c.close < a.low.max(b.low)
        }
        Pattern::FirstReversal => {
            let prior = &w[n - 1 - run_len..n - 1];
            prior.iter().all(green) && red(c) && c.close < w[n - 2].open + 0.5 * body(&w[n - 2])
        }
    }
}

/// bullish = bearish on the price-flipped series.
fn flipped(w: &[Candle]) -> Vec<Candle> {
    w.iter()
        .map(|c| Candle {
            timestamp: c.timestamp,
            open: -c.open,
            high: -c.low,
            low: -c.high,
            close: -c.close,
            volume: c.volume,
        })
        .collect()
}

impl CandleSequence {
    pub fn new(
        pattern: Pattern,
        direction: Direction,
        timescale: Timescale,
        run_len: usize,
        min_body_pct: f64,
        instance_id: String,
    ) -> Self {
        Self { pattern, direction, timescale, run_len, min_body_pct, instance_id }
    }

    /// closed candles of this timescale (drops the forming candle on 5m/1h).
    fn closed<'a>(&self, candles: &'a [Candle]) -> &'a [Candle] {
        if self.timescale == Timescale::OneMinute || candles.is_empty() {
            candles
        } else {
            &candles[..candles.len() - 1]
        }
    }
}

impl Indicator for CandleSequence {
    fn name(&self) -> &str {
        "candle_sequence"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        self.pattern.lookback(self.run_len) + 1
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        let w = self.closed(candles);
        let need = self.pattern.lookback(self.run_len);
        if w.len() < need {
            return None;
        }
        let w = &w[w.len() - need..];
        let last = &w[w.len() - 1];
        let body_pct = body(last) / last.close.max(1e-9);
        let big_enough = body_pct >= self.min_body_pct;

        let bear = matches!(self.direction, Direction::Bearish | Direction::Both)
            && big_enough
            && bearish(self.pattern, w, self.run_len);
        let bull = !bear
            && matches!(self.direction, Direction::Bullish | Direction::Both)
            && big_enough
            && bearish(self.pattern, &flipped(w), self.run_len);

        let score = if bear {
            -1.0
        } else if bull {
            1.0
        } else {
            0.0
        };
        let mut metadata = HashMap::new();
        metadata.insert("pattern_ts".to_string(), last.timestamp.timestamp() as f64);
        metadata.insert("body_pct".to_string(), body_pct * 100.0);
        Some(IndicatorOutput { score, raw_value: score, metadata })
    }
}

pub fn candle_sequence_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let pattern = config
        .params
        .get("pattern")
        .and_then(|v| v.as_str())
        .and_then(Pattern::parse)
        .unwrap_or(Pattern::Engulfing);
    let direction = match config.params.get("direction").and_then(|v| v.as_str()) {
        Some("bearish") => Direction::Bearish,
        Some("bullish") => Direction::Bullish,
        _ => Direction::Both,
    };
    let run_len = config
        .params
        .get("run_len")
        .and_then(|v| v.as_u64())
        .map(|v| v.max(1) as usize)
        .unwrap_or(3);
    let min_body_pct = config
        .params
        .get("min_body_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Box::new(CandleSequence::new(
        pattern,
        direction,
        config.timescale,
        run_len,
        min_body_pct,
        config.instance_id.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn c(o: f64, h: f64, l: f64, cl: f64, i: i64) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(2026, 6, 1, 14, 0, 0).unwrap() + Duration::minutes(i),
            open: o,
            high: h,
            low: l,
            close: cl,
            volume: 1000.0,
        }
    }
    fn ms(candles: Vec<Candle>, ts: Timescale) -> MarketState {
        let mut m = HashMap::new();
        let last = candles.last().unwrap().close;
        let t = candles.last().unwrap().timestamp;
        m.insert(ts, candles);
        MarketState {
            last_price: last,
            bid: last,
            ask: last,
            timestamp: t,
            candles: m,
            spread: 0.0,
            session_vwap: last,
            session_volume: 0.0,
            position_context: None,
            session_progress: None,
            entries_blocked: false,
            total_deployed_capital: None,
            total_initial_capital: None,
            index_return: None,
            cross_ticker_correlation: None,
            cross: None,
        }
    }
    fn ind(p: Pattern, ts: Timescale) -> CandleSequence {
        CandleSequence::new(p, Direction::Both, ts, 3, 0.0, "x".into())
    }

    #[test]
    fn bearish_engulfing_fires_on_closed_1m_bar() {
        let w = vec![c(100.0, 101.0, 99.5, 100.8, 0), c(100.9, 101.2, 99.0, 99.2, 1)];
        let o = ind(Pattern::Engulfing, Timescale::OneMinute).compute(&ms(w, Timescale::OneMinute)).unwrap();
        assert_eq!(o.score, -1.0);
    }

    #[test]
    fn bullish_engulfing_is_mirror() {
        let w = vec![c(100.8, 101.0, 99.5, 100.0, 0), c(99.9, 102.0, 99.0, 101.5, 1)];
        let o = ind(Pattern::Engulfing, Timescale::OneMinute).compute(&ms(w, Timescale::OneMinute)).unwrap();
        assert_eq!(o.score, 1.0);
    }

    #[test]
    fn five_minute_ignores_forming_candle() {
        // pattern completes on candles 0,1; candle 2 is the forming one and must be ignored
        let w = vec![
            c(100.0, 101.0, 99.5, 100.8, 0),
            c(100.9, 101.2, 99.0, 99.2, 5),
            c(99.2, 99.3, 99.1, 99.25, 10),
        ];
        let o = ind(Pattern::Engulfing, Timescale::FiveMinute).compute(&ms(w, Timescale::FiveMinute)).unwrap();
        assert_eq!(o.score, -1.0);
    }

    #[test]
    fn three_crows_and_evening_star() {
        let crows = vec![
            c(100.0, 100.2, 99.0, 99.1, 0),
            c(99.1, 99.2, 98.0, 98.1, 1),
            c(98.1, 98.2, 97.0, 97.1, 2),
        ];
        assert_eq!(ind(Pattern::ThreeSoldiers, Timescale::OneMinute).compute(&ms(crows, Timescale::OneMinute)).unwrap().score, -1.0);
        let star = vec![
            c(100.0, 102.1, 99.9, 102.0, 0),
            c(102.3, 102.6, 102.1, 102.4, 1),
            c(102.2, 102.3, 100.0, 100.5, 2),
        ];
        assert_eq!(ind(Pattern::Star, Timescale::OneMinute).compute(&ms(star, Timescale::OneMinute)).unwrap().score, -1.0);
    }

    #[test]
    fn no_pattern_scores_zero_and_min_body_filters() {
        let w = vec![c(100.0, 100.5, 99.5, 100.1, 0), c(100.1, 100.6, 99.6, 100.2, 1)];
        assert_eq!(ind(Pattern::Engulfing, Timescale::OneMinute).compute(&ms(w.clone(), Timescale::OneMinute)).unwrap().score, 0.0);
        let strict = CandleSequence::new(Pattern::Engulfing, Direction::Both, Timescale::OneMinute, 3, 0.05, "x".into());
        let eng = vec![c(100.0, 101.0, 99.5, 100.8, 0), c(100.9, 101.2, 99.0, 99.2, 1)];
        assert_eq!(strict.compute(&ms(eng, Timescale::OneMinute)).unwrap().score, 0.0);
    }
}
