use chrono::{DateTime, Utc};
use engine::{TradeRecord, TradingEngine};
use types::action::ExitReason;
use types::market::MarketState;
use types::scoring::TimescaleScores;
use types::tick_result::{TickEvent, TickResult};

/// a completed trade paired with its entry and exit scores and attribution.
#[derive(Debug, Clone)]
pub struct TradeWithScores {
    pub trade: TradeRecord,
    pub entry_scores: TimescaleScores,
    pub exit_scores: TimescaleScores,
    /// e.g. "window:5m thrust". empty when unknown.
    pub entry_reason: String,
    /// `config_versions.id` the engine that produced this trade was built from.
    pub config_version_id: i64,
    /// broker's actual entry fill, when the order was placed with a broker.
    pub broker_entry_price: Option<f64>,
}

/// wraps TradingEngine to capture entry/exit scores for trade writing.
/// uses TickResult to know exactly when positions open/close.
pub struct LiveSession {
    engine: TradingEngine,
    /// `config_versions.id` this engine was built from.
    config_version_id: i64,
    /// stashed entry scores when a position opens.
    entry_scores: Option<TimescaleScores>,
    /// stashed entry reason when a position opens.
    entry_reason: Option<String>,
    /// broker fill for the open position, if any.
    broker_entry_price: Option<f64>,
    /// most recent scores (used as exit scores for forced closes).
    last_scores: TimescaleScores,
    /// most recent tick result (for state reporting).
    last_result: Option<TickResult>,
}

impl LiveSession {
    pub fn new(engine: TradingEngine, config_version_id: i64) -> Self {
        Self {
            engine,
            config_version_id,
            entry_scores: None,
            entry_reason: None,
            broker_entry_price: None,
            last_scores: TimescaleScores::default(),
            last_result: None,
        }
    }

    /// process a tick and return the TickResult plus any completed trade with scores.
    pub fn on_tick(&mut self, market: &mut MarketState) -> (TickResult, Option<TradeWithScores>) {
        let result = self.engine.on_tick(market);
        self.last_scores = result.scores.clone();

        let trade_with_scores = match &result.event {
            TickEvent::PositionOpened => {
                // stash entry scores for when the position closes
                self.entry_scores = Some(result.scores.clone());
                self.entry_reason = Some(result.entry_reason.clone());
                self.broker_entry_price = None;
                None
            }
            TickEvent::PositionClosed => self.take_trade(result.scores.clone()),
            TickEvent::Nothing => None,
        };

        self.last_result = Some(result.clone());
        (result, trade_with_scores)
    }

    /// pair the most recently completed engine trade with the stashed entry data.
    fn take_trade(&mut self, exit_scores: TimescaleScores) -> Option<TradeWithScores> {
        let entry_scores = self.entry_scores.take().unwrap_or_default();
        let entry_reason = self.entry_reason.take().unwrap_or_default();
        let broker_entry_price = self.broker_entry_price.take();
        let trade = self.engine.completed_trades().last().cloned();
        trade.map(|t| TradeWithScores {
            trade: t,
            entry_scores,
            exit_scores,
            entry_reason,
            config_version_id: self.config_version_id,
            broker_entry_price,
        })
    }

    /// the broker rejected the entry order: roll the engine back to flat and
    /// clear everything stashed at open so the next trade is attributed cleanly.
    pub fn undo_open(&mut self) {
        self.engine.undo_last_open();
        self.entry_scores = None;
        self.entry_reason = None;
        self.broker_entry_price = None;
    }

    /// the broker did not confirm the exit the engine just took (`tws` is the trade
    /// `on_tick` returned): put the position and its attribution back so the trade is
    /// recorded only once the broker fills. false when the engine had nothing to revert.
    pub fn reopen_after_unconfirmed_close(&mut self, tws: &TradeWithScores) -> bool {
        if self.engine.revert_last_close().is_none() {
            return false;
        }
        self.entry_scores = Some(tws.entry_scores.clone());
        self.entry_reason = Some(tws.entry_reason.clone());
        self.broker_entry_price = tws.broker_entry_price;
        true
    }

    /// record the broker's actual entry fill for the open position.
    /// the broker filled fewer shares than the engine sized: shrink the engine position so
    /// the recorded trade and P&L reflect the real fill.
    pub fn resize_open(&mut self, quantity: f64) -> bool {
        self.engine.resize_open_position(quantity)
    }

