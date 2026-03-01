use chrono::{TimeZone, Utc};
use types::action::{Action, ActionPhase, ActionSignal, ExitReason, Position, TradeDirection};
use types::market::Timescale;
use types::scoring::TimescaleScores;
use types::test_fixtures::*;

use actions::entry::score_threshold::ScoreThresholdEntry;
use actions::exit::atr_trailing_stop::AtrTrailingStop;
use actions::exit::fixed_pct_stop::FixedPctStop;
use actions::exit::max_hold_timeout::MaxHoldTimeout;
use actions::exit::session_close::SessionCloseExit;
use actions::monitor::breakeven_stop::BreakevenStop;
use actions::sizing::fixed_fractional::FixedFractionalSizing;

fn default_scores(composite: f64) -> TimescaleScores {
    TimescaleScores {
        composite,
        ..Default::default()
    }
}

fn long_position(entry: f64, current: f64, hwm: f64) -> Position {
    Position {
        ticker: "SPY".to_string(),
        direction: TradeDirection::Long,
        entry_price: entry,
        current_price: current,
        size: 100.0,
        entry_time: Utc::now(),
        unrealized_pnl: (current - entry) * 100.0,
        unrealized_pnl_pct: (current - entry) / entry,
        high_water_mark: hwm,
        low_water_mark: entry,
        hold_duration_ms: 60_000,
    }
}

fn short_position(entry: f64, current: f64, lwm: f64) -> Position {
    Position {
        ticker: "SPY".to_string(),
        direction: TradeDirection::Short,
        entry_price: entry,
        current_price: current,
        size: 100.0,
        entry_time: Utc::now(),
        unrealized_pnl: (entry - current) * 100.0,
        unrealized_pnl_pct: (entry - current) / entry,
        high_water_mark: entry,
        low_water_mark: lwm,
        hold_duration_ms: 60_000,
    }
}

// ── ScoreThresholdEntry ──

