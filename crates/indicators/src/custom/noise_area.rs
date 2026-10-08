//! noise area (research, 2026-10-07): the intraday-momentum boundary of Zarattini, Aziz & Barbon,
//! "Beat the Market: An Effective Intraday Momentum Strategy for S&P500 ETF (SPY)" (SFI 24-97, 2024).
//!
//! for each time of day `t`, sigma(t) = the average over the last `lookback_sessions` sessions of
//! |close at t / that session's open − 1|. the boundaries are
//!   upper = max(open, prior close) · (1 + band_mult · sigma(t))
//!   lower = min(open, prior close) · (1 − band_mult · sigma(t))
//! (the paper's gap adjustment; `gap_adjust: false` uses today's open on both sides). price outside
//! the band = the day's move exceeds normal noise → trade in its direction.
//!
//! stateless: sigma comes from the HOURLY window (the live/replay window holds 200 candles ≈ 28
//! sessions; the 1m window holds only 200 bars), so it is exact at the hourly marks 10:00, 11:00 …
//! 16:00 (each clock-aligned hourly candle closes on the hour; the first one is 09:30–10:00) and
//! interpolated linearly in sqrt(minutes since the open) in between (and from 0 at 09:30), the
//! random-walk shape of the move-from-open curve. a session counts for a mark only if its first
//! hourly candle is the 09:00 bucket (a full start) and it has that mark (half days stop at 13:00).
//! needs `min_sessions` such sessions per mark (default `lookback_sessions`); `None` otherwise —
//! the replay's `--lookback-days` (calendar days) must cover them.
//!
//! weight 0 by design: entry windows read the metadata (`{instance_id}.{key}`):
//! - `pos`: 0 inside [ref_lo, ref_hi]; above ref_hi (price/ref_hi − 1)/(band_mult·sigma), below
//!   ref_lo (price/ref_lo − 1)/(band_mult·sigma). pos ≥ 1 ⇔ price ≥ upper, pos ≤ −1 ⇔ price ≤ lower.
//! - `upper`, `lower`, `avg_move_pct` (sigma(t) in %), `day_move_pct` (sigma at the 16:00 mark in %:
//!   the recent average open-to-close move, a realised-volatility gauge), `sessions` (count at the
//!   current mark).
//! - `decision`: 1 on the first 1m bar at or after a decision minute (every `decision_every_min`
//!   minutes since 09:30, counting the current bar, so the 09:59 bar — close at 10:00 — is minute
//!   30), at most `decision_tolerance_min` minutes late (IEX misses ~2 % of SPY minutes); else 0.
//! - `vwap_pct`: price vs session VWAP in %; `ret_open_pct`: price vs today's open in %;
//!   `ret_first30_pct` (from minute 30 on): the 09:30–10:00 return in % (Gao, Han, Li & Zhou 2018);
//!   `ret_hour_pct`: return since the start of the current hourly candle in % (at 15:29 it is the
//!   15:00–15:30 return).
//!
//! score = clamp(pos / 2, −1, 1); raw_value = pos.
use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Timelike, Utc};
use chrono_tz::US::Eastern;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

const OPEN_MIN: i64 = 9 * 60 + 30;
/// hourly marks: minutes since 09:30 at which the 09:00 … 15:00 hourly candles close.
const MARKS: [f64; 7] = [30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 390.0];

fn et(ts: DateTime<Utc>) -> (NaiveDate, u32, u32) {
    let l = ts.with_timezone(&Eastern);
    (l.date_naive(), l.hour(), l.minute())
}

/// minutes since 09:30 ET including the current 1m bar (the 09:30 bar → 1).
fn elapsed_min(ts: DateTime<Utc>) -> i64 {
    let (_, h, m) = et(ts);
    (h * 60 + m) as i64 - OPEN_MIN + 1
}

/// one past session from the hourly window: open and |close/open − 1| per mark (None if absent).
struct Session {
    moves: [Option<f64>; 7],
}

pub struct NoiseArea {
    lookback_sessions: usize,
    min_sessions: usize,
    band_mult: f64,
    gap_adjust: bool,
    decision_every_min: i64,
    decision_tolerance_min: i64,
    timescale: Timescale,
}

