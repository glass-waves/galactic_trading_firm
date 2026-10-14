//! point-in-time split un-adjustment for cached (split-adjusted) bars.
//!
//! alpaca's split-adjusted history rebases a ticker's *entire* series to the current
//! (fetch-time) share count, so a window that only spans dates before a later split still
//! reads post-split prices (NVDA at $40-90 instead of $400-900 before its 2024-06-10 10:1
//! split). the live engine never saw that: on any day D before the split it only knew the
//! basis in force on D. replaying day D from the adjusted cache without correction is a
//! different history than the one that produced the trader's live record — see
//! `docs/research/2026-10-14_split_adjustment.md`.
//!
//! `unadjust_to_date` restores that point-in-time basis: for every split that has not
//! happened yet as of the replay date, it multiplies prices (and divides volume) back up by
//! the split ratio for every bar dated before that split. splits already in the past as of
//! the replay date are left alone — the cache already reflects them correctly for that day.

use std::fs;

use chrono::NaiveDate;
use chrono_tz::America::New_York;
use types::market::Candle;

/// one stock split: `ratio` new shares per old share (e.g. a 10:1 split is `ratio = 10.0`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplitRecord {
    pub ticker: String,
    pub date: NaiveDate,
    pub ratio: f64,
}

/// load `ticker,date,ratio` rows from `path`. blank lines and `#`-prefixed comments are
/// skipped; a `ticker,date,ratio` header line (case-insensitive) is skipped if present.
pub fn load_splits_csv(path: &str) -> Result<Vec<SplitRecord>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let mut out = Vec::new();
    for (i, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if i == 0 && line.to_lowercase() == "ticker,date,ratio" {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 3 {
            return Err(format!(
                "{path}:{}: expected 3 columns (ticker,date,ratio), got {}",
                i + 1,
                parts.len()
            ));
        }
        let ticker = parts[0].trim().to_string();
        let date = NaiveDate::parse_from_str(parts[1].trim(), "%Y-%m-%d")
            .map_err(|e| format!("{path}:{}: bad date '{}': {e}", i + 1, parts[1]))?;
        let ratio: f64 = parts[2]
            .trim()
            .parse()
            .map_err(|e| format!("{path}:{}: bad ratio '{}': {e}", i + 1, parts[2]))?;
        if ratio <= 0.0 {
            return Err(format!("{path}:{}: ratio must be positive, got {ratio}", i + 1));
        }
        out.push(SplitRecord { ticker, date, ratio });
    }
    Ok(out)
}