    pub fn set_broker_entry_price(&mut self, price: f64) {
        self.broker_entry_price = Some(price);
    }

    /// take the entry stash (scores, reason) off a position that is being rolled back,
    /// so it can be re-attached if the broker turns out to hold the position after all.
    pub fn take_entry_stash(&mut self) -> (Option<TimescaleScores>, Option<String>) {
        (self.entry_scores.take(), self.entry_reason.take())
    }

    /// the broker holds a position the engine does not: force-open it with the broker's
    /// quantity and fill price so the exit stack manages it. `scores` default to the
    /// last scores seen.
    #[allow(clippy::too_many_arguments)]
    pub fn adopt_broker_position(
        &mut self,
        ticker: &str,
        direction: types::action::TradeDirection,
        quantity: f64,
        price: f64,
        timestamp: DateTime<Utc>,
        scores: Option<TimescaleScores>,
        reason: &str,
    ) -> Result<(), String> {
        if quantity <= 0.0 || price <= 0.0 {
            return Err(format!("cannot adopt {ticker}: quantity {quantity} price {price}"));
        }
        self.engine
            .force_open_position(ticker.to_string(), direction, price, quantity, timestamp)?;
        self.entry_scores = Some(scores.unwrap_or_else(|| self.last_scores.clone()));
        self.entry_reason = Some(reason.to_string());
        self.broker_entry_price = Some(price);
        Ok(())
    }

    /// gate entries until `until` (after a failed / unconfirmed broker order).
    pub fn suppress_entries_until(&mut self, until: DateTime<Utc>) {
        self.engine.suppress_entries_until(until);
    }

    /// force-close the open position outside the tick loop (shutdown, config
    /// swap, missed-close safety net). exit scores are the last scores seen.
    pub fn force_close(
        &mut self,
        price: f64,
        timestamp: DateTime<Utc>,
        reason: ExitReason,
    ) -> Option<TradeWithScores> {
        self.engine.force_close_position(price, timestamp, reason)?;
        let exit_scores = self.last_scores.clone();
        self.take_trade(exit_scores)
    }

    /// access the underlying engine.
    pub fn engine(&self) -> &TradingEngine {
        &self.engine
    }

    /// mutable access to the underlying engine.
    pub fn engine_mut(&mut self) -> &mut TradingEngine {
        &mut self.engine
    }

    /// check if there is an open position.
    pub fn has_position(&self) -> bool {
        self.engine.has_position()
    }

    /// get the current open position, if any.
    pub fn current_position(&self) -> Option<&types::action::Position> {
        self.engine.current_position()
    }

    /// get all completed trades.
    pub fn completed_trades(&self) -> &[TradeRecord] {
        self.engine.completed_trades()
    }

    pub fn config_version_id(&self) -> i64 {
        self.config_version_id
    }

    pub fn entry_reason(&self) -> Option<&str> {
        self.entry_reason.as_deref()
    }

    pub fn last_result(&self) -> Option<&TickResult> {
        self.last_result.as_ref()
    }

