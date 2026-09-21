//! live cross-ticker context: what the index (SPY) and the other traded names are doing.
//!
//! mirrors `backtest::main::session_points` / `build_cross_context` so that
//! `MarketState.cross` means the same thing live as in the replay: session return is
//! close / first RTH bar's open of the eastern date − 1; 5m/15m returns are close vs the
//! close 5 / 15 bars back within the session; peers are the other traded tickers.
//!
//! one difference: the replay looks up every symbol at the *same* timestamp, while live
//! uses each symbol's latest bar (bars for different symbols arrive within the same
//! minute in any order). a ±0.2 % band on the session return is insensitive to that.
use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::US::Eastern;
use types::market::{Candle, CrossContext};

#[derive(Debug, Default, Clone)]
struct SymbolState {
    date: Option<NaiveDate>,
    session_open: f64,
    prior_close: Option<f64>,
    closes: Vec<f64>,
    last_ts: Option<DateTime<Utc>>,
}

impl SymbolState {
    fn on_bar(&mut self, c: &Candle) {
        let d = c.timestamp.with_timezone(&Eastern).date_naive();
        if self.date != Some(d) {
            self.date = Some(d);
            self.session_open = c.open;
            self.prior_close = self.closes.last().copied();
            self.closes.clear();
        }
        self.closes.push(c.close);
        self.last_ts = Some(c.timestamp);
    }
    fn session_ret(&self) -> Option<f64> {
        let last = *self.closes.last()?;
        if self.session_open > 0.0 {
            Some(last / self.session_open - 1.0)
        } else {
            None
        }
    }
    fn ret_back(&self, k: usize) -> f64 {
        let n = self.closes.len();
        if n > k {
            self.closes[n - 1] / self.closes[n - 1 - k] - 1.0
        } else {
            0.0
        }
    }
}

/// tracks every symbol's session state; ask it for the context of one traded ticker.
#[derive(Debug, Default)]
pub struct CrossTracker {
    index: String,
    symbols: HashMap<String, SymbolState>,
    /// if the index's last bar is older than this, the context is withheld (`None`) so a
    /// stalled SPY feed cannot keep a stale "market is flat" verdict alive.
    max_index_age_secs: i64,
}

/// default staleness bound for the index bar: two minutes (bars are one minute apart).
pub const DEFAULT_MAX_INDEX_AGE_SECS: i64 = 120;

impl CrossTracker {
    pub fn new(index: &str) -> Self {
        Self { index: index.to_string(), symbols: HashMap::new(), max_index_age_secs: DEFAULT_MAX_INDEX_AGE_SECS }
    }

    pub fn with_max_index_age(mut self, secs: i64) -> Self {
        self.max_index_age_secs = secs;
        self
    }

    pub fn index_symbol(&self) -> &str {
        &self.index
    }

    /// feed one regular-hours 1-minute bar for any symbol (index or traded ticker).
    pub fn on_bar(&mut self, symbol: &str, candle: &Candle) {
        self.symbols.entry(symbol.to_string()).or_default().on_bar(candle);
    }

    /// seed from historical bars (chronological). used at startup so a mid-session restart
    /// still knows today's session open.
    pub fn seed(&mut self, symbol: &str, candles: &[Candle]) {
        for c in candles {
            self.on_bar(symbol, c);
        }
    }

