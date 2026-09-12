//! calendar exclusion flags (research, 2026-09-12). score −1.0 on a flagged eastern date,
//! +1.0 otherwise, so `indicator_min >= 0` in a window means "not an event day".
//! `dates`: list of "YYYY-MM-DD"; `before_days` / `after_days` widen the flag window.
//! metadata: `days_to_event` (signed, nearest flagged date; 999 if none within ±30).
//! the built-in `fomc` set is the decision day (second day) of every scheduled meeting
//! 2022–2026 (federalreserve.gov calendars); pass `"dates": ["fomc"]` to use it.
use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use chrono_tz::US::Eastern;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

pub const FOMC_DECISION_DAYS: &[&str] = &[
    "2022-01-26", "2022-03-16", "2022-05-04", "2022-06-15", "2022-07-27", "2022-09-21", "2022-11-02", "2022-12-14",
    "2023-02-01", "2023-03-22", "2023-05-03", "2023-06-14", "2023-07-26", "2023-09-20", "2023-11-01", "2023-12-13",
    "2024-01-31", "2024-03-20", "2024-05-01", "2024-06-12", "2024-07-31", "2024-09-18", "2024-11-07", "2024-12-18",
    "2025-01-29", "2025-03-19", "2025-05-07", "2025-06-18", "2025-07-30", "2025-09-17", "2025-10-29", "2025-12-10",
    "2026-01-28", "2026-03-18", "2026-04-29", "2026-06-17", "2026-07-29", "2026-09-16", "2026-10-28", "2026-12-09",
];

pub struct EventCalendar {
    dates: Vec<NaiveDate>,
    before_days: i64,
    after_days: i64,
    timescale: Timescale,
}

impl Indicator for EventCalendar {
    fn name(&self) -> &str {
        "event_calendar"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        1
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let today = market.timestamp.with_timezone(&Eastern).date_naive();
        let mut nearest: i64 = 999;
        let mut flagged = false;
        for d in &self.dates {
            let delta = (*d - today).num_days(); // positive = event ahead
            if delta.abs() < nearest.abs() {
                nearest = delta;
            }
            if delta >= -self.after_days && delta <= self.before_days {
                flagged = true;
            }
        }
        let mut metadata = HashMap::new();
        metadata.insert("days_to_event".to_string(), nearest as f64);
        metadata.insert("weekday".to_string(), today.weekday().num_days_from_monday() as f64);
        let score = if flagged { -1.0 } else { 1.0 };
        Some(IndicatorOutput { score, raw_value: score, metadata })
    }
}

pub fn event_calendar_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let mut dates: Vec<NaiveDate> = Vec::new();
    if let Some(arr) = config.params.get("dates").and_then(|v| v.as_array()) {
        for v in arr {
            match v.as_str() {
                Some("fomc") => dates.extend(FOMC_DECISION_DAYS.iter().filter_map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())),
                Some(s) => {
                    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                        dates.push(d);
                    }
                }
                None => {}
            }
        }
    }
    let before_days = config.params.get("before_days").and_then(|v| v.as_i64()).unwrap_or(0);
    let after_days = config.params.get("after_days").and_then(|v| v.as_i64()).unwrap_or(0);
    Box::new(EventCalendar { dates, before_days, after_days, timescale: config.timescale })
}
