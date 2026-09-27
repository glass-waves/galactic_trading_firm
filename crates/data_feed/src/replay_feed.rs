//! replay harness input/output for `paper_trader --replay-bars <dir> --replay-date D --replay-out F`.
//!
//! bars come from `<dir>/<SYMBOL>.csv` (unix-second timestamp,open,high,low,close,volume;
//! the backtest's `--fetch-bars` cache, RTH only). the eight calendar days before the
//! replay date warm the candle windows and the cross tracker exactly as the live start-up
//! does; the day's bars for every subscribed symbol are merged by timestamp — the index
//! first within a minute, so the cross context is the backtest's lag-0 view — and pushed
//! through the same `BarEvent` channel the websocket feeds. trades are collected in
//! memory and written once, in the backtest's `--output-trades-csv` trade-row format with
//! a `book` column appended. nothing in replay mode touches the database.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use types::action::TradeDirection;
use types::market::Candle;

use crate::alpaca_feed::BarEvent;
use crate::book::ReplayTrade;
use crate::session_clock::{eastern_date, is_regular_hours};

/// calendar days of history before the replay date used for warm-up (= live start-up).
pub const REPLAY_WARMUP_DAYS: i64 = 8;

/// bars for one replay run.
#[derive(Debug, Default)]
pub struct ReplayBars {
    /// per symbol, chronological, eastern dates in [date − warm-up, date).
    pub warmup: HashMap<String, Vec<Candle>>,
    /// the replay date's bars for every symbol, merged by timestamp.
    pub day: Vec<BarEvent>,
}

/// parse `timestamp,open,high,low,close,volume` (header optional). rows that do not parse
/// are an error: a corrupt cache must not silently shorten a parity run.
pub fn read_bars_csv(path: &Path) -> Result<Vec<Candle>, String> {
    let file = fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| format!("read {}: {e}", path.display()))?;
        let line = line.trim();
        if line.is_empty() || (i == 0 && line.starts_with("timestamp")) {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 6 {
            return Err(format!("{}:{}: expected 6 fields, got {}", path.display(), i + 1, f.len()));
        }
        let ts: i64 = f[0].trim().parse().map_err(|e| format!("{}:{}: bad timestamp: {e}", path.display(), i + 1))?;
        let timestamp = DateTime::from_timestamp(ts, 0)
            .ok_or_else(|| format!("{}:{}: invalid timestamp {ts}", path.display(), i + 1))?;
        let num = |k: usize, what: &str| -> Result<f64, String> {
            f[k].trim().parse::<f64>().map_err(|e| format!("{}:{}: bad {what}: {e}", path.display(), i + 1))
        };
        out.push(Candle {
            timestamp,
            open: num(1, "open")?,
            high: num(2, "high")?,
            low: num(3, "low")?,
            close: num(4, "close")?,
            volume: num(5, "volume")?,
        });
    }
    out.sort_by_key(|c| c.timestamp);
    out.dedup_by_key(|c| c.timestamp);
    Ok(out)
}

/// split one symbol's bars into warm-up and replay-day parts (regular hours only).
pub fn split_for_date(candles: Vec<Candle>, date: NaiveDate, warmup_days: i64) -> (Vec<Candle>, Vec<Candle>) {
    let start = date - Duration::days(warmup_days);
    let mut warmup = Vec::new();
    let mut day = Vec::new();
    for c in candles.into_iter().filter(|c| is_regular_hours(c.timestamp)) {
        let d = eastern_date(c.timestamp);
        if d == date {
            day.push(c);
        } else if d >= start && d < date {
            warmup.push(c);
        }
    }
    (warmup, day)
}

/// merge per-symbol day bars into one chronological stream. within a timestamp the index
/// comes first (its bar is then already known when the traded tickers tick, like the
/// backtest's same-timestamp lookup), then the other symbols in the order given.
pub fn merge_day_bars(per_symbol: Vec<(String, Vec<Candle>)>, index: &str) -> Vec<BarEvent> {
    let mut rows: Vec<(DateTime<Utc>, usize, BarEvent)> = Vec::new();
    for (rank, (symbol, candles)) in per_symbol.into_iter().enumerate() {
        let rank = if symbol == index { 0 } else { rank + 1 };
        for candle in candles {
            rows.push((candle.timestamp, rank, BarEvent { symbol: symbol.clone(), candle }));
        }
    }
    rows.sort_by_key(|(ts, rank, _)| (*ts, *rank));
    rows.into_iter().map(|(_, _, ev)| ev).collect()
}

