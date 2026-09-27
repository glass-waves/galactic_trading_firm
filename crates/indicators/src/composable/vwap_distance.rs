use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Timelike, Utc};
use chrono_tz::US::Eastern;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

const SESSION_OPEN_MIN: u32 = 9 * 60 + 30;

#[allow(dead_code)]
pub struct VwapDistanceIndicator {
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl VwapDistanceIndicator {
    pub fn new(timescale: Timescale, instance_id: String) -> Self {
        Self {
            timescale,
            instance_id,
            scale_factor: 0.005,
        }
    }
}

fn et_date(ts: DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&Eastern).date_naive()
}

fn et_minutes(ts: DateTime<Utc>) -> u32 {
    let l = ts.with_timezone(&Eastern);
    l.hour() * 60 + l.minute()
}

/// today's regular-session bars (same eastern date as `today`, from 09:30 ET on) and whether
/// the window also holds an earlier day's bar (i.e. today is complete in the window).
fn todays_bars(candles: &[Candle], today: NaiveDate) -> (&[Candle], bool) {
    let start = candles
        .iter()
        .rposition(|c| et_date(c.timestamp) != today)
        .map(|i| i + 1)
        .unwrap_or(0);
    let bars = &candles[start..];
    let first_rth = bars
        .iter()
        .position(|c| et_minutes(c.timestamp) >= SESSION_OPEN_MIN)
        .unwrap_or(bars.len());
    (&bars[first_rth..], start > 0)
}

/// today's close-vs-running-VWAP path, oldest first: (bar timestamp, distance in percent).
/// built from the 1-minute window; when that window no longer reaches back to the open (it
/// holds 200 bars, i.e. from ~12:50 ET on) the missing prefix is filled with today's completed
/// 5-minute bars. the running VWAP uses typical price × volume like the session VWAP, so the
/// path's last point approximates `session_vwap` (exact while the 1-minute window covers the day).
fn session_path(market: &MarketState) -> Vec<(DateTime<Utc>, f64)> {
    let Some(m1) = market.candles.get(&Timescale::OneMinute) else { return Vec::new() };
    let Some(last) = m1.last() else { return Vec::new() };
    let today = et_date(last.timestamp);
    let (bars1, complete) = todays_bars(m1, today);
    if bars1.is_empty() {
        return Vec::new();
    }
    let first1 = bars1[0].timestamp;
    let mut src: Vec<&Candle> = Vec::with_capacity(bars1.len() + 80);
    let mut from_1m: &[Candle] = bars1;
    if !complete && et_minutes(first1) > SESSION_OPEN_MIN {
        if let Some(m5) = market.candles.get(&Timescale::FiveMinute) {
            // 1-minute bars from the first 5-minute boundary at/after `first1`; everything
            // earlier comes from the (completed) 5-minute buckets that start before it.
            let rem = (first1.timestamp() % 300 + 300) % 300;
            let cut = if rem == 0 { first1 } else { first1 + chrono::Duration::seconds(300 - rem) };
            let (bars5, _) = todays_bars(m5, today);
            src.extend(bars5.iter().filter(|c| c.timestamp < cut));
            from_1m = &bars1[bars1.iter().position(|c| c.timestamp >= cut).unwrap_or(bars1.len())..];
        }
    }
    src.extend(from_1m.iter());
    let mut path = Vec::with_capacity(src.len());
    let (mut pv, mut vol) = (0.0_f64, 0.0_f64);
    for c in src {
        pv += (c.high + c.low + c.close) / 3.0 * c.volume;
        vol += c.volume;
        if vol > 0.0 && pv > 0.0 {
            path.push((c.timestamp, 100.0 * (c.close / (pv / vol) - 1.0)));
        }
    }
    path
}

impl Indicator for VwapDistanceIndicator {
    fn name(&self) -> &str {
        "vwap_distance"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        1
    }

    /// score / raw_value: (last − session VWAP) / VWAP, score scaled by 0.5 % and clamped.
    /// metadata (percent unless noted), for entry-window conditions:
    /// - `vwap`, `pct_distance` (fraction), `dist_pct` — the live distance
    /// - `high_dist_pct` — the last 1-minute bar's high vs VWAP (a wick into VWAP)
    /// - `min_dist_pct_today` / `max_dist_pct_today` — deepest close below / above the running
    ///   VWAP so far today (see `session_path`)
    /// - `rebound_since_min_pct` — the highest close-vs-VWAP after that low, excluding the
    ///   current bar (equals the low when nothing followed it): `<= −0.15` means the current
    ///   bar is the first to come back within 0.15 % of VWAP since the low ("first retest")
    /// - `mins_since_min` — minutes from that low to the current bar
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        if market.session_vwap.abs() < f64::EPSILON {
            return None;
        }

        let pct_distance = (market.last_price - market.session_vwap) / market.session_vwap;
        let score = (pct_distance / self.scale_factor).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("vwap".to_string(), market.session_vwap);
        metadata.insert("pct_distance".to_string(), pct_distance);
        metadata.insert("dist_pct".to_string(), 100.0 * pct_distance);

        if let Some(last1) = market.candles.get(&Timescale::OneMinute).and_then(|c| c.last()) {
            metadata.insert("high_dist_pct".to_string(), 100.0 * (last1.high / market.session_vwap - 1.0));
        }

        let path = session_path(market);
        if let Some((imin, &(tmin, dmin))) = path
            .iter()
            .enumerate()
            .min_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap_or(std::cmp::Ordering::Equal))
        {
            let dmax = path.iter().map(|p| p.1).fold(f64::MIN, f64::max);
            let n = path.len();
            let rebound = if imin + 1 < n.saturating_sub(1) {
                path[imin + 1..n - 1].iter().map(|p| p.1).fold(f64::MIN, f64::max)
            } else {
                dmin
            };
            let (tlast, _) = path[n - 1];
            metadata.insert("min_dist_pct_today".to_string(), dmin);
            metadata.insert("max_dist_pct_today".to_string(), dmax);
            metadata.insert("rebound_since_min_pct".to_string(), rebound);
            metadata.insert("mins_since_min".to_string(), (tlast - tmin).num_minutes() as f64);
        }

        Some(IndicatorOutput {
            score,
            raw_value: pct_distance,
            metadata,
        })
    }
}

pub fn vwap_distance_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(VwapDistanceIndicator::new(config.timescale, config.instance_id.clone()))
}