/// re-base `candles` (already split-adjusted to "today") onto the price basis in force on
/// `replay_date`, in place.
///
/// for every split on `ticker` whose date is strictly after `replay_date` (it had not
/// happened yet, as of that day), every bar dated before the split gets multiplied back up:
/// price fields × ratio, volume ÷ ratio. bars on or after the split date, and splits whose
/// date is on or before `replay_date`, are left untouched — the cache already matches the
/// live basis for them. splits are applied oldest-qualifying-first; this matters only when
/// two qualifying splits straddle a bar (both get applied, compounding).
///
/// returns the qualifying `(split_date, ratio)` pairs actually applied, for logging; empty
/// when none of `splits` apply to `ticker`/`replay_date`.
pub fn unadjust_to_date(
    candles: &mut [Candle],
    ticker: &str,
    replay_date: NaiveDate,
    splits: &[SplitRecord],
) -> Vec<(NaiveDate, f64)> {
    let mut applied = Vec::new();
    let mut qualifying: Vec<&SplitRecord> = splits
        .iter()
        .filter(|s| s.ticker == ticker && s.date > replay_date)
        .collect();
    qualifying.sort_by_key(|s| s.date);

    for split in qualifying {
        applied.push((split.date, split.ratio));
        for c in candles.iter_mut() {
            let bar_date = c.timestamp.with_timezone(&New_York).date_naive();
            if bar_date < split.date {
                c.open *= split.ratio;
                c.high *= split.ratio;
                c.low *= split.ratio;
                c.close *= split.ratio;
                c.volume /= split.ratio;
            }
        }
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn candle(y: i32, m: u32, d: u32, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp: Utc.with_ymd_and_hms(y, m, d, 14, 30, 0).unwrap(),
            open: close,
            high: close,
            low: close,
            close,
            volume,
        }
    }

    #[test]
    fn split_before_replay_date_leaves_bars_untouched() {
        // NVDA split 2024-06-10; replaying 2024-07-01 (after the split) should not touch
        // anything — the cache basis already matches what the live engine saw that day.
        let splits = vec![SplitRecord {
            ticker: "NVDA".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
            ratio: 10.0,
        }];
        let mut candles = vec![
            candle(2024, 5, 20, 95.0, 1000.0),
            candle(2024, 7, 1, 120.0, 1000.0),
        ];
        let replay_date = NaiveDate::from_ymd_opt(2024, 7, 1).unwrap();
        let applied = unadjust_to_date(&mut candles, "NVDA", replay_date, &splits);
        assert!(applied.is_empty());
        assert!((candles[0].close - 95.0).abs() < f64::EPSILON);
        assert!((candles[1].close - 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn split_after_replay_date_scales_pre_split_bars() {
        // replaying 2024-05-20, before the 2024-06-10 10:1 split: the cache's $95 is the
        // post-split basis; the live engine on 2024-05-20 saw ~$950.
        let splits = vec![SplitRecord {
            ticker: "NVDA".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
            ratio: 10.0,
        }];
        let mut candles = vec![candle(2024, 5, 20, 95.0, 10_000.0)];
        let replay_date = NaiveDate::from_ymd_opt(2024, 5, 20).unwrap();
        let applied = unadjust_to_date(&mut candles, "NVDA", replay_date, &splits);
        assert_eq!(applied, vec![(NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(), 10.0)]);
        assert!((candles[0].open - 950.0).abs() < 1e-9);
        assert!((candles[0].high - 950.0).abs() < 1e-9);
        assert!((candles[0].low - 950.0).abs() < 1e-9);
        assert!((candles[0].close - 950.0).abs() < 1e-9);
        assert!((candles[0].volume - 1_000.0).abs() < 1e-9);
    }

    #[test]
    fn split_on_replay_date_itself_does_not_qualify() {
        // `date > replay_date` is strict: a split dated exactly on the replay date has
        // already happened by the close of that session on the live basis.
        let splits = vec![SplitRecord {
            ticker: "NVDA".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
            ratio: 10.0,
        }];
        let mut candles = vec![candle(2024, 6, 10, 95.0, 1000.0)];
        let replay_date = NaiveDate::from_ymd_opt(2024, 6, 10).unwrap();
        let applied = unadjust_to_date(&mut candles, "NVDA", replay_date, &splits);
        assert!(applied.is_empty());
        assert!((candles[0].close - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn two_splits_straddling_replay_date_only_applies_the_later_one() {
        // synthetic two-split ticker: S1 2023-01-10, S2 2023-06-10, replaying 2023-03-01
        // (between them). S1 already happened by the replay date so its factor must not
        // apply; S2 has not, so every bar in the window (both before and after S1) gets S2's
        // factor only.
        let splits = vec![
            SplitRecord { ticker: "ZZZ".into(), date: NaiveDate::from_ymd_opt(2023, 1, 10).unwrap(), ratio: 2.0 },
            SplitRecord { ticker: "ZZZ".into(), date: NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(), ratio: 3.0 },
        ];
        let mut candles = vec![
            candle(2023, 1, 1, 100.0, 300.0),  // before S1
            candle(2023, 2, 1, 100.0, 300.0),  // between S1 and replay date
        ];
        let replay_date = NaiveDate::from_ymd_opt(2023, 3, 1).unwrap();
        let applied = unadjust_to_date(&mut candles, "ZZZ", replay_date, &splits);
        assert_eq!(applied, vec![(NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(), 3.0)]);
        assert!((candles[0].close - 300.0).abs() < 1e-9);
        assert!((candles[0].volume - 100.0).abs() < 1e-9);
        assert!((candles[1].close - 300.0).abs() < 1e-9);
        assert!((candles[1].volume - 100.0).abs() < 1e-9);
    }

    #[test]
    fn two_splits_both_after_replay_date_compound() {
        let splits = vec![
            SplitRecord { ticker: "ZZZ".into(), date: NaiveDate::from_ymd_opt(2023, 1, 10).unwrap(), ratio: 2.0 },
            SplitRecord { ticker: "ZZZ".into(), date: NaiveDate::from_ymd_opt(2023, 6, 10).unwrap(), ratio: 3.0 },
        ];
        let mut candles = vec![candle(2022, 12, 1, 100.0, 600.0)];
        let replay_date = NaiveDate::from_ymd_opt(2022, 12, 1).unwrap();
        let applied = unadjust_to_date(&mut candles, "ZZZ", replay_date, &splits);
        assert_eq!(applied.len(), 2);
        // 100 * 2 * 3 = 600; volume 600 / 2 / 3 = 100
        assert!((candles[0].close - 600.0).abs() < 1e-9);
        assert!((candles[0].volume - 100.0).abs() < 1e-9);
    }

    #[test]
    fn other_tickers_are_not_touched() {
        let splits = vec![SplitRecord {
            ticker: "NVDA".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, 10).unwrap(),
            ratio: 10.0,
        }];
        let mut candles = vec![candle(2024, 5, 20, 95.0, 1000.0)];
        let replay_date = NaiveDate::from_ymd_opt(2024, 5, 20).unwrap();
        let applied = unadjust_to_date(&mut candles, "SPY", replay_date, &splits);
        assert!(applied.is_empty());
        assert!((candles[0].close - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn load_splits_csv_parses_header_and_comments() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("splits_test_{}.csv", std::process::id()));
        fs::write(
            &path,
            "ticker,date,ratio\n# comment\nNVDA,2024-06-10,10\n\nAMZN,2022-06-06,20\n",
        )
        .unwrap();
        let records = load_splits_csv(path.to_str().unwrap()).unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].ticker, "NVDA");
        assert_eq!(records[0].date, NaiveDate::from_ymd_opt(2024, 6, 10).unwrap());
        assert!((records[0].ratio - 10.0).abs() < f64::EPSILON);
        assert_eq!(records[1].ticker, "AMZN");
    }

    #[test]
    fn load_splits_csv_rejects_bad_column_count() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("splits_bad_{}.csv", std::process::id()));
        fs::write(&path, "NVDA,2024-06-10\n").unwrap();
        let err = load_splits_csv(path.to_str().unwrap());
        fs::remove_file(&path).ok();
        assert!(err.is_err());
    }
}