/// load everything one replay needs. `symbols` = every subscribed symbol (union of the
/// books' tickers + the index). a symbol without a csv or without bars on `date` is an
/// error — a silent gap would make the parity check lie.
pub fn load_replay_bars(
    dir: &str,
    symbols: &[String],
    index: &str,
    date: NaiveDate,
    warmup_days: i64,
) -> Result<ReplayBars, String> {
    let mut warmup = HashMap::new();
    let mut per_symbol = Vec::new();
    for sym in symbols {
        let path = Path::new(dir).join(format!("{sym}.csv"));
        let all = read_bars_csv(&path)?;
        let (w, d) = split_for_date(all, date, warmup_days);
        if d.is_empty() {
            return Err(format!("{}: no regular-hours bars on {date}", path.display()));
        }
        warmup.insert(sym.clone(), w);
        per_symbol.push((sym.clone(), d));
    }
    Ok(ReplayBars { warmup, day: merge_day_bars(per_symbol, index) })
}

/// header of the backtest's `--output-trades-csv` output, plus `book`.
pub const REPLAY_TRADES_HEADER: &str = "row_type,date,ticker,direction,entry_time,exit_time,entry_price,exit_price,size,pnl,pnl_pct,hold_duration_ms,exit_reason,entry_reason,candle_pattern,entry_composite,exit_composite,entry_1m,entry_5m,entry_1h,exit_1m,exit_5m,exit_1h,max_composite,max_composite_time,positive_ticks,total_ticks,book";

/// decode the candle pattern from the entry scores' indicator metadata, as the backtest does.
fn candle_pattern(scores: &types::scoring::TimescaleScores) -> &'static str {
    scores
        .indicator_scores
        .as_ref()
        .and_then(|m| m.get("candle_5min.pattern_name"))
        .and_then(|v| *v)
        .map(|v| match v as i64 {
            1 => "bullish_engulfing",
            -1 => "bearish_engulfing",
            2 => "hammer",
            -2 => "shooting_star",
            -3 => "evening_star",
            4 => "three_outside_up",
            -4 => "three_outside_down",
            _ => "",
        })
        .unwrap_or("")
}

/// one trade row in the backtest's format (`trade,...` with the same trailing empties)
/// followed by `,<book>`.
pub fn replay_trade_row(t: &ReplayTrade) -> String {
    let tr = &t.tws.trade;
    let dir = match tr.direction {
        TradeDirection::Long => "Long",
        TradeDirection::Short => "Short",
    };
    let e = &t.tws.entry_scores;
    let x = &t.tws.exit_scores;
    let opt = |v: Option<f64>| v.map(|v| format!("{v:.4}")).unwrap_or_default();
    format!(
        "trade,{},{},{},{},{},{:.4},{:.4},{:.2},{:.2},{:.6},{},{:?},{},{},{:.4},{:.4},{},{},{},{},{},{},,,,{}",
        eastern_date(tr.entry_time),
        tr.ticker,
        dir,
        tr.entry_time.to_rfc3339(),
        tr.exit_time.to_rfc3339(),
        tr.entry_price,
        tr.exit_price,
        tr.size,
        tr.pnl,
        tr.pnl_pct,
        tr.hold_duration_ms,
        tr.exit_reason,
        t.tws.entry_reason,
        candle_pattern(e),
        e.composite,
        x.composite,
        opt(e.one_minute),
        opt(e.five_minute),
        opt(e.one_hour),
        opt(x.one_minute),
        opt(x.five_minute),
        opt(x.one_hour),
        t.book,
    )
}