impl NoiseArea {
    /// split the hourly window into ET-date groups (oldest first).
    fn groups(hourly: &[Candle]) -> Vec<(NaiveDate, Vec<&Candle>)> {
        let mut out: Vec<(NaiveDate, Vec<&Candle>)> = Vec::new();
        for c in hourly {
            let (d, _, _) = et(c.timestamp);
            match out.last_mut() {
                Some((ld, v)) if *ld == d => v.push(c),
                _ => out.push((d, vec![c])),
            }
        }
        out
    }

    fn session_from(candles: &[&Candle]) -> Option<Session> {
        let first = candles.first()?;
        let (_, h0, _) = et(first.timestamp);
        if h0 != 9 || first.open <= 0.0 {
            return None; // truncated start (window edge) or no 09:30 bucket
        }
        let mut moves = [None; 7];
        for c in candles {
            let (_, h, _) = et(c.timestamp);
            if (9..=15).contains(&h) {
                moves[(h - 9) as usize] = Some((c.close / first.open - 1.0).abs());
            }
        }
        Some(Session { moves })
    }

    /// sigma at each mark over the most recent sessions that have it (newest first).
    fn sigmas(&self, past: &[Session]) -> [Option<(f64, usize)>; 7] {
        let mut out = [None; 7];
        for (k, slot) in out.iter_mut().enumerate() {
            let vals: Vec<f64> = past.iter().rev().filter_map(|s| s.moves[k]).take(self.lookback_sessions).collect();
            if vals.len() >= self.min_sessions && !vals.is_empty() {
                *slot = Some((vals.iter().sum::<f64>() / vals.len() as f64, vals.len()));
            }
        }
        out
    }

    /// sigma at `e` minutes since the open, sqrt-time interpolated between marks.
    fn sigma_at(sig: &[Option<(f64, usize)>; 7], e: f64) -> Option<(f64, usize)> {
        let e = e.clamp(1.0, 390.0);
        let k = MARKS.iter().position(|m| e <= *m)?;
        let (sb, nb) = sig[k]?;
        let (a, sa) = if k == 0 { (0.0, 0.0) } else { (MARKS[k - 1], sig[k - 1]?.0) };
        let b = MARKS[k];
        let w = (e.sqrt() - a.sqrt()) / (b.sqrt() - a.sqrt());
        Some((sa + (sb - sa) * w, nb))
    }

    /// 1 if the current bar is the first bar at/after a decision minute, within the tolerance.
    fn decision(&self, market: &MarketState, e: i64) -> f64 {
        let every = self.decision_every_min.max(1);
        let d = (e / every) * every; // latest decision minute <= e
        if d < every || e - d > self.decision_tolerance_min {
            return 0.0;
        }
        if e == d {
            return 1.0;
        }
        // late by 1..=tol minutes: only if no bar of today's session fell on [d, e)
        let prev = market
            .candles
            .get(&Timescale::OneMinute)
            .and_then(|c| if c.len() >= 2 { Some(&c[c.len() - 2]) } else { None });
        match prev {
            Some(p) if et(p.timestamp).0 == et(market.timestamp).0 && elapsed_min(p.timestamp) >= d => 0.0,
            _ => 1.0,
        }
    }
}

impl Indicator for NoiseArea {
    fn name(&self) -> &str {
        "noise_area"
    }
    fn timescale(&self) -> Timescale {
        self.timescale
    }
    fn min_lookback(&self) -> usize {
        1
    }
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let hourly = market.candles.get(&Timescale::OneHour)?;
        let price = market.last_price;
        let e = elapsed_min(market.timestamp);
        if !(1..=390).contains(&e) || !price.is_finite() || price <= 0.0 {
            return None;
        }
        let (today, _, _) = et(market.timestamp);
        let groups = Self::groups(hourly);
        let (last_date, today_c) = groups.last()?;
        if *last_date != today {
            return None;
        }
        let first = today_c.first()?;
        if et(first.timestamp).1 != 9 || first.open <= 0.0 {
            return None;
        }
        let open = first.open;
        let prev_close = groups.len().checked_sub(2).and_then(|i| groups[i].1.last()).map(|c| c.close)?;
        let past: Vec<Session> = groups[..groups.len() - 1].iter().filter_map(|(_, v)| Self::session_from(v)).collect();
        let sig = self.sigmas(&past);
        let (sigma, n) = Self::sigma_at(&sig, e as f64)?;
        let band = self.band_mult * sigma;
        if !(band.is_finite() && band > 0.0) {
            return None;
        }
        let (ref_hi, ref_lo) = if self.gap_adjust { (open.max(prev_close), open.min(prev_close)) } else { (open, open) };
        let pos = if price > ref_hi {
            (price / ref_hi - 1.0) / band
        } else if price < ref_lo {
            (price / ref_lo - 1.0) / band
        } else {
            0.0
        };