    /// context for `ticker`, with `tickers` (the traded set) as the peer universe.
    /// `None` until the index has at least one bar for the ticker's current eastern date.
    pub fn context_for(&self, ticker: &str, tickers: &[String], now: DateTime<Utc>) -> Option<CrossContext> {
        let today = now.with_timezone(&Eastern).date_naive();
        let idx = self.symbols.get(&self.index)?;
        if idx.date != Some(today) {
            return None;
        }
        // staleness: `now` is the traded ticker's bar time; the index bar must be recent
        let idx_ts = idx.last_ts?;
        if (now - idx_ts).num_seconds() > self.max_index_age_secs {
            return None;
        }
        let index_session_ret = idx.session_ret()?;
        let mut n = 0usize;
        let mut red = 0usize;
        let mut sum = 0.0;
        for t in tickers.iter().filter(|t| t.as_str() != ticker) {
            if let Some(s) = self.symbols.get(t) {
                if s.date == Some(today) {
                    if let Some(r) = s.session_ret() {
                        n += 1;
                        sum += r;
                        if r < 0.0 {
                            red += 1;
                        }
                    }
                }
            }
        }
        Some(CrossContext {
            index_session_ret,
            index_ret_prior_close: match (idx.prior_close, idx.closes.last()) {
                (Some(pc), Some(last)) if pc > 0.0 => Some(last / pc - 1.0),
                _ => None,
            },
            index_ret_5m: idx.ret_back(5),
            index_ret_15m: idx.ret_back(15),
            peers_red_frac: if n > 0 { red as f64 / n as f64 } else { 0.5 },
            peers_mean_session_ret: if n > 0 { sum / n as f64 } else { 0.0 },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn bar(day: u32, i: i64, o: f64, c: f64) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(2026, 6, day, 13, 30, 0).unwrap() + Duration::minutes(i),
            open: o,
            high: o.max(c) + 0.1,
            low: o.min(c) - 0.1,
            close: c,
            volume: 100.0,
        }
    }

    #[test]
    fn session_return_and_peers() {
        let mut t = CrossTracker::new("SPY");
        t.on_bar("SPY", &bar(1, 0, 500.0, 500.5));
        t.on_bar("SPY", &bar(1, 1, 500.5, 501.0)); // +0.2 %
        t.on_bar("AAPL", &bar(1, 1, 200.0, 199.0)); // red
        t.on_bar("MSFT", &bar(1, 1, 400.0, 404.0)); // green
        let tickers = vec!["AAPL".to_string(), "MSFT".to_string(), "NVDA".to_string()];
        let x = t.context_for("NVDA", &tickers, bar(1, 1, 0.0, 0.0).timestamp).unwrap();
        assert!((x.index_session_ret - 0.002).abs() < 1e-9);
        assert!((x.peers_red_frac - 0.5).abs() < 1e-9);
        assert!((x.peers_mean_session_ret - 0.0025).abs() < 1e-9, "{}", x.peers_mean_session_ret);
        // the ticker itself is excluded from its own peers
        let y = t.context_for("AAPL", &tickers, bar(1, 1, 0.0, 0.0).timestamp).unwrap();
        assert!((y.peers_red_frac - 0.0).abs() < 1e-9);
    }

    #[test]
    fn stale_index_bar_withholds_context() {
        let mut t = CrossTracker::new("SPY");
        t.on_bar("SPY", &bar(1, 0, 500.0, 500.0));
        let tickers = vec!["AAPL".to_string()];
        assert!(t.context_for("AAPL", &tickers, bar(1, 1, 0.0, 0.0).timestamp).is_some());
        assert!(t.context_for("AAPL", &tickers, bar(1, 3, 0.0, 0.0).timestamp).is_none(), "3 min stale");
    }

    #[test]
    fn none_without_todays_index_bar_and_resets_per_day() {
        let mut t = CrossTracker::new("SPY");
        t.on_bar("SPY", &bar(1, 0, 500.0, 505.0));
        let tickers = vec!["AAPL".to_string()];
        assert!(t.context_for("AAPL", &tickers, bar(2, 0, 0.0, 0.0).timestamp).is_none());
        t.on_bar("SPY", &bar(2, 0, 510.0, 510.0));
        let x = t.context_for("AAPL", &tickers, bar(2, 0, 0.0, 0.0).timestamp).unwrap();
        assert!(x.index_session_ret.abs() < 1e-9, "new day starts at 0: {}", x.index_session_ret);
    }
}
