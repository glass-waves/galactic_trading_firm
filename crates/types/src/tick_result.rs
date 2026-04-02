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
