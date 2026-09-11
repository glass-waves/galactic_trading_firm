//! exchange-clock helpers. all session logic in this crate uses US/Eastern.

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc, Weekday};
use chrono_tz::US::Eastern;

/// regular trading hours: 09:30 (inclusive) to 16:00 (exclusive), monday–friday, US/Eastern.
pub const RTH_OPEN_MINUTES: u32 = 9 * 60 + 30;
pub const RTH_CLOSE_MINUTES: u32 = 16 * 60;

/// minutes since midnight, US/Eastern.
pub fn eastern_minutes(ts: DateTime<Utc>) -> u32 {
    let local = ts.with_timezone(&Eastern);
    local.hour() * 60 + local.minute()
}

/// calendar date in US/Eastern.
pub fn eastern_date(ts: DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&Eastern).date_naive()
}

/// true when `ts` falls inside regular trading hours on a weekday.
/// holidays are not modelled — no bars arrive on those days anyway.
pub fn is_regular_hours(ts: DateTime<Utc>) -> bool {
    let local = ts.with_timezone(&Eastern);
    let wd = local.weekday();
    if wd == Weekday::Sat || wd == Weekday::Sun {
        return false;
    }
    let m = local.hour() * 60 + local.minute();
    (RTH_OPEN_MINUTES..RTH_CLOSE_MINUTES).contains(&m)
}

/// "HH:MM" → minutes since midnight.
pub fn parse_hm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    Some(h.trim().parse::<u32>().ok()? * 60 + m.trim().parse::<u32>().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn rth_boundaries_in_winter_and_summer() {
        // 2024-01-16 (tue): 14:30 UTC = 09:30 EST → open; 14:29 → closed; 21:00 UTC = 16:00 → closed
        assert!(is_regular_hours(Utc.with_ymd_and_hms(2024, 1, 16, 14, 30, 0).unwrap()));
        assert!(!is_regular_hours(Utc.with_ymd_and_hms(2024, 1, 16, 14, 29, 0).unwrap()));
        assert!(is_regular_hours(Utc.with_ymd_and_hms(2024, 1, 16, 20, 59, 0).unwrap()));
        assert!(!is_regular_hours(Utc.with_ymd_and_hms(2024, 1, 16, 21, 0, 0).unwrap()));
        // 2024-07-16 (tue): 13:30 UTC = 09:30 EDT
        assert!(is_regular_hours(Utc.with_ymd_and_hms(2024, 7, 16, 13, 30, 0).unwrap()));
        assert!(!is_regular_hours(Utc.with_ymd_and_hms(2024, 7, 16, 13, 29, 0).unwrap()));
    }

    #[test]
    fn weekends_are_closed() {
        assert!(!is_regular_hours(Utc.with_ymd_and_hms(2024, 7, 13, 15, 0, 0).unwrap())); // sat
        assert!(!is_regular_hours(Utc.with_ymd_and_hms(2024, 7, 14, 15, 0, 0).unwrap())); // sun
    }

    #[test]
    fn eastern_date_rolls_at_midnight_eastern_not_utc() {
        // 03:30 UTC on jul 17 is 23:30 EDT on jul 16
        let d = eastern_date(Utc.with_ymd_and_hms(2024, 7, 17, 3, 30, 0).unwrap());
        assert_eq!(d, NaiveDate::from_ymd_opt(2024, 7, 16).unwrap());
    }

    #[test]
    fn parse_hm_works() {
        assert_eq!(parse_hm("11:55"), Some(715));
        assert_eq!(parse_hm(""), None);
        assert_eq!(parse_hm("x"), None);
    }
}