        let mut md = HashMap::new();
        md.insert("pos".to_string(), pos);
        md.insert("upper".to_string(), ref_hi * (1.0 + band));
        md.insert("lower".to_string(), ref_lo * (1.0 - band));
        md.insert("avg_move_pct".to_string(), sigma * 100.0);
        if let Some((s, _)) = sig[6] {
            md.insert("day_move_pct".to_string(), s * 100.0);
        }
        md.insert("sessions".to_string(), n as f64);
        md.insert("decision".to_string(), self.decision(market, e));
        if market.session_vwap.is_finite() && market.session_vwap > 0.0 {
            md.insert("vwap_pct".to_string(), (price / market.session_vwap - 1.0) * 100.0);
        }
        md.insert("ret_open_pct".to_string(), (price / open - 1.0) * 100.0);
        if e >= 30 {
            // the 09:00 bucket holds 09:30–09:59; at minute 30 it is complete (its close = price)
            md.insert("ret_first30_pct".to_string(), (first.close / open - 1.0) * 100.0);
        }
        if let Some(cur) = today_c.last() {
            if cur.open > 0.0 {
                md.insert("ret_hour_pct".to_string(), (price / cur.open - 1.0) * 100.0);
            }
        }
        Some(IndicatorOutput { score: (pos / 2.0).clamp(-1.0, 1.0), raw_value: pos, metadata: md })
    }
}

