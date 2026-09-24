use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::America::New_York;
use types::market::{Candle, Timescale};

use crate::replay::BacktestData;

/// max retries for alpaca API requests (handles rate limiting / transient errors).
const MAX_RETRIES: u32 = 3;
/// base delay between retries in milliseconds (doubles each attempt).
const RETRY_BASE_MS: u64 = 1000;

/// convert an apca bar Num to f64, defaulting to 0.0 on failure.
fn num_to_f64(n: &num_decimal::Num) -> f64 {
    n.to_f64().unwrap_or(0.0)
}

/// check if an error string looks like a rate limit (HTTP 429) or transient server error.
fn is_retryable_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("500")
        || lower.contains("502")
        || lower.contains("503")
        || lower.contains("504")
        || lower.contains("timeout")
        || lower.contains("connection")
}

/// fetch 1-minute bars for a single ticker on a specific date from alpaca.
pub async fn fetch_day_bars(
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    date: NaiveDate,
) -> Result<Vec<Candle>, String> {
    use apca::data::v2::bars;
    use apca::ApiInfo;
    use apca::Client;

    let api_info = ApiInfo::from_parts(
        "https://paper-api.alpaca.markets",
        api_key,
        api_secret,
    )
    .map_err(|e| format!("api info error: {e}"))?;

    let client = Client::new(api_info);

    // market hours: 9:30 - 16:00 eastern
    let market_open = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
    let market_close = NaiveTime::from_hms_opt(16, 0, 0).unwrap();

    let start_et = date.and_time(market_open);
    let end_et = date.and_time(market_close);

    let start_utc = New_York
        .from_local_datetime(&start_et)
        .single()
        .ok_or_else(|| format!("ambiguous or invalid start time for date {date}"))?
        .with_timezone(&chrono::Utc);

    let end_utc = New_York
        .from_local_datetime(&end_et)
        .single()
        .ok_or_else(|| format!("ambiguous or invalid end time for date {date}"))?
        .with_timezone(&chrono::Utc);

    let req = bars::ListReqInit {
        limit: Some(500),
        ..Default::default()
    }
    .init(symbol, start_utc, end_utc, bars::TimeFrame::OneMinute);

    let mut last_err = String::new();
    for attempt in 0..=MAX_RETRIES {
        match client.issue::<bars::List>(&req).await {
            Ok(response) => {
                // the request spans several days, so alpaca returns overnight and
                // pre-market bars for the intermediate days. the live engine only
                // ever sees regular-hours bars (data_feed filters them), so the
                // backtest must too — otherwise indicator state, 5m/1h bucket
                // boundaries and VWAP differ at 09:31 and paper never matches.
                let candles: Vec<Candle> = response
                    .bars
                    .iter()
                    .filter(|bar| is_regular_hours(bar.time))
                    .map(|bar| Candle {
                        timestamp: bar.time,
                        open: num_to_f64(&bar.open),
                        high: num_to_f64(&bar.high),
                        low: num_to_f64(&bar.low),
                        close: num_to_f64(&bar.close),
                        volume: bar.volume as f64,
                    })
                    .collect();
                return Ok(candles);
            }
            Err(e) => {
                last_err = format!("{e}");
                if attempt < MAX_RETRIES && is_retryable_error(&last_err) {
                    let delay_ms = RETRY_BASE_MS * 2u64.pow(attempt);
                    eprintln!(
                        "  {:<6} rate limited (attempt {}/{}), retrying in {}ms...",
                        symbol,
                        attempt + 1,
                        MAX_RETRIES + 1,
                        delay_ms
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
            }
        }
    }

    Err(format!("alpaca bars request failed after {} attempts: {last_err}", MAX_RETRIES + 1))
}

/// true when `ts` falls inside regular trading hours (09:30 ≤ t < 16:00 America/New_York)
/// on a weekday. mirrors `data_feed::session_clock::is_regular_hours`.
pub fn is_regular_hours(ts: DateTime<Utc>) -> bool {
    use chrono::Datelike;
    let local = ts.with_timezone(&New_York);
    let wd = local.weekday();
    if wd == chrono::Weekday::Sat || wd == chrono::Weekday::Sun {
        return false;
    }
    let m = local.hour() * 60 + local.minute();
    (9 * 60 + 30..16 * 60).contains(&m)
}

/// return (market_open_utc, market_close_utc) for a given date.
/// market hours: 9:30 - 16:00 eastern.
pub fn market_hours_utc(date: NaiveDate) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let market_open = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
    let market_close = NaiveTime::from_hms_opt(16, 0, 0).unwrap();

    let open_utc = New_York
        .from_local_datetime(&date.and_time(market_open))
        .single()
        .ok_or_else(|| format!("ambiguous or invalid open time for date {date}"))?
        .with_timezone(&Utc);

    let close_utc = New_York
        .from_local_datetime(&date.and_time(market_close))
        .single()
        .ok_or_else(|| format!("ambiguous or invalid close time for date {date}"))?
        .with_timezone(&Utc);

    Ok((open_utc, close_utc))
}

/// fetch 1-minute bars for a single ticker across a date range from alpaca.
/// `start_date` and `end_date` are inclusive calendar dates.
pub async fn fetch_bars_range(
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<Candle>, String> {
    fetch_bars_range_feed(api_key, api_secret, symbol, start_date, end_date, None).await
}

/// like `fetch_bars_range` but with an explicit feed: `Some(Feed::IEX)` returns the bars the
/// live websocket actually sees on the free plan (~3 % of consolidated volume for mega-caps);
/// `None` = the account default (SIP for historical).
pub async fn fetch_bars_range_feed(
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    feed: Option<apca::data::v2::Feed>,
) -> Result<Vec<Candle>, String> {
    use apca::data::v2::bars;
    use apca::ApiInfo;
    use apca::Client;

    let api_info = ApiInfo::from_parts(
        "https://paper-api.alpaca.markets",
        api_key,
        api_secret,
    )
    .map_err(|e| format!("api info error: {e}"))?;

    let client = Client::new(api_info);

    let (start_utc, _) = market_hours_utc(start_date)?;
    let (_, mut end_utc) = market_hours_utc(end_date)?;
    // the free data plan refuses SIP bars from the last ~15 minutes, and a window whose end is
    // in the future is rejected outright. clamp a same-day request to now − 16 min so an
    // intraday or just-after-close fetch returns everything that is available.
    let latest_allowed = chrono::Utc::now() - chrono::Duration::minutes(16);
    if end_utc > latest_allowed {
        end_utc = latest_allowed;
    }
    if end_utc <= start_utc {
        return Ok(Vec::new());
    }

    // alpaca pages at 10,000 bars per response and the limit counts pre/post-market
    // bars too (they are filtered out below), so a multi-day request can span
    // several pages. follow `next_page_token` until it is exhausted; ignoring it
    // silently drops the tail of the range (whole days for heavily traded names).
    let mut all: Vec<Candle> = Vec::new();
    let mut page_token: Option<String> = None;
    loop {
        let req = bars::ListReqInit {
            limit: Some(10_000),
            page_token: page_token.clone(),
            feed,
            ..Default::default()
        }
        .init(symbol, start_utc, end_utc, bars::TimeFrame::OneMinute);

        let mut last_err = String::new();
        let mut page: Option<bars::Bars> = None;
        for attempt in 0..=MAX_RETRIES {
            match client.issue::<bars::List>(&req).await {
                Ok(response) => {
                    page = Some(response);
                    break;
                }
                Err(e) => {
                    last_err = format!("{e}");
                    if attempt < MAX_RETRIES && is_retryable_error(&last_err) {
                        let delay_ms = RETRY_BASE_MS * 2u64.pow(attempt);
                        eprintln!(
                            "  {:<6} rate limited (attempt {}/{}), retrying in {}ms...",
                            symbol,
                            attempt + 1,
                            MAX_RETRIES + 1,
                            delay_ms
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                }
            }
        }
        let Some(response) = page else {
            return Err(format!("alpaca bars request failed after {} attempts: {last_err}", MAX_RETRIES + 1));
        };
        // the live engine only ever sees regular-hours bars (data_feed filters them),
        // so the backtest must too — otherwise indicator state, 5m/1h bucket
        // boundaries and VWAP differ at 09:31 and paper never matches.
        all.extend(response.bars.iter().filter(|bar| is_regular_hours(bar.time)).map(|bar| Candle {
            timestamp: bar.time,
            open: num_to_f64(&bar.open),
            high: num_to_f64(&bar.high),
            low: num_to_f64(&bar.low),
            close: num_to_f64(&bar.close),
            volume: bar.volume as f64,
        }));
        match response.next_page_token {
            Some(token) if !token.is_empty() => page_token = Some(token),
            _ => break,
        }
    }
    Ok(all)
}

/// aggregate 1-minute candles into a coarser timescale using clock-aligned boundaries.
pub fn aggregate_to_timescale(candles: &[Candle], timescale: Timescale) -> Vec<Candle> {
    if candles.is_empty() {
        return Vec::new();
    }

    let bucket_minutes = match timescale {
        Timescale::OneMinute => return candles.to_vec(),
        Timescale::FiveMinute => 5,
        Timescale::OneHour => 60,
        // daily/monthly not used for intraday, but handle gracefully
        Timescale::OneDay | Timescale::OneMonth => return aggregate_all(candles),
    };

    let mut result = Vec::new();
    let mut bucket_start = align_to_bucket(candles[0].timestamp, bucket_minutes);
    let mut acc = CandleAcc::new(&candles[0], bucket_start);

    for candle in candles.iter().skip(1) {
        let this_bucket = align_to_bucket(candle.timestamp, bucket_minutes);
        if this_bucket == bucket_start {
            acc.update(candle);
        } else {
            result.push(acc.to_candle());
            bucket_start = this_bucket;
            acc = CandleAcc::new(candle, bucket_start);
        }
    }

    // finalize last bucket
    result.push(acc.to_candle());

    result
}

/// build multi-timescale backtest data from 1-minute candles.
pub fn build_backtest_data(
    one_min_candles: Vec<Candle>,
    required_timescales: &HashSet<Timescale>,
) -> BacktestData {
    let mut candles: HashMap<Timescale, Vec<Candle>> = HashMap::new();

    for &ts in required_timescales {
        let aggregated = aggregate_to_timescale(&one_min_candles, ts);
        if !aggregated.is_empty() {
            candles.insert(ts, aggregated);
        }
    }

    // determine finest timescale as primary (drives tick loop)
    let primary_timescale = if required_timescales.contains(&Timescale::OneMinute) {
        Timescale::OneMinute
    } else if required_timescales.contains(&Timescale::FiveMinute) {
        Timescale::FiveMinute
    } else if required_timescales.contains(&Timescale::OneHour) {
        Timescale::OneHour
    } else {
        // fallback: use whatever we have, preferring finest
        Timescale::OneMinute
    };

    // ensure primary timescale has data even if not explicitly required
    if let std::collections::hash_map::Entry::Vacant(e) = candles.entry(primary_timescale) {
        let aggregated = aggregate_to_timescale(&one_min_candles, primary_timescale);
        if !aggregated.is_empty() {
            e.insert(aggregated);
        }
    }

    BacktestData {
        candles,
        primary_timescale,
        cross_by_ts: None,
    }
}

/// clock-align a timestamp to a bucket boundary.
fn align_to_bucket(
    ts: chrono::DateTime<chrono::Utc>,
    bucket_minutes: u32,
) -> chrono::DateTime<chrono::Utc> {
    let minute = ts.minute();
    let aligned = (minute / bucket_minutes) * bucket_minutes;
    ts.with_minute(aligned)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
}

/// aggregate all candles into a single candle (for daily/monthly).
fn aggregate_all(candles: &[Candle]) -> Vec<Candle> {
    if candles.is_empty() {
        return Vec::new();
    }
    let mut acc = CandleAcc::new(&candles[0], candles[0].timestamp);
    for candle in candles.iter().skip(1) {
        acc.update(candle);
    }
    vec![acc.to_candle()]
}

/// OHLCV accumulator for aggregation.
struct CandleAcc {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl CandleAcc {
    fn new(candle: &Candle, start_time: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            volume: candle.volume,
            start_time,
        }
    }

    fn update(&mut self, candle: &Candle) {
        self.high = self.high.max(candle.high);
        self.low = self.low.min(candle.low);
        self.close = candle.close;
        self.volume += candle.volume;
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
            timestamp: chrono::Utc
                .with_ymd_and_hms(2025, 6, 15, hour, minute, 0)
                .unwrap(),
            open,
            high,
            low,
            close,
            volume,
        }
    }

    #[test]
    fn aggregate_one_minute_is_passthrough() {
        let candles = vec![
            make_1min_candle(14, 30, 100.0, 101.0, 99.0, 100.5, 1000.0),
            make_1min_candle(14, 31, 100.5, 102.0, 100.0, 101.0, 1500.0),
        ];
        let result = aggregate_to_timescale(&candles, Timescale::OneMinute);
        assert_eq!(result.len(), 2);
        assert!((result[0].open - 100.0).abs() < f64::EPSILON);
        assert!((result[1].close - 101.0).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_five_minute_basic() {
        // 5 candles in the 14:30 bucket
        let candles: Vec<Candle> = (0..5)
            .map(|m| make_1min_candle(14, 30 + m, 100.0, 102.0, 99.0, 101.0, 1000.0))
            .collect();
        let result = aggregate_to_timescale(&candles, Timescale::FiveMinute);
        assert_eq!(result.len(), 1);
        assert!((result[0].open - 100.0).abs() < f64::EPSILON);
        assert!((result[0].volume - 5000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_five_minute_two_buckets() {
        // 10 candles spanning two 5-min buckets: 14:30-14:34, 14:35-14:39
        let candles: Vec<Candle> = (0..10)
            .map(|m| make_1min_candle(14, 30 + m, 100.0, 102.0, 99.0, 101.0, 1000.0))
            .collect();
        let result = aggregate_to_timescale(&candles, Timescale::FiveMinute);
        assert_eq!(result.len(), 2);
        assert!((result[0].volume - 5000.0).abs() < f64::EPSILON);
        assert!((result[1].volume - 5000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_five_minute_ohlcv_correctness() {
        let candles = vec![
            make_1min_candle(14, 30, 100.0, 102.0, 99.0, 101.0, 1000.0),
            make_1min_candle(14, 31, 101.0, 105.0, 100.0, 104.0, 2000.0),
            make_1min_candle(14, 32, 104.0, 106.0, 103.0, 103.0, 1500.0),
            make_1min_candle(14, 33, 103.0, 104.0, 97.0, 98.0, 3000.0),
            make_1min_candle(14, 34, 98.0, 100.0, 96.0, 99.0, 500.0),
        ];
        let result = aggregate_to_timescale(&candles, Timescale::FiveMinute);
        assert_eq!(result.len(), 1);
        let bar = &result[0];
        assert!((bar.open - 100.0).abs() < f64::EPSILON);
        assert!((bar.high - 106.0).abs() < f64::EPSILON);
        assert!((bar.low - 96.0).abs() < f64::EPSILON);
        assert!((bar.close - 99.0).abs() < f64::EPSILON);
        assert!((bar.volume - 8000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_hourly_basic() {
        // 60 candles in hour 14
        let candles: Vec<Candle> = (0..60)
            .map(|m| make_1min_candle(14, m, 100.0, 102.0, 99.0, 101.0, 1000.0))
            .collect();
        let result = aggregate_to_timescale(&candles, Timescale::OneHour);
        assert_eq!(result.len(), 1);
        assert!((result[0].volume - 60000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_hourly_two_hours() {
        let mut candles = Vec::new();
        for m in 0..60 {
            candles.push(make_1min_candle(14, m, 100.0, 102.0, 99.0, 101.0, 1000.0));
        }
        for m in 0..30 {
            candles.push(make_1min_candle(15, m, 101.0, 103.0, 100.0, 102.0, 1500.0));
        }
        let result = aggregate_to_timescale(&candles, Timescale::OneHour);
        assert_eq!(result.len(), 2);
        assert!((result[0].volume - 60000.0).abs() < f64::EPSILON);
        assert!((result[1].volume - 45000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn aggregate_empty_returns_empty() {
        let result = aggregate_to_timescale(&[], Timescale::FiveMinute);
        assert!(result.is_empty());
    }

    #[test]
    fn build_backtest_data_selects_finest_primary() {
        let candles: Vec<Candle> = (0..10)
            .map(|m| make_1min_candle(14, 30 + m, 100.0, 102.0, 99.0, 101.0, 1000.0))
            .collect();
        let mut required = HashSet::new();
        required.insert(Timescale::FiveMinute);
        required.insert(Timescale::OneHour);

        let data = build_backtest_data(candles, &required);
        assert_eq!(data.primary_timescale, Timescale::FiveMinute);
        assert!(data.candles.contains_key(&Timescale::FiveMinute));
        assert!(data.candles.contains_key(&Timescale::OneHour));
    }

    #[test]
    fn build_backtest_data_one_minute_primary() {
        let candles: Vec<Candle> = (0..5)
            .map(|m| make_1min_candle(14, 30 + m, 100.0, 102.0, 99.0, 101.0, 1000.0))
            .collect();
        let mut required = HashSet::new();
        required.insert(Timescale::OneMinute);
        required.insert(Timescale::FiveMinute);

        let data = build_backtest_data(candles, &required);
        assert_eq!(data.primary_timescale, Timescale::OneMinute);
        assert_eq!(data.candles[&Timescale::OneMinute].len(), 5);
        assert_eq!(data.candles[&Timescale::FiveMinute].len(), 1);
    }

    #[test]
    fn align_to_bucket_five_min() {
        let ts = chrono::Utc
            .with_ymd_and_hms(2025, 6, 15, 14, 37, 15)
            .unwrap();
        let aligned = align_to_bucket(ts, 5);
        assert_eq!(aligned.minute(), 35);
        assert_eq!(aligned.second(), 0);
    }

    #[test]
    fn align_to_bucket_hourly() {
        let ts = chrono::Utc
            .with_ymd_and_hms(2025, 6, 15, 14, 45, 30)
            .unwrap();
        let aligned = align_to_bucket(ts, 60);
        assert_eq!(aligned.minute(), 0);
        assert_eq!(aligned.second(), 0);
    }

    #[test]
    fn market_hours_utc_summer() {
        // summer (EDT = UTC-4): 9:30 ET = 13:30 UTC, 16:00 ET = 20:00 UTC
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(); // a sunday but that's ok for the helper
        let (open, close) = market_hours_utc(date).unwrap();
        assert_eq!(open.hour(), 13);
        assert_eq!(open.minute(), 30);
        assert_eq!(close.hour(), 20);
        assert_eq!(close.minute(), 0);
    }

    #[test]
    fn market_hours_utc_winter() {
        // winter (EST = UTC-5): 9:30 ET = 14:30 UTC, 16:00 ET = 21:00 UTC
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let (open, close) = market_hours_utc(date).unwrap();
        assert_eq!(open.hour(), 14);
        assert_eq!(open.minute(), 30);
        assert_eq!(close.hour(), 21);
        assert_eq!(close.minute(), 0);
    }

    #[test]
    fn multi_day_candles_produce_enough_hourly_bars() {
        // simulate 3 trading days of 1-min candles (390 per day)
        // each day: 9:30-16:00 ET = 13:30-20:00 UTC in summer
        let mut candles = Vec::new();
        for day_offset in 0..3u32 {
            let day = 16 + day_offset; // june 16, 17, 18 2025
            for min in 0..390u32 {
                let total_minutes = 13 * 60 + 30 + min; // starting at 13:30 UTC
                let hour = total_minutes / 60;
                let minute = total_minutes % 60;
                candles.push(Candle {
                    timestamp: chrono::Utc
                        .with_ymd_and_hms(2025, 6, day, hour, minute, 0)
                        .unwrap(),
                    open: 100.0,
                    high: 101.0,
                    low: 99.0,
                    close: 100.5,
                    volume: 1000.0,
                });
            }
        }

        let hourly = aggregate_to_timescale(&candles, Timescale::OneHour);
        // 6.5 hours per day * 3 days = ~19.5, but partial hours round up: 7 buckets/day * 3 = 21
        assert!(
            hourly.len() >= 20,
            "expected >= 20 hourly candles from 3 days, got {}",
            hourly.len()
        );
    }
}