/// write the collected trades (sorted by book, ticker, entry time) to `path`.
pub fn write_replay_trades(path: &str, trades: &[ReplayTrade]) -> std::io::Result<()> {
    let mut rows: Vec<&ReplayTrade> = trades.iter().collect();
    rows.sort_by(|a, b| {
        (a.book.as_str(), a.tws.trade.ticker.as_str(), a.tws.trade.entry_time)
            .cmp(&(b.book.as_str(), b.tws.trade.ticker.as_str(), b.tws.trade.entry_time))
    });
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut f = fs::File::create(path)?;
    writeln!(f, "{REPLAY_TRADES_HEADER}")?;
    for t in rows {
        writeln!(f, "{}", replay_trade_row(t))?;
    }
    f.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn bar(ts: DateTime<Utc>, close: f64) -> Candle {
        Candle { timestamp: ts, open: close, high: close + 0.1, low: close - 0.1, close, volume: 100.0 }
    }

    #[test]
    fn csv_roundtrip_and_header_skip() {
        let dir = std::env::temp_dir().join(format!("gtf_replay_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("X.csv");
        fs::write(&p, "timestamp,open,high,low,close,volume\n1743773460,1,2,0.5,1.5,10\n1743773400,1,2,0.5,1.4,11\n").unwrap();
        let c = read_bars_csv(&p).unwrap();
        assert_eq!(c.len(), 2);
        assert!(c[0].timestamp < c[1].timestamp, "sorted");
        assert!((c[0].close - 1.4).abs() < f64::EPSILON);
        fs::write(&p, "1743773460,1,2,x,1.5,10\n").unwrap();
        assert!(read_bars_csv(&p).is_err(), "corrupt row is an error");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn split_keeps_only_the_warmup_window_and_the_day() {
        let date = NaiveDate::from_ymd_opt(2025, 4, 4).unwrap();
        // 2025-04-04 13:30 UTC = 09:30 EDT (in RTH); 2025-04-04 12:00 UTC is pre-market
        let d = Utc.with_ymd_and_hms(2025, 4, 4, 13, 30, 0).unwrap();
        let pre = Utc.with_ymd_and_hms(2025, 4, 4, 12, 0, 0).unwrap();
        let w1 = Utc.with_ymd_and_hms(2025, 4, 3, 14, 0, 0).unwrap();
        let old = Utc.with_ymd_and_hms(2025, 3, 20, 14, 0, 0).unwrap();
        let (w, day) = split_for_date(vec![bar(old, 1.0), bar(w1, 2.0), bar(pre, 3.0), bar(d, 4.0)], date, 8);
        assert_eq!(w.len(), 1);
        assert!((w[0].close - 2.0).abs() < f64::EPSILON);
        assert_eq!(day.len(), 1);
        assert!((day[0].close - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn merge_is_chronological_with_the_index_first_per_minute() {
        let t0 = Utc.with_ymd_and_hms(2025, 4, 4, 13, 30, 0).unwrap();
        let t1 = t0 + Duration::minutes(1);
        let per = vec![
            ("AAPL".to_string(), vec![bar(t0, 1.0), bar(t1, 1.1)]),
            ("SPY".to_string(), vec![bar(t0, 5.0), bar(t1, 5.1)]),
            ("MSFT".to_string(), vec![bar(t1, 3.1)]),
        ];
        let merged = merge_day_bars(per, "SPY");
        let order: Vec<(String, DateTime<Utc>)> = merged.iter().map(|e| (e.symbol.clone(), e.candle.timestamp)).collect();
        assert_eq!(
            order,
            vec![
                ("SPY".to_string(), t0),
                ("AAPL".to_string(), t0),
                ("SPY".to_string(), t1),
                ("AAPL".to_string(), t1),
                ("MSFT".to_string(), t1),
            ]
        );
    }

    #[test]
    fn trade_row_matches_the_backtest_layout_plus_book() {
        use crate::live_session::TradeWithScores;
        use engine::TradeRecord;
        use types::action::ExitReason;
        let entry = Utc.with_ymd_and_hms(2025, 4, 4, 13, 32, 0).unwrap();
        let exit = entry + Duration::minutes(91);
        let t = ReplayTrade {
            book: "primary".to_string(),
            tws: TradeWithScores {
                trade: TradeRecord {
                    ticker: "NVDA".to_string(),
                    direction: TradeDirection::Short,
                    entry_price: 98.1,
                    exit_price: 95.35,
                    size: 30.0,
                    entry_time: entry,
                    exit_time: exit,
                    pnl: 82.5,
                    pnl_pct: 0.028,
                    hold_duration_ms: 5_460_000,
                    exit_reason: ExitReason::MaxHoldTimeout,
                    high_water_mark: 98.2,
                    low_water_mark: 95.0,
                },
                entry_scores: Default::default(),
                exit_scores: Default::default(),
                entry_reason: "window:strong core short".to_string(),
                config_version_id: 12,
                broker_entry_price: None,
            },
            broker_exit_price: None,
        };
        let row = replay_trade_row(&t);
        let header_cols = REPLAY_TRADES_HEADER.split(',').count();
        // the backtest's trade rows carry one field fewer than its header; we mirror that
        assert_eq!(row.split(',').count(), header_cols - 1, "{row}");
        assert!(row.starts_with("trade,2025-04-04,NVDA,Short,2025-04-04T13:32:00+00:00,2025-04-04T15:03:00+00:00,98.1000,95.3500,30.00,82.50,0.028000,5460000,MaxHoldTimeout,window:strong core short,"));
        assert!(row.ends_with(",primary"));
    }
}