pub fn noise_area_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let p = &config.params;
    let u = |k: &str, d: u64| p.get(k).and_then(|v| v.as_u64()).unwrap_or(d);
    let lookback = u("lookback_sessions", 14).max(1) as usize;
    Box::new(NoiseArea {
        lookback_sessions: lookback,
        min_sessions: (u("min_sessions", lookback as u64).max(1) as usize).min(lookback),
        band_mult: p.get("band_mult").and_then(|v| v.as_f64()).unwrap_or(1.0),
        gap_adjust: p.get("gap_adjust").and_then(|v| v.as_bool()).unwrap_or(true),
        decision_every_min: u("decision_every_min", 30) as i64,
        decision_tolerance_min: u("decision_tolerance_min", 2) as i64,
        timescale: config.timescale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// hourly candle for 2024-06-<day> (EDT, UTC−4) at ET hour `h`.
    fn hbar(day: u32, h: u32, open: f64, close: f64) -> Candle {
        let ts = Utc.with_ymd_and_hms(2024, 6, day, h + 4, if h == 9 { 30 } else { 0 }, 0).unwrap();
        Candle { timestamp: ts, open, high: open.max(close), low: open.min(close), close, volume: 1.0 }
    }
    fn mbar(day: u32, h: u32, m: u32, close: f64) -> Candle {
        let ts = Utc.with_ymd_and_hms(2024, 6, day, h + 4, m, 0).unwrap();
        Candle { timestamp: ts, open: close, high: close, low: close, close, volume: 1.0 }
    }
    /// a full past session opening at 100 whose hourly closes sit `mv`·100 % above the open.
    fn session(day: u32, mv: f64, last_close: f64) -> Vec<Candle> {
        let mut v: Vec<Candle> = (9..15).map(|h| hbar(day, h, 100.0, 100.0 * (1.0 + mv))).collect();
        v.push(hbar(day, 15, 100.0, last_close));
        v
    }
    fn state(hourly: Vec<Candle>, minute: Vec<Candle>, vwap: f64) -> MarketState {
        let last = minute.last().unwrap().clone();
        let mut m = HashMap::new();
        m.insert(Timescale::OneHour, hourly);
        m.insert(Timescale::OneMinute, minute);
        MarketState {
            last_price: last.close, bid: last.close, ask: last.close, timestamp: last.timestamp, candles: m,
            spread: 0.0, session_vwap: vwap, session_volume: 0.0, position_context: None, session_progress: None,
            entries_blocked: false, total_deployed_capital: None, total_initial_capital: None, index_return: None,
            cross_ticker_correlation: None, cross: None,
        }
    }
    fn ind(lookback: usize) -> NoiseArea {
        NoiseArea { lookback_sessions: lookback, min_sessions: lookback, band_mult: 1.0, gap_adjust: true,
                    decision_every_min: 30, decision_tolerance_min: 2, timescale: Timescale::OneMinute }
    }
    /// two past sessions (moves 1 % and 3 % at every mark, closes 101 / 100), today open 100.
    fn hist() -> Vec<Candle> {
        let mut h = session(3, 0.01, 101.0);
        h.extend(session(4, 0.03, 100.0));
        h
    }

    #[test]
    fn band_at_an_hourly_mark_is_the_mean_absolute_move() {
        // 09:59 bar (minute 30, close 10:00 price): sigma = (1 % + 3 %)/2 = 2 % → upper 102, lower 98
        // (the last session's 16:00 close is 100, so no gap: ref = open = 100 on both sides)
        let mut h = hist();
        h.push(hbar(5, 9, 100.0, 102.5));
        let o = ind(2).compute(&state(h, vec![mbar(5, 9, 59, 102.5)], 101.0)).unwrap();
        assert!((o.metadata["avg_move_pct"] - 2.0).abs() < 1e-9);
        assert!((o.metadata["upper"] - 102.0).abs() < 1e-9 && (o.metadata["lower"] - 98.0).abs() < 1e-9);
        assert!((o.metadata["pos"] - 1.25).abs() < 1e-9, "2.5 % above the open / 2 % = 1.25");
        assert_eq!(o.metadata["decision"], 1.0);
        assert!((o.metadata["ret_first30_pct"] - 2.5).abs() < 1e-9);
        assert!((o.metadata["vwap_pct"] - 100.0 * (102.5 / 101.0 - 1.0)).abs() < 1e-9);
        assert!(o.score > 0.6);
    }

    #[test]
    fn sigma_interpolates_in_sqrt_time_and_inside_the_band_is_zero() {
        // 09:44 bar = minute 15: sigma = 2 % · sqrt(15/30)
        let mut h = hist();
        h.push(hbar(5, 9, 100.0, 100.5));
        let o = ind(2).compute(&state(h, vec![mbar(5, 9, 44, 100.5)], 100.0)).unwrap();
        assert!((o.metadata["avg_move_pct"] - 2.0 * 0.5f64.sqrt()).abs() < 1e-9);
        let pos = o.metadata["pos"];
        assert!((pos - 0.5 / (2.0 * 0.5f64.sqrt())).abs() < 1e-9 && pos < 1.0, "0.5 % is inside a 1.41 % band: {pos}");
        assert_eq!(o.metadata["decision"], 0.0);
        assert!(!o.metadata.contains_key("ret_first30_pct"));
    }

    #[test]
    fn gap_widens_the_reference_on_the_gap_side() {
        // previous close 104 (gap down to 100): upper uses 104, lower uses 100
        let mut h = session(3, 0.01, 101.0);
        h.extend(session(4, 0.03, 104.0));
        h.push(hbar(5, 9, 100.0, 103.0));
        let o = ind(2).compute(&state(h, vec![mbar(5, 9, 59, 103.0)], 101.0)).unwrap();
        assert!((o.metadata["upper"] - 104.0 * 1.02).abs() < 1e-9);
        assert!((o.metadata["lower"] - 98.0).abs() < 1e-9);
        assert_eq!(o.metadata["pos"], 0.0, "103 is above the open but below the prior close");
        let mut no_gap = ind(2);
        no_gap.gap_adjust = false;
        let mut h2 = session(3, 0.01, 101.0);
        h2.extend(session(4, 0.03, 104.0));
        h2.push(hbar(5, 9, 100.0, 103.0));
        let o2 = no_gap.compute(&state(h2, vec![mbar(5, 9, 59, 103.0)], 101.0)).unwrap();
        assert!((o2.metadata["pos"] - 1.5).abs() < 1e-9);
    }

    #[test]
    fn short_side_and_negative_pos() {
        let mut h = hist();
        h.push(hbar(5, 9, 100.0, 97.0));
        let o = ind(2).compute(&state(h, vec![mbar(5, 9, 59, 97.0)], 99.0)).unwrap();
        assert!((o.metadata["pos"] + 1.5).abs() < 1e-9);
        assert!(o.score < -0.7);
    }

    #[test]
    fn needs_enough_full_sessions() {
        let mut h = hist();
        h.push(hbar(5, 9, 100.0, 102.5));
        assert!(ind(3).compute(&state(h.clone(), vec![mbar(5, 9, 59, 102.5)], 101.0)).is_none());
        // a truncated first session (window edge: starts at the 11:00 bucket) does not count
        let mut t: Vec<Candle> = session(3, 0.01, 101.0).into_iter().skip(2).collect();
        t.extend(session(4, 0.03, 100.0));
        t.push(hbar(5, 9, 100.0, 102.5));
        assert!(ind(2).compute(&state(t, vec![mbar(5, 9, 59, 102.5)], 101.0)).is_none());
        // with min_sessions 1 the one full session is enough
        let mut lax = ind(2);
        lax.min_sessions = 1;
        let mut t2: Vec<Candle> = session(3, 0.01, 101.0).into_iter().skip(2).collect();
        t2.extend(session(4, 0.03, 100.0));
        t2.push(hbar(5, 9, 100.0, 102.5));
        let o = lax.compute(&state(t2, vec![mbar(5, 9, 59, 102.5)], 101.0)).unwrap();
        assert!((o.metadata["avg_move_pct"] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn half_days_are_skipped_for_marks_they_lack() {
        // session 3 is a half day (09:00–12:00 buckets: marks up to 13:00), lookback 2, min 1:
        // at the 15:00 mark (minute 330) only session 4 counts
        let mut h: Vec<Candle> = (9..13).map(|hh| hbar(3, hh, 100.0, 101.0)).collect();
        h.extend(session(4, 0.03, 103.0));
        h.push(hbar(5, 9, 100.0, 100.0));
        h.push(hbar(5, 14, 100.0, 100.0));
        let mut i = ind(2);
        i.min_sessions = 1;
        let o = i.compute(&state(h, vec![mbar(5, 14, 59, 100.0)], 100.0)).unwrap();
        assert!((o.metadata["avg_move_pct"] - 3.0).abs() < 1e-9);
        assert_eq!(o.metadata["sessions"], 1.0);
        assert!((o.metadata["day_move_pct"] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn decision_fires_once_and_tolerates_a_missing_bar() {
        let mk = |bars: Vec<Candle>| {
            let mut h = hist();
            h.push(hbar(5, 9, 100.0, 100.0));
            h.push(hbar(5, 10, 100.0, 100.0));
            state(h, bars, 100.0)
        };
        let i = ind(2);
        // 10:29 present: decision on 10:29, not on 10:30
        assert_eq!(i.compute(&mk(vec![mbar(5, 10, 28, 100.0), mbar(5, 10, 29, 100.0)])).unwrap().metadata["decision"], 1.0);
        assert_eq!(i.compute(&mk(vec![mbar(5, 10, 29, 100.0), mbar(5, 10, 30, 100.0)])).unwrap().metadata["decision"], 0.0);
        // 10:29 missing: the 10:30 bar takes the decision; 10:33 (3 late) does not
        assert_eq!(i.compute(&mk(vec![mbar(5, 10, 28, 100.0), mbar(5, 10, 30, 100.0)])).unwrap().metadata["decision"], 1.0);
        assert_eq!(i.compute(&mk(vec![mbar(5, 10, 28, 100.0), mbar(5, 10, 33, 100.0)])).unwrap().metadata["decision"], 0.0);
    }

    #[test]
    fn none_before_the_open_or_without_today() {
        let h = hist();
        // last hourly group is yesterday's → None
        assert!(ind(2).compute(&state(h, vec![mbar(5, 9, 40, 100.0)], 100.0)).is_none());
    }

    #[test]
    fn factory_defaults() {
        let cfg = IndicatorConfig {
            indicator_type: "noise_area".into(), instance_id: "noise".into(), timescale: Timescale::OneMinute,
            enabled: true, weight: 0.0, params: HashMap::new(), last_modified_by: None, last_modified_at: None,
            modification_reason: None,
        };
        let i = noise_area_factory(&cfg);
        assert_eq!(i.name(), "noise_area");
    }
}