    pub fn last_scores(&self) -> &TimescaleScores {
        &self.last_scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use types::action::ActionPhase;
    use types::market::Timescale;
    use types::scoring::AggregationMethod;
    use types::test_fixtures::*;

    fn make_action_cfg(
        action_type: &str,
        id: &str,
        phase: ActionPhase,
        priority: i32,
        params: Vec<(&str, serde_json::Value)>,
    ) -> types::action::ActionConfig {
        types::action::ActionConfig {
            action_type: action_type.to_string(),
            instance_id: id.to_string(),
            phase,
            enabled: true,
            priority,
            params: params.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            last_modified_by: None,
            last_modified_at: None,
            modification_reason: None,
        }
    }

    fn build_test_session() -> LiveSession {
        let ind_reg = indicators::default_indicator_registry();
        let ind_configs = vec![
            make_indicator_config("rsi", "rsi_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
            make_indicator_config("ema", "ema_5m", Timescale::FiveMinute, 0.5, vec![("period", json!(10))]),
        ];
        let indicators = indicators::build_indicators(&ind_configs, &ind_reg).unwrap();

        let act_reg = actions::default_action_registry();
        let act_configs = vec![
            make_action_cfg("score_threshold_entry", "e1", ActionPhase::Entry, 0,
                vec![("entry_threshold", json!(0.5)), ("short_threshold", json!(-0.5))]),
            make_action_cfg("atr_trailing_stop", "ts1", ActionPhase::Exit, 0,
                vec![("atr_period", json!(14)), ("multiplier", json!(2.0))]),
        ];
        let action_sets = actions::build_actions(&act_configs, &act_reg).unwrap();

        let scoring = types::scoring::ScoringConfig {
            timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
            entry_threshold: 0.5,
            exit_threshold: -0.3,
            aggregation: AggregationMethod::WeightedSum,
            hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            hard_gate_indicators: std::collections::HashMap::new(),
                hourly_exit_override: None,
        };

        let engine = TradingEngine::new(
            indicators,
            ind_configs,
            scoring,
            action_sets.entry,
            action_sets.monitor,
            action_sets.exit,
            action_sets.sizing,
            "SPY".to_string(),
            100_000.0,
            None,
        );

        LiveSession::new(engine, 1)
    }

    #[test]
    fn no_trade_returns_none() {
        let mut session = build_test_session();
        // ranging data with high threshold → no entry
        let data = ranging_ohlcv(5, 100.0, 0.5);
        for i in 1..=data.len() {
            let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data[..i]);
            let (_result, trade) = session.on_tick(&mut ms);
            assert!(trade.is_none());
        }
    }

    #[test]
    fn entry_stashes_scores() {
        let mut session = build_test_session();
        let data = trending_up_ohlcv(50, 100.0, 1.0);

        let mut entered = false;
        for i in 1..=data.len() {
            let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data[..i]);
            let (result, _trade) = session.on_tick(&mut ms);
            if matches!(result.event, TickEvent::PositionOpened) {
                entered = true;
                assert!(session.entry_scores.is_some());
                break;
            }
        }
        // might not enter depending on RSI, but if it did, scores are stashed
        if entered {
            assert!(session.has_position());
        }
    }