#[test]
fn score_entry_fires_above_threshold() {
    let action = ScoreThresholdEntry::new(0.65, -0.65, "e1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let scores = default_scores(0.7);
    match action.evaluate(None, &ms, &scores) {
        ActionSignal::Enter { direction, .. } => {
            assert!(matches!(direction, TradeDirection::Long));
        }
        other => panic!("expected Enter, got {other:?}"),
    }
}

#[test]
fn score_entry_holds_below_threshold() {
    let action = ScoreThresholdEntry::new(0.65, -0.65, "e1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let scores = default_scores(0.3);
    assert!(matches!(action.evaluate(None, &ms, &scores), ActionSignal::Hold));
}

#[test]
fn score_entry_holds_when_position_exists() {
    let action = ScoreThresholdEntry::new(0.65, -0.65, "e1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let scores = default_scores(0.9);
    let pos = long_position(100.0, 101.0, 101.0);
    assert!(matches!(action.evaluate(Some(&pos), &ms, &scores), ActionSignal::Hold));
}

#[test]
fn score_entry_short_on_negative_score() {
    let action = ScoreThresholdEntry::new(0.65, -0.65, "e1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let scores = default_scores(-0.8);
    match action.evaluate(None, &ms, &scores) {
        ActionSignal::Enter { direction, .. } => {
            assert!(matches!(direction, TradeDirection::Short));
        }
        other => panic!("expected Enter Short, got {other:?}"),
    }
}

#[test]
fn score_entry_factory_works() {
    let cfg = types::action::ActionConfig {
        action_type: "score_threshold_entry".into(),
        instance_id: "e1".into(),
        phase: ActionPhase::Entry,
        enabled: true,
        priority: 0,
        params: Default::default(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    };
    let action = actions::entry::score_threshold::score_threshold_entry_factory(&cfg);
    assert_eq!(action.name(), "score_threshold_entry");
    assert_eq!(action.phase(), ActionPhase::Entry);
}

// ── ATRTrailingStop ──

#[test]
fn atr_trailing_holds_in_profit() {
    let action = AtrTrailingStop::new(14, 2.0, Timescale::FiveMinute, "ts1".into());
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    let pos = long_position(100.0, 114.0, 114.0);
    let scores = default_scores(0.5);
    assert!(matches!(action.evaluate(Some(&pos), &ms, &scores), ActionSignal::Hold));
}

#[test]
fn atr_trailing_exits_when_triggered() {
    let action = AtrTrailingStop::new(14, 2.0, Timescale::FiveMinute, "ts1".into());
    // ATR will be about 2.0 (high-low range is 2.0 per candle)
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    // HWM is 120, current price dropped to 110 (well below HWM - 2*ATR)
    let pos = long_position(100.0, 110.0, 120.0);
    let scores = default_scores(0.5);
    match action.evaluate(Some(&pos), &ms, &scores) {
        ActionSignal::Exit { reason } => assert_eq!(reason, ExitReason::TrailingStop),
        other => panic!("expected Exit TrailingStop, got {other:?}"),
    }
}

#[test]
fn atr_trailing_works_for_short() {
    let action = AtrTrailingStop::new(14, 2.0, Timescale::FiveMinute, "ts1".into());
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_down_ohlcv(30, 200.0, 0.5));
    // LWM is 180, current jumped to 195 (above LWM + 2*ATR)
    let pos = short_position(200.0, 195.0, 180.0);
    let scores = default_scores(-0.5);
    match action.evaluate(Some(&pos), &ms, &scores) {
        ActionSignal::Exit { reason } => assert_eq!(reason, ExitReason::TrailingStop),
        other => panic!("expected Exit TrailingStop for short, got {other:?}"),
    }
}

#[test]
fn atr_trailing_factory_works() {
    let cfg = types::action::ActionConfig {
        action_type: "atr_trailing_stop".into(),
        instance_id: "ts1".into(),
        phase: ActionPhase::Exit,
        enabled: true,
        priority: 0,
        params: Default::default(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    };
    let action = actions::exit::atr_trailing_stop::atr_trailing_stop_factory(&cfg);
    assert_eq!(action.name(), "atr_trailing_stop");
}

// ── FixedPctStop ──

#[test]
fn fixed_pct_exits_on_loss_exceeding_threshold() {
    let action = FixedPctStop::new(0.02, "fps1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[97.0]);
    let pos = long_position(100.0, 97.0, 100.0); // 3% loss > 2% threshold
    let scores = default_scores(0.0);
    match action.evaluate(Some(&pos), &ms, &scores) {
        ActionSignal::Exit { reason } => assert_eq!(reason, ExitReason::HardStop),
        other => panic!("expected Exit HardStop, got {other:?}"),
    }
}

#[test]
fn fixed_pct_holds_within_tolerance() {
    let action = FixedPctStop::new(0.02, "fps1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[99.0]);
    let pos = long_position(100.0, 99.0, 100.0); // 1% loss < 2% threshold
    let scores = default_scores(0.0);
    assert!(matches!(action.evaluate(Some(&pos), &ms, &scores), ActionSignal::Hold));
}

// ── SessionCloseExit ──

#[test]
fn session_close_exits_after_time() {
    let action = SessionCloseExit::new("15:55".to_string(), "sc1".into());
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    // set timestamp to 15:56 UTC
    ms.timestamp = Utc.with_ymd_and_hms(2024, 1, 15, 15, 56, 0).unwrap();
    let pos = long_position(100.0, 100.5, 100.5);
    let scores = default_scores(0.5);
    match action.evaluate(Some(&pos), &ms, &scores) {
        ActionSignal::Exit { reason } => assert_eq!(reason, ExitReason::SessionClose),
        other => panic!("expected Exit SessionClose, got {other:?}"),
    }
}

#[test]
fn session_close_holds_before_time() {
    let action = SessionCloseExit::new("15:55".to_string(), "sc1".into());
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.timestamp = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
    let pos = long_position(100.0, 100.5, 100.5);
    let scores = default_scores(0.5);
    assert!(matches!(action.evaluate(Some(&pos), &ms, &scores), ActionSignal::Hold));
}

// ── MaxHoldTimeout ──

#[test]
fn max_hold_exits_after_timeout() {
    let action = MaxHoldTimeout::new(3_600_000, "mh1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let mut pos = long_position(100.0, 100.5, 100.5);
    pos.hold_duration_ms = 3_700_000; // over 1 hour
    let scores = default_scores(0.5);
    match action.evaluate(Some(&pos), &ms, &scores) {
        ActionSignal::Exit { reason } => assert_eq!(reason, ExitReason::MaxHoldTimeout),
        other => panic!("expected Exit MaxHoldTimeout, got {other:?}"),
    }
}

#[test]
fn max_hold_holds_within_limit() {
    let action = MaxHoldTimeout::new(3_600_000, "mh1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let mut pos = long_position(100.0, 100.5, 100.5);
    pos.hold_duration_ms = 1_800_000; // 30 minutes
    let scores = default_scores(0.5);
    assert!(matches!(action.evaluate(Some(&pos), &ms, &scores), ActionSignal::Hold));
}

// ── BreakevenStop ──

#[test]
fn breakeven_activates_after_profit_trigger() {
    let action = BreakevenStop::new(0.01, "be1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[101.5]);
    // HWM reached 1.5% above entry → triggers 1% breakeven
    let pos = long_position(100.0, 101.2, 101.5);
    let scores = default_scores(0.5);
    match action.evaluate(Some(&pos), &ms, &scores) {
        ActionSignal::ModifyStop { new_stop_price } => {
            assert!((new_stop_price - 100.0).abs() < f64::EPSILON);
        }
        other => panic!("expected ModifyStop, got {other:?}"),
    }
}

#[test]
fn breakeven_does_not_activate_before_trigger() {
    let action = BreakevenStop::new(0.01, "be1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.3]);
    // HWM only 0.5% above entry
    let pos = long_position(100.0, 100.3, 100.5);
    let scores = default_scores(0.5);
    assert!(matches!(action.evaluate(Some(&pos), &ms, &scores), ActionSignal::Hold));
}

// ── FixedFractionalSizing ──

#[test]
fn fixed_fractional_calculates_correct_fraction() {
    let action = FixedFractionalSizing::new(0.05, "ff1".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    let scores = default_scores(0.7);
    match action.evaluate(None, &ms, &scores) {
        ActionSignal::Enter { size_fraction, direction, .. } => {
            assert!((size_fraction - 0.05).abs() < f64::EPSILON);
            assert!(matches!(direction, TradeDirection::Long));
        }
        other => panic!("expected Enter with fraction, got {other:?}"),
    }
}
