use crate::scoring::TimescaleScores;

/// what happened during a tick.
#[derive(Debug, Clone)]
pub enum TickEvent {
    /// no position change.
    Nothing,
    /// a new position was opened.
    PositionOpened,
    /// a position was closed, producing a trade record.
    /// the trade record is stored in engine::position::TradeRecord (not re-exported
    /// here to avoid circular deps). the live session wrapper matches on this variant
    /// to capture scores.
    PositionClosed,
}

/// result of processing a single tick through the trading engine.
#[derive(Debug, Clone)]
pub struct TickResult {
    /// computed scores for all timescales + composite.
    pub scores: TimescaleScores,
    /// what event occurred (nothing, position opened, position closed).
    pub event: TickEvent,
    /// entry reason string (e.g. "window:candle_reversal"). only populated on PositionOpened.
    pub entry_reason: String,
    /// why no entry was evaluated this tick, when flat and blocked by a session
    /// gate or a reject gate (e.g. "avoid_first_minutes", "reject_gate:1m noise filter").
    /// `None` when in a position, when an entry fired, or when windows were
    /// evaluated and simply did not fire.
    pub entry_blocked_by: Option<String>,
    /// when flat, unblocked, and no window fired: a compact description of the
    /// closest windows and which conditions failed, e.g.
    /// "5m thrust: FiveMinute 0.31<0.50 | strong core: OneHour 0.12<0.30".
    /// only populated when at least one window's composite floor was met.
    pub near_miss: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_result_nothing_event() {
        let result = TickResult {
            scores: TimescaleScores::default(),
            event: TickEvent::Nothing,
            entry_reason: String::new(),
            entry_blocked_by: None,
            near_miss: None,
        };
        assert!(matches!(result.event, TickEvent::Nothing));
        assert!((result.scores.composite - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tick_result_position_opened() {
        let result = TickResult {
            scores: TimescaleScores {
                five_minute: Some(0.7),
                composite: 0.7,
                ..Default::default()
            },
            event: TickEvent::PositionOpened,
            entry_reason: "window:5m_thrust".to_string(),
            entry_blocked_by: None,
            near_miss: None,
        };
        assert!(matches!(result.event, TickEvent::PositionOpened));
        assert_eq!(result.scores.five_minute, Some(0.7));
    }

    #[test]
    fn tick_result_position_closed() {
        let result = TickResult {
            scores: TimescaleScores {
                five_minute: Some(-0.5),
                composite: -0.5,
                ..Default::default()
            },
            event: TickEvent::PositionClosed,
            entry_reason: String::new(),
            entry_blocked_by: None,
            near_miss: None,
        };
        assert!(matches!(result.event, TickEvent::PositionClosed));
    }

    #[test]
    fn tick_result_clone() {
        let result = TickResult {
            scores: TimescaleScores {
                one_minute: Some(0.3),
                five_minute: Some(0.5),
                composite: 0.4,
                ..Default::default()
            },
            event: TickEvent::PositionOpened,
            entry_reason: "score_threshold".to_string(),
            entry_blocked_by: None,
            near_miss: None,
        };
        let cloned = result.clone();
        assert_eq!(cloned.scores.one_minute, Some(0.3));
        assert!(matches!(cloned.event, TickEvent::PositionOpened));
    }

    #[test]
    fn default_timescale_scores_all_none() {
        let scores = TimescaleScores::default();
        assert!(scores.one_minute.is_none());
        assert!(scores.five_minute.is_none());
        assert!(scores.one_hour.is_none());
        assert!(scores.one_day.is_none());
        assert!(scores.one_month.is_none());
        assert!((scores.composite - 0.0).abs() < f64::EPSILON);
    }
}