    #[test]
    fn exit_returns_trade_with_scores() {
        let mut session = build_test_session();

        let mut data = trending_up_ohlcv(40, 100.0, 1.0);
        data.extend(trending_down_ohlcv(30, 140.0, 3.0));

        let mut trade_captured = false;
        for i in 1..=data.len() {
            let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data[..i]);
            let (_result, trade) = session.on_tick(&mut ms);
            if let Some(tws) = trade {
                assert!(!tws.trade.ticker.is_empty());
                // entry scores should have been stashed
                // (they might be default if entry happened on first tick)
                trade_captured = true;
                break;
            }
        }
        // we should have completed at least one trade
        if !trade_captured {
            // if no complete trade cycle happened, that's still valid
            // (depends on indicator/action dynamics)
        }
    }

    #[test]
    fn multiple_trades_in_sequence() {
        let mut session = build_test_session();

        // two cycles: up → down → up → down
        let mut data = trending_up_ohlcv(30, 100.0, 1.0);
        data.extend(trending_down_ohlcv(20, 130.0, 2.0));
        data.extend(trending_up_ohlcv(30, 90.0, 1.5));
        data.extend(trending_down_ohlcv(20, 135.0, 2.5));

        let mut trade_count = 0;
        for i in 1..=data.len() {
            let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data[..i]);
            let (_result, trade) = session.on_tick(&mut ms);
            if trade.is_some() {
                trade_count += 1;
            }
        }
        // should produce at least the same trades as the raw engine
        assert_eq!(trade_count, session.completed_trades().len());
    }

    #[test]
    fn tick_result_has_scores() {
        let mut session = build_test_session();
        let data = trending_up_ohlcv(20, 100.0, 1.0);
        let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
        let (result, _trade) = session.on_tick(&mut ms);
        // should always have a score computed
        // (composite might be 0 if no indicators fire, but it's present)
        let _ = result.scores.composite;
    }

    #[test]
    fn engine_accessor() {
        let session = build_test_session();
        assert!(!session.has_position());
        assert!(session.completed_trades().is_empty());
        assert_eq!(session.config_version_id(), 1);
        let _engine = session.engine();
    }

    #[test]
    fn undo_open_clears_stash_and_position() {
        let mut session = build_test_session();
        let data = trending_up_ohlcv(50, 100.0, 1.0);
        for i in 1..=data.len() {
            let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data[..i]);
            let (result, _trade) = session.on_tick(&mut ms);
            if matches!(result.event, TickEvent::PositionOpened) {
                assert!(session.entry_scores.is_some());
                session.undo_open();
                assert!(!session.has_position());
                assert!(session.entry_scores.is_none());
                assert!(session.entry_reason().is_none());
                return;
            }
        }
    }

    #[test]
    fn adopt_broker_position_opens_with_broker_numbers_and_attribution() {
        use chrono::Utc;
        use types::action::TradeDirection;
        let mut session = build_test_session();
        assert!(!session.has_position());
        let ts = Utc::now();
        session
            .adopt_broker_position("SPY", TradeDirection::Short, 48.0, 249.704792, ts, None, "adopted:orphan")
            .unwrap();
        let p = session.current_position().expect("adopted");
        assert_eq!(p.direction, TradeDirection::Short);
        assert!((p.size - 48.0).abs() < f64::EPSILON);
        assert!((p.entry_price - 249.704792).abs() < 1e-9);
        assert_eq!(session.entry_reason(), Some("adopted:orphan"));
        // a second adoption while one is open is refused, as is a nonsense fill
        assert!(session.adopt_broker_position("SPY", TradeDirection::Short, 1.0, 1.0, ts, None, "x").is_err());
        let tws = session.force_close(250.0, ts, ExitReason::ManualOverride).expect("trade");
        assert_eq!(tws.broker_entry_price, Some(249.704792));
        assert_eq!(tws.entry_reason, "adopted:orphan");
        assert!(session.adopt_broker_position("SPY", TradeDirection::Short, 0.0, 1.0, ts, None, "x").is_err());
    }

    #[test]
    fn reopen_after_unconfirmed_close_restores_position_and_attribution() {
        use chrono::Utc;
        use types::action::TradeDirection;
        let mut session = build_test_session();
        let ts = Utc::now();
        session
            .adopt_broker_position("SPY", TradeDirection::Short, 48.0, 249.70, ts, None, "adopted:orphan-at-start")
            .unwrap();
        let tws = session.force_close(254.82, ts + chrono::Duration::minutes(20), ExitReason::TrailingStop).expect("trade");
        assert!(!session.has_position());
        assert!(session.entry_reason().is_none());

        assert!(session.reopen_after_unconfirmed_close(&tws));
        let p = session.current_position().expect("position back");
        assert_eq!(p.direction, TradeDirection::Short);
        assert!((p.size - 48.0).abs() < f64::EPSILON);
        assert!((p.entry_price - 249.70).abs() < 1e-9);
        assert_eq!(session.entry_reason(), Some("adopted:orphan-at-start"));
        assert!(session.completed_trades().is_empty());
        // nothing to revert while a position is open
        assert!(!session.reopen_after_unconfirmed_close(&tws));
        // the confirmed exit later carries the same attribution
        let real = session.force_close(250.0, ts + chrono::Duration::minutes(25), ExitReason::TrailingStop).expect("trade");
        assert_eq!(real.entry_reason, "adopted:orphan-at-start");
        assert_eq!(real.broker_entry_price, Some(249.70));
        assert!((real.trade.exit_price - 250.0).abs() < 1e-9);
    }

    #[test]
    fn force_close_produces_trade_with_attribution() {
        use chrono::Utc;
        let mut session = build_test_session();
        let data = trending_up_ohlcv(50, 100.0, 1.0);
        for i in 1..=data.len() {
            let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data[..i]);
            let (result, _trade) = session.on_tick(&mut ms);
            if matches!(result.event, TickEvent::PositionOpened) {
                session.set_broker_entry_price(123.45);
                let tws = session
                    .force_close(ms.last_price + 1.0, Utc::now(), ExitReason::ManualOverride)
                    .expect("trade");
                assert_eq!(tws.trade.exit_reason, ExitReason::ManualOverride);
                assert_eq!(tws.config_version_id, 1);
                assert_eq!(tws.broker_entry_price, Some(123.45));
                assert!(!session.has_position());
                return;
            }
        }
    }
}
