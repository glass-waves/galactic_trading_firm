use std::collections::{HashMap, HashSet};
use std::fs;
use std::process;

use tracing_subscriber::{prelude::*, EnvFilter};

use backtest::alpaca_loader::{build_backtest_data, fetch_bars_range, market_hours_utc};
use backtest::config_loader::{load_promoted_config_with_id, write_backtest_trades};
use backtest::replay::{BacktestConfig, BacktestCostConfig, BacktestData};
use backtest::{compute_metrics, load_candles_from_csv, run_backtest, to_json};
use types::scoring::TimescaleScores;
use chrono::{Duration, NaiveDate};
use engine::TradeRecord;
use types::action::{ActionConfig, ActionPhase};
use types::config::StrategyConfig;
use types::indicator::IndicatorConfig;
use types::market::Timescale;

/// CLI overrides for testing new features without modifying the DB config.
#[derive(Default)]
struct ConfigOverrides {
    entry_cooldown_ms: Option<i64>,
    max_daily_loss_pct: Option<f64>,
    sizing_fraction: Option<f64>,
    max_capital_deployed_pct: Option<f64>,
    no_vol_sizing: bool,
    profit_extension_ms: Option<i64>,
    loss_reduction_ms: Option<i64>,
    score_scaled_sizing: bool,
    score_scaled_min: Option<f64>,
    score_scaled_max: Option<f64>,
    add_rvol_weight: Option<f64>,
    // phase 4 optimization overrides
    avoid_first_minutes: Option<u32>,
    max_concurrent_positions: Option<u32>,
    entry_threshold: Option<f64>,
    exit_threshold: Option<f64>,
    /// override indicator weights by instance_id (e.g. "macd_5min=0.35")
    indicator_weights: Vec<(String, f64)>,
    /// override timescale weights (1min, 5min, 1hr)
    timescale_weight_1m: Option<f64>,
    timescale_weight_5m: Option<f64>,
    timescale_weight_1h: Option<f64>,
    /// enable cross-timescale agreement (ConfidenceMultiplier mode)
    enable_agreement: bool,
    agreement_exponent: Option<f64>,
    /// add OFI indicator to 5-min timescale
    add_ofi_weight: Option<f64>,
    /// enable dynamic fusion
    enable_dynamic_fusion: bool,
    fusion_vol_weight: Option<f64>,
    fusion_trend_weight: Option<f64>,
    fusion_adjustment: Option<f64>,
    fusion_bias: Option<f64>,
    // phase 5 — new alpha indicators
    add_vpin_weight: Option<f64>,
    /// VPIN hard gate: block entries when raw VPIN > threshold (e.g., 0.85).
    /// adds VPIN at weight 0.0 and uses hard_gate_indicators to block.
    add_vpin_gate: Option<f64>,
    add_position_direction: Option<f64>,
    add_unrealized_pnl: Option<f64>,
    add_hold_duration: Option<f64>,
    add_session_remaining: Option<f64>,
    add_momentum_persistence: Option<f64>,
    // candle pattern indicator
    add_candle_pattern: Option<f64>,
    candle_pattern_no_reversion: bool,
    candle_pattern_no_confluence: bool,
    candle_pattern_min_confluence: Option<f64>,
    candle_pattern_no_affirmative: bool,
    // generic indicator addition: --add-indicator TYPE:TIMESCALE:WEIGHT
    extra_indicators: Vec<(String, String, f64)>,
    // session overrides
    tickers_override: Option<String>,
    no_new_entries_after: Option<String>,
    /// override session.force_exit_by (HH:MM US/Eastern) and the session_close action param.
    force_exit_by: Option<String>,
    // per-ticker overrides: --ticker-override "NVDA:entry_threshold=0.35,stop_loss_pct=0.03"
    ticker_overrides: HashMap<String, types::config::TickerOverrides>,
    // entry windows: --use-entry-windows [--only-window NAME] [--disable-window NAME]
    use_entry_windows: bool,
    only_window: Option<String>,
    disable_window: Option<String>,
    // window parameter overrides (for sweep testing without recompiling)
    w1_composite_min: Option<f64>,
    w1_lead_by: Option<f64>,
    w1_5m_min: Option<f64>,
    w1_1h_min: Option<f64>,
    w4_composite_min: Option<f64>,
    w4_5m_min: Option<f64>,
    w4_1h_min: Option<f64>,
    reject_1m_lead: Option<f64>,
    reject_5m_max: Option<f64>,
    // W5 candle reversal window
    w5_indicator_min: Option<f64>,
    w5_1h_min: Option<f64>,
    w5_composite_min: Option<f64>,
    // exit action overrides
    max_hold_ms: Option<i64>,
    hold_score_gate: Option<f64>,
    hold_tiers: Vec<(f64, i64)>,       // (pnl_pct, extension_ms) pairs
    add_profit_trail: Option<f64>,     // giveback_fraction
    profit_trail_min: Option<f64>,     // min_profit_pct
    hourly_exit_override: Option<f64>, // suppress ScoreExit when 1h > threshold + profitable
    // generic blob overrides — work on whatever the promoted config already contains,
    // unlike --use-entry-windows which appends its own windows.
    //   --disable-action ID[,ID..]      set enabled=false on those instance_ids
    //   --enable-action ID[,ID..]       set enabled=true
    //   --set-action-param ID:PATH=JSON set a param; PATH may descend with '/', e.g.
    //       window_5m_thrust:conditions/1/min_score=0.4   or   hard_stop:stop_loss_pct=0.02
    //   (repeatable)
    disable_actions: Vec<String>,
    enable_actions: Vec<String>,
    set_action_params: Vec<(String, String, serde_json::Value)>,
    /// --mirror-short: for every enabled long entry_window / entry_reject_gate in the
    /// blob, add a short twin with every threshold sign-flipped (min↔max, lead↔lag).
    /// candle-pattern (indicator_min) windows are not mirrored.
    mirror_short: bool,
    /// --short-only: with --mirror-short, disable the original long windows.
    short_only: bool,
    /// --max-position-pct: override session.max_position_pct (the hard size clamp).
    max_position_pct: Option<f64>,
    /// --patch-json <file>: research patch. JSON object with optional keys
    ///   "disable": [instance_id, ...]        (actions to disable)
    ///   "indicators": [IndicatorConfig, ...] (appended)
    ///   "actions": [ActionConfig, ...]       (appended)
    /// applied after every other override. repeatable.
    patch_files: Vec<String>,
}

impl ConfigOverrides {
    /// build per-window exit overrides for entry windows mode.
    fn window_exit_overrides(&self) -> HashMap<String, engine::WindowExitOverrides> {
        if !self.use_entry_windows {
            return HashMap::new();
        }
        // v3 = best: no exit overrides. ScoreExit at -0.05 is load-bearing.
        // v4 (disabled ScoreExit): catastrophic churn (10x trades, all years negative)
        // v5 (-0.10/-0.15): mild churn, worse PF than v3 in every year
        // conclusion: exit tuning creates re-entry churn. keep default exits.
        HashMap::new()
    }
}

impl ConfigOverrides {
    fn any_active(&self) -> bool {
        self.entry_cooldown_ms.is_some()
            || self.max_daily_loss_pct.is_some()
            || self.sizing_fraction.is_some()
            || self.max_capital_deployed_pct.is_some()
            || self.no_vol_sizing
            || self.profit_extension_ms.is_some()
            || self.loss_reduction_ms.is_some()
            || self.score_scaled_sizing
            || self.score_scaled_min.is_some()
            || self.score_scaled_max.is_some()
            || self.add_rvol_weight.is_some()
            || self.avoid_first_minutes.is_some()
            || self.force_exit_by.is_some()
            || self.max_concurrent_positions.is_some()
            || self.entry_threshold.is_some()
            || self.exit_threshold.is_some()
            || !self.indicator_weights.is_empty()
            || self.timescale_weight_1m.is_some()
            || self.timescale_weight_5m.is_some()
            || self.timescale_weight_1h.is_some()
            || self.enable_agreement
            || self.agreement_exponent.is_some()
            || self.add_ofi_weight.is_some()
            || self.enable_dynamic_fusion
            || self.add_vpin_weight.is_some()
            || self.add_vpin_gate.is_some()
            || self.add_position_direction.is_some()
            || self.add_unrealized_pnl.is_some()
            || self.add_hold_duration.is_some()
            || self.add_session_remaining.is_some()
            || self.add_momentum_persistence.is_some()
            || self.add_candle_pattern.is_some()
            || self.candle_pattern_min_confluence.is_some()
            || self.candle_pattern_no_affirmative
            || !self.extra_indicators.is_empty()
            || !self.ticker_overrides.is_empty()
            || self.use_entry_windows
            || self.w5_indicator_min.is_some()
            || self.w5_1h_min.is_some()
            || self.w5_composite_min.is_some()
            || self.max_hold_ms.is_some()
            || self.hold_score_gate.is_some()
            || !self.hold_tiers.is_empty()
            || self.add_profit_trail.is_some()
            || self.hourly_exit_override.is_some()
            || !self.disable_actions.is_empty()
            || !self.enable_actions.is_empty()
            || !self.set_action_params.is_empty()
            || self.mirror_short
            || self.max_position_pct.is_some()
            || !self.patch_files.is_empty()
    }

    fn apply(&self, config: &mut StrategyConfig) {
        if let Some(cooldown) = self.entry_cooldown_ms {
            config.session.entry_cooldown_ms = cooldown;
        }
        if let Some(loss_pct) = self.max_daily_loss_pct {
            config.session.max_daily_loss_pct = Some(loss_pct);
        }
        if let Some(fraction) = self.sizing_fraction {
            for action in &mut config.actions {
                if action.phase == ActionPhase::Sizing {
                    if action.params.contains_key("fraction") {
                        action.params.insert("fraction".to_string(), serde_json::json!(fraction));
                        action.enabled = true;
                    }
                    if action.params.contains_key("base_fraction") {
                        action.params.insert("base_fraction".to_string(), serde_json::json!(fraction));
                    }
                }
            }
        }
        if self.no_vol_sizing {
            config.actions.retain(|a| a.action_type != "volatility_scaled");
            // ensure fixed_fractional is enabled as fallback
            for action in &mut config.actions {
                if action.action_type == "fixed_fractional" {
                    action.enabled = true;
                }
            }
        }
        if let Some(max_deployed) = self.max_capital_deployed_pct {
            config.session.max_capital_deployed_pct = max_deployed;
        }
        if let Some(cap) = self.max_position_pct {
            config.session.max_position_pct = Some(cap);
        }
        if let Some(ext) = self.profit_extension_ms {
            for action in &mut config.actions {
                if action.action_type == "max_hold_timeout" {
                    action.params.insert("profit_extension_ms".to_string(), serde_json::json!(ext));
                }
            }
        }
        if let Some(red) = self.loss_reduction_ms {
            for action in &mut config.actions {
                if action.action_type == "max_hold_timeout" {
                    action.params.insert("loss_reduction_ms".to_string(), serde_json::json!(red));
                }
            }
        }
        if self.score_scaled_sizing {
            let min = self.score_scaled_min.unwrap_or(0.03);
            let max = self.score_scaled_max.unwrap_or(0.06);
            let mut params = HashMap::new();
            params.insert("min_fraction".to_string(), serde_json::json!(min));
            params.insert("max_fraction".to_string(), serde_json::json!(max));
            params.insert("entry_threshold".to_string(), serde_json::json!(0.58));
            // disable existing sizing actions
            for action in &mut config.actions {
                if action.phase == ActionPhase::Sizing {
                    action.enabled = false;
                }
            }
            config.actions.push(ActionConfig {
                action_type: "score_scaled".to_string(),
                instance_id: "sizing_score".to_string(),
                phase: ActionPhase::Sizing,
                enabled: true,
                priority: 0,
                params,
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override for testing".to_string()),
            });
        }
        if let Some(weight) = self.add_rvol_weight {
            let mut params = HashMap::new();
            params.insert("lookback_period".to_string(), serde_json::json!(20));
            params.insert("high_threshold".to_string(), serde_json::json!(1.5));
            params.insert("low_threshold".to_string(), serde_json::json!(0.5));
            config.indicators.push(IndicatorConfig {
                indicator_type: "relative_volume".to_string(),
                instance_id: "rvol_20_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params,
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override for testing".to_string()),
            });
        }
        // phase 4 optimization overrides
        if let Some(mins) = self.avoid_first_minutes {
            config.session.avoid_first_minutes = mins;
        }
        if let Some(max_pos) = self.max_concurrent_positions {
            config.session.max_concurrent_positions = max_pos;
        }
        if let Some(thresh) = self.entry_threshold {
            config.scoring.entry_threshold = thresh;
            // also update the entry action's threshold param
            for action in &mut config.actions {
                if action.action_type == "score_threshold_entry" {
                    action.params.insert("entry_threshold".to_string(), serde_json::json!(thresh));
                }
            }
        }
        if let Some(thresh) = self.exit_threshold {
            config.scoring.exit_threshold = thresh;
        }
        // override specific indicator weights
        for (instance_id, weight) in &self.indicator_weights {
            for ind in &mut config.indicators {
                if ind.instance_id == *instance_id {
                    ind.weight = *weight;
                }
            }
        }
        // override timescale weights
        if let Some(w) = self.timescale_weight_1m {
            config.scoring.timescale_weights.insert(Timescale::OneMinute, w);
        }
        if let Some(w) = self.timescale_weight_5m {
            config.scoring.timescale_weights.insert(Timescale::FiveMinute, w);
        }
        if let Some(w) = self.timescale_weight_1h {
            config.scoring.timescale_weights.insert(Timescale::OneHour, w);
        }
        // agreement config
        if self.enable_agreement {
            let exponent = self.agreement_exponent.unwrap_or(0.5);
            config.scoring.agreement = Some(types::scoring::AgreementConfig {
                enabled: true,
                mode: types::scoring::AgreementMode::ConfidenceMultiplier,
                exponent,
                gate_threshold: 0.0,
            });
        }
        // add OFI indicator
        if let Some(weight) = self.add_ofi_weight {
            config.indicators.push(IndicatorConfig {
                indicator_type: "ofi".to_string(),
                instance_id: "ofi_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: OFI indicator".to_string()),
            });
        }
        // dynamic fusion
        if self.enable_dynamic_fusion {
            config.scoring.aggregation = types::scoring::AggregationMethod::DynamicFusion;
            config.scoring.dynamic_fusion = Some(types::scoring::DynamicFusionConfig {
                volatility_weight: self.fusion_vol_weight.unwrap_or(0.3),
                trend_weight: self.fusion_trend_weight.unwrap_or(0.2),
                bias: self.fusion_bias.unwrap_or(0.0),
                adjustment: self.fusion_adjustment.unwrap_or(0.15),
                volatility_indicator_id: "bb_bw_20_1hr".to_string(),
                trend_indicator_id: "adx_14_1hr".to_string(),
            });
        }
        // VPIN indicator (order flow toxicity — negative score when toxic)
        if let Some(weight) = self.add_vpin_weight {
            config.indicators.push(IndicatorConfig {
                indicator_type: "vpin".to_string(),
                instance_id: "vpin_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: VPIN indicator".to_string()),
            });
        }
        // VPIN hard gate: block entries when raw VPIN > threshold.
        // raw VPIN → score = 1 - 2*raw, so score_threshold = 1 - 2*raw_threshold.
        if let Some(raw_threshold) = self.add_vpin_gate {
            let score_threshold = 1.0 - 2.0 * raw_threshold;
            // add VPIN at weight 0.0 (no score contribution, purely a gate)
            config.indicators.push(IndicatorConfig {
                indicator_type: "vpin".to_string(),
                instance_id: "vpin_gate_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight: 0.0,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some(format!("CLI override: VPIN hard gate at {}", raw_threshold)),
            });
            config.scoring.hard_gate_indicators.insert(
                "vpin_gate_5min".to_string(),
                score_threshold,
            );
        }
        // position context meta-indicators
        if let Some(weight) = self.add_position_direction {
            config.indicators.push(IndicatorConfig {
                indicator_type: "position_direction".to_string(),
                instance_id: "pos_dir_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: position direction".to_string()),
            });
        }
        if let Some(weight) = self.add_unrealized_pnl {
            config.indicators.push(IndicatorConfig {
                indicator_type: "unrealized_pnl".to_string(),
                instance_id: "upnl_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: unrealized P&L".to_string()),
            });
        }
        if let Some(weight) = self.add_hold_duration {
            config.indicators.push(IndicatorConfig {
                indicator_type: "hold_duration".to_string(),
                instance_id: "hold_dur_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: hold duration".to_string()),
            });
        }
        if let Some(weight) = self.add_session_remaining {
            config.indicators.push(IndicatorConfig {
                indicator_type: "session_remaining".to_string(),
                instance_id: "sess_rem_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: session remaining".to_string()),
            });
        }
        // momentum persistence (ROC of ROC — second derivative)
        if let Some(weight) = self.add_momentum_persistence {
            config.indicators.push(IndicatorConfig {
                indicator_type: "momentum_persistence".to_string(),
                instance_id: "mom_persist_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: momentum persistence".to_string()),
            });
        }
        // candle pattern (engulfing with confluence scoring)
        if let Some(weight) = self.add_candle_pattern {
            let mut params = HashMap::new();
            if self.candle_pattern_no_reversion {
                params.insert("mean_reversion_mode".to_string(), serde_json::json!(false));
            }
            if self.candle_pattern_no_confluence {
                params.insert("use_confluence".to_string(), serde_json::json!(false));
            }
            config.indicators.push(IndicatorConfig {
                indicator_type: "candle_pattern".to_string(),
                instance_id: "candle_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params,
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: candle pattern".to_string()),
            });
        }
        // candle pattern overrides on existing config indicators
        if let Some(min_conf) = self.candle_pattern_min_confluence {
            for ind in &mut config.indicators {
                if ind.indicator_type == "candle_pattern" {
                    ind.params.insert("min_confluence_product".to_string(), serde_json::json!(min_conf));
                }
            }
        }
        if self.candle_pattern_no_affirmative {
            for ind in &mut config.indicators {
                if ind.indicator_type == "candle_pattern" {
                    ind.params.insert("affirmative_only".to_string(), serde_json::json!(false));
                }
            }
        }
        // generic indicator addition
        for (ind_type, ts_str, weight) in &self.extra_indicators {
            let timescale = match ts_str.as_str() {
                "1m" | "OneMinute" => Timescale::OneMinute,
                "5m" | "FiveMinute" => Timescale::FiveMinute,
                "1h" | "OneHour" => Timescale::OneHour,
                _ => Timescale::FiveMinute,
            };
            let instance_id = format!("{}_{}", ind_type, ts_str);
            config.indicators.push(IndicatorConfig {
                indicator_type: ind_type.clone(),
                instance_id,
                timescale,
                enabled: true,
                weight: *weight,
                params: HashMap::new(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some(format!("CLI override: {}", ind_type)),
            });
        }

        // hourly exit override: suppress ScoreExit when hourly trend is strong + profitable
        if let Some(thresh) = self.hourly_exit_override {
            config.scoring.hourly_exit_override = Some(thresh);
        }

        // generic blob overrides
        for action in &mut config.actions {
            if self.disable_actions.iter().any(|id| id == &action.instance_id) {
                action.enabled = false;
            }
            if self.enable_actions.iter().any(|id| id == &action.instance_id) {
                action.enabled = true;
            }
        }
        if self.mirror_short {
            // the hourly hard gate floors the composite to 0 whenever the 1h score is
            // negative — a long-only assumption that makes short windows unreachable.
            // the windows carry their own 1h conditions, so drop the gate for symmetry.
            config.scoring.hard_gate_timescales.clear();
            let mut twins = Vec::new();
            for action in &config.actions {
                if !action.enabled
                    || (action.action_type != "entry_window" && action.action_type != "entry_reject_gate")
                {
                    continue;
                }
                let Some(conds) = action.params.get("conditions").and_then(|v| v.as_array()) else { continue };
                if conds.iter().any(|c| c.get("type").and_then(|t| t.as_str()).map(|t| t.starts_with("indicator_")).unwrap_or(false)) {
                    continue; // candle / indicator windows: no symmetric meaning
                }
                let mirrored: Vec<serde_json::Value> = conds.iter().map(mirror_condition).collect();
                let mut twin = action.clone();
                twin.instance_id = format!("{}_short", action.instance_id);
                twin.params.insert("conditions".to_string(), serde_json::Value::Array(mirrored));
                if action.action_type == "entry_window" {
                    twin.params.insert("direction".to_string(), serde_json::json!("short"));
                    let name = action.params.get("name").and_then(|v| v.as_str()).unwrap_or(&action.instance_id);
                    twin.params.insert("name".to_string(), serde_json::json!(format!("{name} short")));
                }
                twins.push(twin);
            }
            if self.short_only {
                for action in &mut config.actions {
                    if action.action_type == "entry_window" {
                        action.enabled = false;
                    }
                }
            }
            config.actions.extend(twins);
        }
        for file in &self.patch_files {
            let text = match std::fs::read_to_string(file) {
                Ok(t) => t,
                Err(e) => { eprintln!("error: --patch-json {file}: {e}"); std::process::exit(2); }
            };
            let patch: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => { eprintln!("error: --patch-json {file}: invalid json: {e}"); std::process::exit(2); }
            };
            if let Some(ids) = patch.get("disable").and_then(|v| v.as_array()) {
                for action in &mut config.actions {
                    if ids.iter().any(|x| x.as_str() == Some(action.instance_id.as_str())) {
                        action.enabled = false;
                    }
                }
            }
            if let Some(inds) = patch.get("indicators") {
                match serde_json::from_value::<Vec<IndicatorConfig>>(inds.clone()) {
                    Ok(v) => config.indicators.extend(v),
                    Err(e) => { eprintln!("error: --patch-json {file}: indicators: {e}"); std::process::exit(2); }
                }
            }
            if let Some(acts) = patch.get("actions") {
                match serde_json::from_value::<Vec<ActionConfig>>(acts.clone()) {
                    Ok(v) => config.actions.extend(v),
                    Err(e) => { eprintln!("error: --patch-json {file}: actions: {e}"); std::process::exit(2); }
                }
            }
        }
        for (id, path, value) in &self.set_action_params {
            let Some(action) = config.actions.iter_mut().find(|a| &a.instance_id == id) else {
                eprintln!("warning: --set-action-param: no action with instance_id '{id}'");
                continue;
            };
            let (key, rest) = match path.split_once('/') {
                Some((k, r)) => (k.to_string(), Some(format!("/{r}"))),
                None => (path.clone(), None),
            };
            match rest {
                None => {
                    action.params.insert(key, value.clone());
                }
                Some(ptr) => {
                    let Some(root) = action.params.get_mut(&key) else {
                        eprintln!("warning: --set-action-param: '{id}' has no param '{key}'");
                        continue;
                    };
                    match root.pointer_mut(&ptr) {
                        Some(slot) => *slot = value.clone(),
                        None => eprintln!("warning: --set-action-param: path '{path}' not found in '{id}'"),
                    }
                }
            }
        }

        // max hold timeout overrides (base ms, score gate, profit tiers)
        if self.max_hold_ms.is_some() || self.hold_score_gate.is_some() || !self.hold_tiers.is_empty() {
            for action in &mut config.actions {
                if action.action_type == "max_hold_timeout" {
                    if let Some(ms) = self.max_hold_ms {
                        action.params.insert("max_hold_ms".to_string(), serde_json::json!(ms));
                    }
                    if let Some(gate) = self.hold_score_gate {
                        action.params.insert("score_gate".to_string(), serde_json::json!(gate));
                    }
                    for (i, (pnl_pct, ext_ms)) in self.hold_tiers.iter().enumerate() {
                        action.params.insert(format!("tier{}_pnl_pct", i + 1), serde_json::json!(pnl_pct));
                        action.params.insert(format!("tier{}_extension_ms", i + 1), serde_json::json!(ext_ms));
                    }
                }
            }
        }
        // add profit trailing stop action
        if let Some(giveback) = self.add_profit_trail {
            let min_profit = self.profit_trail_min.unwrap_or(0.0005);
            let mut params = HashMap::new();
            params.insert("giveback_fraction".to_string(), serde_json::json!(giveback));
            params.insert("min_profit_pct".to_string(), serde_json::json!(min_profit));
            config.actions.push(ActionConfig {
                action_type: "profit_trailing_stop".to_string(),
                instance_id: "profit_trail".to_string(),
                phase: ActionPhase::Exit,
                enabled: true,
                priority: 2, // between trailing_stop_atr (0) and max_hold (5)
                params,
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override: profit trailing stop".to_string()),
            });
        }

        // entry windows: replace score_threshold_entry with window actions
        if self.use_entry_windows {
            // remove existing score_threshold_entry
            config.actions.retain(|a| a.action_type != "score_threshold_entry");

            // reject gate params (overridable)
            let reject_1m_lead = self.reject_1m_lead.unwrap_or(0.15);
            let reject_5m_max = self.reject_5m_max.unwrap_or(0.35);

            // add reject gate (highest priority = 0)
            config.actions.push(ActionConfig {
                action_type: "entry_reject_gate".to_string(),
                instance_id: "reject_1m_noise".to_string(),
                phase: ActionPhase::Entry,
                enabled: true,
                priority: 0,
                params: serde_json::from_value(serde_json::json!({
                    "name": "1m noise filter",
                    "conditions": [
                        {"type": "timescale_lead", "timescale": "OneMinute", "lead_by": reject_1m_lead},
                        {"type": "timescale_max", "timescale": "FiveMinute", "max_score": reject_5m_max}
                    ]
                })).unwrap(),
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("entry windows CLI".to_string()),
            });

            // window params (overridable via CLI for sweep testing)
            let w1_composite = self.w1_composite_min.unwrap_or(0.35);
            let w1_lead = self.w1_lead_by.unwrap_or(0.15);
            let w1_5m = self.w1_5m_min.unwrap_or(0.50);
            let w1_1h = self.w1_1h_min.unwrap_or(0.0);
            let w4_composite = self.w4_composite_min.unwrap_or(0.35);
            let w4_5m = self.w4_5m_min.unwrap_or(0.40);
            let w4_1h = self.w4_1h_min.unwrap_or(0.30);
            let w5_indicator = self.w5_indicator_min.unwrap_or(0.40);
            let w5_1h = self.w5_1h_min.unwrap_or(0.20);
            let w5_composite = self.w5_composite_min.unwrap_or(0.20);

            // window definitions (priority: lower = evaluated first after reject gates)
            let windows = vec![
                // W1: momentum breakout — 5m decisively leads
                ("window_5m_thrust", "5m thrust", 10, serde_json::json!([
                    {"type": "composite_min", "min_score": w1_composite},
                    {"type": "timescale_lead", "timescale": "FiveMinute", "lead_by": w1_lead},
                    {"type": "timescale_min", "timescale": "FiveMinute", "min_score": w1_5m},
                    {"type": "timescale_min", "timescale": "OneHour", "min_score": w1_1h}
                ])),
                // W5: candle reversal — pattern fires with hourly support
                ("window_candle_reversal", "candle reversal", 15, serde_json::json!([
                    {"type": "indicator_min", "instance_id": "candle_5min", "min_score": w5_indicator},
                    {"type": "timescale_min", "timescale": "OneHour", "min_score": w5_1h},
                    {"type": "composite_min", "min_score": w5_composite}
                ])),
                // W4: high conviction — both core timescales strong
                ("window_strong_core", "strong core", 20, serde_json::json!([
                    {"type": "composite_min", "min_score": w4_composite},
                    {"type": "timescale_min", "timescale": "FiveMinute", "min_score": w4_5m},
                    {"type": "timescale_min", "timescale": "OneHour", "min_score": w4_1h}
                ])),
            ];

            for (id, name, priority, conditions) in windows {
                let enabled = match (&self.only_window, &self.disable_window) {
                    (Some(only), _) => only == name || only == id,
                    (_, Some(disabled)) => disabled != name && disabled != id,
                    _ => true,
                };
                config.actions.push(ActionConfig {
                    action_type: "entry_window".to_string(),
                    instance_id: id.to_string(),
                    phase: ActionPhase::Entry,
                    enabled,
                    priority,
                    params: serde_json::from_value(serde_json::json!({
                        "name": name,
                        "direction": "long",
                        "conditions": conditions,
                    })).unwrap(),
                    last_modified_by: Some("backtest_cli".to_string()),
                    last_modified_at: None,
                    modification_reason: Some("entry windows CLI".to_string()),
                });
            }
        }
    }
}

fn parse_overrides(args: &[String]) -> ConfigOverrides {
    // parse indicator weight overrides: --indicator-weight instance_id=weight
    let mut indicator_weights = Vec::new();
    // parse generic indicator additions: --add-indicator TYPE:TIMESCALE:WEIGHT
    let mut extra_indicators = Vec::new();
    // parse hold tiers: --hold-tier PNL_PCT:EXTENSION_MS (repeatable)
    let mut hold_tiers = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--indicator-weight" {
            if let Some(val) = args.get(i + 1) {
                if let Some((id, w)) = val.split_once('=') {
                    if let Ok(weight) = w.parse::<f64>() {
                        indicator_weights.push((id.to_string(), weight));
                    }
                }
            }
        }
        if args[i] == "--add-indicator" {
            if let Some(val) = args.get(i + 1) {
                let parts: Vec<&str> = val.split(':').collect();
                if parts.len() == 3 {
                    if let Ok(weight) = parts[2].parse::<f64>() {
                        extra_indicators.push((
                            parts[0].to_string(),
                            parts[1].to_string(),
                            weight,
                        ));
                    }
                }
            }
        }
        if args[i] == "--hold-tier" {
            if let Some(val) = args.get(i + 1) {
                if let Some((pnl_str, ext_str)) = val.split_once(':') {
                    if let (Ok(pnl), Ok(ext)) = (pnl_str.parse::<f64>(), ext_str.parse::<i64>()) {
                        hold_tiers.push((pnl, ext));
                    }
                }
            }
        }
        i += 1;
    }

    ConfigOverrides {
        entry_cooldown_ms: get_arg(args, "--entry-cooldown-ms").and_then(|s| s.parse().ok()),
        max_daily_loss_pct: get_arg(args, "--max-daily-loss-pct").and_then(|s| s.parse().ok()),
        sizing_fraction: get_arg(args, "--sizing-fraction").and_then(|s| s.parse().ok()),
        max_capital_deployed_pct: get_arg(args, "--max-capital-deployed-pct").and_then(|s| s.parse().ok()),
        no_vol_sizing: args.iter().any(|a| a == "--no-vol-sizing"),
        profit_extension_ms: get_arg(args, "--profit-extension-ms").and_then(|s| s.parse().ok()),
        loss_reduction_ms: get_arg(args, "--loss-reduction-ms").and_then(|s| s.parse().ok()),
        score_scaled_sizing: args.iter().any(|a| a == "--score-scaled-sizing"),
        score_scaled_min: get_arg(args, "--score-scaled-min").and_then(|s| s.parse().ok()),
        score_scaled_max: get_arg(args, "--score-scaled-max").and_then(|s| s.parse().ok()),
        add_rvol_weight: get_arg(args, "--add-rvol").and_then(|s| s.parse().ok()),
        avoid_first_minutes: get_arg(args, "--avoid-first-minutes").and_then(|s| s.parse().ok()),
        max_concurrent_positions: get_arg(args, "--max-concurrent-positions").and_then(|s| s.parse().ok()),
        entry_threshold: get_arg(args, "--entry-threshold").and_then(|s| s.parse().ok()),
        exit_threshold: get_arg(args, "--exit-threshold").and_then(|s| s.parse().ok()),
        indicator_weights,
        timescale_weight_1m: get_arg(args, "--ts-weight-1m").and_then(|s| s.parse().ok()),
        timescale_weight_5m: get_arg(args, "--ts-weight-5m").and_then(|s| s.parse().ok()),
        timescale_weight_1h: get_arg(args, "--ts-weight-1h").and_then(|s| s.parse().ok()),
        enable_agreement: args.iter().any(|a| a == "--enable-agreement"),
        agreement_exponent: get_arg(args, "--agreement-exponent").and_then(|s| s.parse().ok()),
        add_ofi_weight: get_arg(args, "--add-ofi").and_then(|s| s.parse().ok()),
        enable_dynamic_fusion: args.iter().any(|a| a == "--enable-dynamic-fusion"),
        fusion_vol_weight: get_arg(args, "--fusion-vol-weight").and_then(|s| s.parse().ok()),
        fusion_trend_weight: get_arg(args, "--fusion-trend-weight").and_then(|s| s.parse().ok()),
        fusion_adjustment: get_arg(args, "--fusion-adjustment").and_then(|s| s.parse().ok()),
        fusion_bias: get_arg(args, "--fusion-bias").and_then(|s| s.parse().ok()),
        add_vpin_weight: get_arg(args, "--add-vpin").and_then(|s| s.parse().ok()),
        add_vpin_gate: get_arg(args, "--add-vpin-gate").and_then(|s| s.parse().ok()),
        add_position_direction: get_arg(args, "--add-position-direction").and_then(|s| s.parse().ok()),
        add_unrealized_pnl: get_arg(args, "--add-unrealized-pnl").and_then(|s| s.parse().ok()),
        add_hold_duration: get_arg(args, "--add-hold-duration").and_then(|s| s.parse().ok()),
        add_session_remaining: get_arg(args, "--add-session-remaining").and_then(|s| s.parse().ok()),
        add_momentum_persistence: get_arg(args, "--add-momentum-persistence").and_then(|s| s.parse().ok()),
        add_candle_pattern: get_arg(args, "--add-candle-pattern").and_then(|s| s.parse().ok()),
        candle_pattern_no_reversion: args.iter().any(|a| a == "--candle-pattern-no-reversion"),
        candle_pattern_no_confluence: args.iter().any(|a| a == "--candle-pattern-no-confluence"),
        candle_pattern_min_confluence: get_arg(args, "--candle-min-confluence").and_then(|s| s.parse().ok()),
        candle_pattern_no_affirmative: args.iter().any(|a| a == "--candle-pattern-no-affirmative"),
        extra_indicators,
        tickers_override: get_arg(args, "--tickers"),
        no_new_entries_after: get_arg(args, "--no-new-entries-after"),
        force_exit_by: get_arg(args, "--force-exit-by"),
        ticker_overrides: parse_ticker_overrides(args),
        use_entry_windows: args.iter().any(|a| a == "--use-entry-windows"),
        only_window: get_arg(args, "--only-window"),
        disable_window: get_arg(args, "--disable-window"),
        // window param overrides for sweep testing
        w1_composite_min: get_arg(args, "--w1-composite-min").and_then(|s| s.parse().ok()),
        w1_lead_by: get_arg(args, "--w1-lead-by").and_then(|s| s.parse().ok()),
        w1_5m_min: get_arg(args, "--w1-5m-min").and_then(|s| s.parse().ok()),
        w1_1h_min: get_arg(args, "--w1-1h-min").and_then(|s| s.parse().ok()),
        w4_composite_min: get_arg(args, "--w4-composite-min").and_then(|s| s.parse().ok()),
        w4_5m_min: get_arg(args, "--w4-5m-min").and_then(|s| s.parse().ok()),
        w4_1h_min: get_arg(args, "--w4-1h-min").and_then(|s| s.parse().ok()),
        reject_1m_lead: get_arg(args, "--reject-1m-lead").and_then(|s| s.parse().ok()),
        reject_5m_max: get_arg(args, "--reject-5m-max").and_then(|s| s.parse().ok()),
        // W5 candle reversal window
        w5_indicator_min: get_arg(args, "--w5-indicator-min").and_then(|s| s.parse().ok()),
        w5_1h_min: get_arg(args, "--w5-1h-min").and_then(|s| s.parse().ok()),
        w5_composite_min: get_arg(args, "--w5-composite-min").and_then(|s| s.parse().ok()),
        // exit action overrides
        max_hold_ms: get_arg(args, "--max-hold-ms").and_then(|s| s.parse().ok()),
        hold_score_gate: get_arg(args, "--hold-score-gate").and_then(|s| s.parse().ok()),
        hold_tiers,
        add_profit_trail: get_arg(args, "--add-profit-trail").and_then(|s| s.parse().ok()),
        profit_trail_min: get_arg(args, "--profit-trail-min").and_then(|s| s.parse().ok()),
        hourly_exit_override: get_arg(args, "--hourly-exit-override").and_then(|s| s.parse().ok()),
        disable_actions: get_all_args(args, "--disable-action")
            .iter()
            .flat_map(|v| v.split(',').map(|x| x.trim().to_string()))
            .filter(|x| !x.is_empty())
            .collect(),
        enable_actions: get_all_args(args, "--enable-action")
            .iter()
            .flat_map(|v| v.split(',').map(|x| x.trim().to_string()))
            .filter(|x| !x.is_empty())
            .collect(),
        mirror_short: args.iter().any(|a| a == "--mirror-short"),
        max_position_pct: get_arg(args, "--max-position-pct").and_then(|s| s.parse().ok()),
        patch_files: get_all_args(args, "--patch-json"),
        short_only: args.iter().any(|a| a == "--short-only"),
        set_action_params: get_all_args(args, "--set-action-param")
            .iter()
            .filter_map(|spec| {
                let (id, kv) = spec.split_once(':')?;
                let (path, raw) = kv.split_once('=')?;
                let value = serde_json::from_str::<serde_json::Value>(raw)
                    .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()));
                Some((id.to_string(), path.to_string(), value))
            })
            .collect(),
    }
}

/// sign-flip a window condition for a short twin: min→max (negated), lead→lag.
fn mirror_condition(c: &serde_json::Value) -> serde_json::Value {
    let mut m = c.clone();
    let t = c.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let neg = |m: &mut serde_json::Value, from: &str, to: &str| {
        if let Some(v) = m.get(from).and_then(|v| v.as_f64()) {
            m.as_object_mut().unwrap().remove(from);
            m[to] = serde_json::json!(-v);
        }
    };
    match t.as_str() {
        "timescale_min" => { m["type"] = "timescale_max".into(); neg(&mut m, "min_score", "max_score"); }
        "timescale_max" => { m["type"] = "timescale_min".into(); neg(&mut m, "max_score", "min_score"); }
        "composite_min" => { m["type"] = "composite_max".into(); neg(&mut m, "min_score", "max_score"); }
        "composite_max" => { m["type"] = "composite_min".into(); neg(&mut m, "max_score", "min_score"); }
        "timescale_lead" => {
            m["type"] = "timescale_lag".into();
            if let Some(v) = m.get("lead_by").and_then(|v| v.as_f64()) {
                m.as_object_mut().unwrap().remove("lead_by");
                m["lag_by"] = serde_json::json!(v);
            }
        }
        "timescale_lag" => {
            m["type"] = "timescale_lead".into();
            if let Some(v) = m.get("lag_by").and_then(|v| v.as_f64()) {
                m.as_object_mut().unwrap().remove("lag_by");
                m["lead_by"] = serde_json::json!(v);
            }
        }
        "timescale_all_min" => { m["type"] = "timescale_all_max".into(); neg(&mut m, "min_score", "max_score"); }
        "timescale_range" => {
            let lo = m.get("min_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let hi = m.get("max_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            m["min_score"] = serde_json::json!(-hi);
            m["max_score"] = serde_json::json!(-lo);
        }
        _ => {}
    }
    m
}

/// every value following any occurrence of `flag` (repeatable flags).
fn get_all_args(args: &[String], flag: &str) -> Vec<String> {
    args.windows(2)
        .filter(|w| w[0] == flag)
        .map(|w| w[1].clone())
        .collect()
}

/// parse `--ticker-override "NVDA:entry_threshold=0.35,stop_loss_pct=0.03"` flags.
/// multiple flags allowed (one per ticker).
fn parse_ticker_overrides(args: &[String]) -> HashMap<String, types::config::TickerOverrides> {
    let mut result = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--ticker-override" && i + 1 < args.len() {
            let val = &args[i + 1];
            if let Some(colon) = val.find(':') {
                let ticker = val[..colon].to_string();
                let pairs = &val[colon + 1..];
                let mut ovr = types::config::TickerOverrides::default();
                for pair in pairs.split(',') {
                    let parts: Vec<&str> = pair.splitn(2, '=').collect();
                    if parts.len() != 2 {
                        continue;
                    }
                    let key = parts[0].trim();
                    let val_str = parts[1].trim();
                    match key {
                        "entry_threshold" => ovr.entry_threshold = val_str.parse().ok(),
                        "exit_threshold" => ovr.exit_threshold = val_str.parse().ok(),
                        "max_hold_ms" => ovr.max_hold_ms = val_str.parse().ok(),
                        "stop_loss_pct" => ovr.stop_loss_pct = val_str.parse().ok(),
                        "atr_multiplier" => ovr.atr_multiplier = val_str.parse().ok(),
                        "sizing_fraction" => ovr.sizing_fraction = val_str.parse().ok(),
                        k if k.starts_with("weight:") => {
                            let instance_id = k.trim_start_matches("weight:");
                            if let Ok(w) = val_str.parse::<f64>() {
                                ovr.indicator_weights.insert(instance_id.to_string(), w);
                            }
                        }
                        _ => eprintln!("warning: unknown ticker override key '{key}'"),
                    }
                }
                result.insert(ticker, ovr);
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    result
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let capital: f64 = get_arg(&args, "--capital")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);

    let cost_config = parse_cost_config(&args);
    let overrides = parse_overrides(&args);

    let verbose = args.iter().any(|a| a == "--verbose");
    let output_equity = args.iter().any(|a| a == "--output-equity");
    let output_trades_csv = args.iter().any(|a| a == "--output-trades-csv");

    if let Some(date_str) = get_arg(&args, "--date") {
        let lookback_days: i64 = get_arg(&args, "--lookback-days")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let write_db = args.iter().any(|a| a == "--write-db");
        let bars_dir = get_arg(&args, "--bars-dir");
        let dump_ticks = get_arg(&args, "--dump-ticks");
        let cross_index = get_arg(&args, "--cross-index");
        if let Some(lag) = get_arg(&args, "--cross-lag") {
            std::env::set_var("BACKTEST_CROSS_LAG", lag);
        }
        if args.iter().any(|a| a == "--dump-window-only") {
            std::env::set_var("BACKTEST_DUMP_WINDOW_ONLY", "1");
        }
        run_date_mode(&date_str, lookback_days, write_db, capital, &cost_config, verbose, output_equity, output_trades_csv, &overrides, bars_dir.as_deref(), dump_ticks.as_deref(), cross_index.as_deref()).await;
    } else if let Some(dir) = get_arg(&args, "--fetch-bars") {
        // build / extend the local 1-minute bar cache used by `--bars-dir`
        let start = get_arg(&args, "--start").expect("--fetch-bars needs --start YYYY-MM-DD");
        let end = get_arg(&args, "--end").expect("--fetch-bars needs --end YYYY-MM-DD");
        let tickers = get_arg(&args, "--tickers").expect("--fetch-bars needs --tickers A,B,C");
        run_fetch_bars(&dir, &start, &end, &tickers).await;
    } else {
        run_legacy_mode(&args, capital, &cost_config);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_date_mode(date_str: &str, lookback_days: i64, write_db: bool, capital: f64, cost_config: &Option<BacktestCostConfig>, verbose: bool, output_equity: bool, output_trades_csv: bool, overrides: &ConfigOverrides, bars_dir: Option<&str>, dump_ticks: Option<&str>, cross_index: Option<&str>) {
    dotenvy::dotenv().ok();

    // file + console layered logging
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/backtest_{}.log", log_dir, date_str))
        .expect("failed to open log file");

    // guard must live until end of function to flush buffered log writes
    let (non_blocking, _log_flush_guard) = tracing_appender::non_blocking(log_file);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(EnvFilter::new("info"));

    let console_layer = tracing_subscriber::fmt::layer()
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("backtest=info".parse().unwrap()),
        );

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: invalid date '{}': {}", date_str, e);
            process::exit(1);
        }
    };

    // connect to postgres
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("error: DATABASE_URL not set");
        process::exit(1);
    });

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: failed to connect to database: {e}");
            process::exit(1);
        });

    // load promoted config
    let (mut config, config_version_id) = match load_promoted_config_with_id(&pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    // apply CLI overrides for testing
    if overrides.any_active() {
        overrides.apply(&mut config);
    }

    // ticker override
    if let Some(ticker_str) = overrides.tickers_override.as_ref() {
        config.tickers = ticker_str.split(',').map(|s| s.trim().to_string()).collect();
    }

    // session time override
    if let Some(ref time) = overrides.force_exit_by {
        config.session.force_exit_by = time.clone();
        for action in &mut config.actions {
            if action.action_type == "session_close" {
                action
                    .params
                    .insert("force_exit_by".to_string(), serde_json::json!(time));
            }
        }
    }
    if let Some(ref time) = overrides.no_new_entries_after {
        config.session.no_new_entries_after = time.clone();
    }

    // alpaca credentials (not needed when replaying from the local bar cache)
    let (api_key, api_secret) = if bars_dir.is_some() {
        (String::new(), String::new())
    } else {
        (
            std::env::var("APCA_API_KEY_ID").unwrap_or_else(|_| {
                eprintln!("error: APCA_API_KEY_ID not set");
                process::exit(1);
            }),
            std::env::var("APCA_API_SECRET_KEY").unwrap_or_else(|_| {
                eprintln!("error: APCA_API_SECRET_KEY not set");
                process::exit(1);
            }),
        )
    };

    // collect all unique timescales from indicators + scoring weights
    let required_timescales = collect_timescales(&config);

    // compute lookback range for indicator warmup
    let lookback_start = date - Duration::days(lookback_days);

    // target date market hours for filtering results
    let (target_open_utc, target_close_utc) = match market_hours_utc(date) {
        Ok(bounds) => bounds,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if !output_trades_csv {
        if lookback_days > 0 {
            println!("backtest: {} (lookback from {})", date, lookback_start);
        } else {
            println!("backtest: {}", date);
        }
        if let Some(ref costs) = cost_config {
            println!(
                "costs: slippage={:.1}bps  spread=${:.4}  commission=${:.4}/sh",
                costs.slippage_bps, costs.half_spread, costs.commission_per_share,
            );
        }
        println!();
    }

    // cross-ticker context (research): per-symbol session-return series for the index
    // and every traded ticker, from the bar cache. requires --bars-dir.
    let cross_series: Option<HashMap<String, HashMap<chrono::DateTime<chrono::Utc>, SessionPoint>>> = match (cross_index, bars_dir) {
        (Some(index), Some(dir)) => {
            let mut m = HashMap::new();
            let mut syms: Vec<String> = config.tickers.clone();
            syms.push(index.to_string());
            for sym in syms {
                match load_cached_bars(dir, &sym, lookback_start, date) {
                    Ok(c) => {
                        m.insert(sym, session_points(&c));
                    }
                    Err(e) => eprintln!("  {:<6} cross-context load error: {}", sym, e),
                }
            }
            Some(m)
        }
        (Some(_), None) => {
            eprintln!("error: --cross-index needs --bars-dir");
            process::exit(1);
        }
        _ => None,
    };

    let mut total_pnl = 0.0;
    #[allow(clippy::type_complexity)]
    let mut ticker_results: Vec<(String, f64, usize, Vec<TradeRecord>, Vec<(TimescaleScores, TimescaleScores)>, Vec<String>, f64, Option<chrono::DateTime<chrono::Utc>>, usize, usize)> = Vec::new();

    for ticker in &config.tickers {
        let loaded = match bars_dir {
            Some(dir) => load_cached_bars(dir, ticker, lookback_start, date),
            None => fetch_bars_range(&api_key, &api_secret, ticker, lookback_start, date).await,
        };
        let candles = match loaded {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  {:<6} error: {}", ticker, e);
                continue;
            }
        };

        if candles.is_empty() {
            eprintln!("  {:<6} no data", ticker);
            eprintln!("DATA_QUALITY:{ticker}:0:{lookback_days}");
            continue;
        }

        // a full trading day has ~390 1-min bars; lookback days should have roughly
        // lookback_days * 390 bars (minus weekends/holidays). emit count for scripts.
        let candle_count = candles.len();
        let expected_min = if lookback_days > 0 {
            // conservative: ~250 trading days/year, so ~70% of calendar days are trading days
            (lookback_days as f64 * 0.65 * 300.0) as usize
        } else {
            200 // single day: at least ~200 bars expected for a partial session
        };
        if candle_count < expected_min {
            eprintln!(
                "  {:<6} WARNING: only {} candles (expected ~{}) — possible rate limiting or data gap",
                ticker, candle_count, expected_min
            );
        }
        eprintln!("DATA_QUALITY:{ticker}:{candle_count}:{lookback_days}");

        let mut backtest_data = build_backtest_data(candles, &required_timescales);
        if let (Some(series), Some(index)) = (cross_series.as_ref(), cross_index) {
            backtest_data.cross_by_ts = Some(build_cross_context(series, index, ticker, &config.tickers));
        }

        // build effective config: base + config-level ticker overrides + CLI ticker overrides
        let mut effective = config.clone();
        if let Some(ovr) = config.ticker_overrides.get(ticker.as_str()) {
            ovr.apply(&mut effective);
        }
        if let Some(ovr) = overrides.ticker_overrides.get(ticker.as_str()) {
            ovr.apply(&mut effective);
        }

        let backtest_config = BacktestConfig {
            ticker: ticker.clone(),
            initial_capital: capital,
            indicator_configs: effective.indicators.clone(),
            action_configs: effective.actions.clone(),
            scoring_config: effective.scoring.clone(),
            cost_config: cost_config.clone(),
            session_config: Some(effective.session.clone()),
            window_exit_overrides: overrides.window_exit_overrides(),
            record_ticks: dump_ticks.is_some(),
        };

        match run_backtest(&backtest_config, &backtest_data) {
            Ok(result) => {
                // filter trades to target date's market hours only,
                // keeping scores parallel with their trades
                let mut filtered_trades = Vec::new();
                let mut filtered_scores = Vec::new();
                let mut filtered_reasons = Vec::new();
                for (i, trade) in result.trades.into_iter().enumerate() {
                    if trade.entry_time >= target_open_utc && trade.entry_time <= target_close_utc {
                        filtered_trades.push(trade);
                        if let Some(scores) = result.trade_scores.get(i) {
                            filtered_scores.push(scores.clone());
                        }
                        filtered_reasons.push(
                            result.trade_entry_reasons.get(i).cloned().unwrap_or_default()
                        );
                    }
                }

                let (metrics, _equity_curve) = compute_metrics(
                    &filtered_trades,
                    backtest_config.initial_capital,
                    target_open_utc,
                    target_close_utc,
                    &[],
                );

                let pnl = metrics.total_pnl;
                let trades = filtered_trades.len();
                total_pnl += pnl;

                if let Some(path) = dump_ticks {
                    // --dump-window-only: stop the dump at session.no_new_entries_after (ET)
                    let dump_close = if std::env::var("BACKTEST_DUMP_WINDOW_ONLY").is_ok() {
                        use chrono::Timelike;
                        let cutoff = &effective.session.no_new_entries_after;
                        let mins: u32 = cutoff.split_once(':').and_then(|(h, m)| Some(h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()?)).unwrap_or(24 * 60);
                        result.ticks.iter().map(|t| t.timestamp).filter(|ts| { let l = ts.with_timezone(&chrono_tz::US::Eastern); l.hour() * 60 + l.minute() < mins }).max().unwrap_or(target_open_utc)
                    } else {
                        target_close_utc
                    };
                    if let Err(e) = append_tick_dump(path, date_str, ticker, &result.ticks, target_open_utc, dump_close) {
                        eprintln!("  {:<6} tick dump error: {}", ticker, e);
                    }
                }

                if write_db && !filtered_trades.is_empty() {
                    match write_backtest_trades(
                        &pool,
                        config_version_id,
                        &filtered_trades,
                        &filtered_scores,
                        &filtered_reasons,
                    )
                    .await
                    {
                        Ok(n) => println!("  {:<6} wrote {} trades to db", ticker, n),
                        Err(e) => eprintln!("  {:<6} db write error: {}", ticker, e),
                    }
                }

                ticker_results.push((ticker.clone(), pnl, trades, filtered_trades, filtered_scores, filtered_reasons, result.max_composite, result.max_composite_time, result.positive_score_ticks, result.total_ticks));
            }
            Err(e) => {
                eprintln!("  {:<6} error: {}", ticker, e);
            }
        }
    }

    if output_trades_csv {
        // CSV mode: trade rows + daily summary rows per ticker, machine-readable
        println!("row_type,date,ticker,direction,entry_time,exit_time,entry_price,exit_price,size,pnl,pnl_pct,hold_duration_ms,exit_reason,entry_reason,candle_pattern,entry_composite,exit_composite,entry_1m,entry_5m,entry_1h,exit_1m,exit_5m,exit_1h,max_composite,max_composite_time,positive_ticks,total_ticks");
        for (ticker, pnl, num_trades, trade_records, scores, entry_reasons, max_comp, max_comp_time, pos_ticks, tot_ticks) in &ticker_results {
            // trade rows
            for (idx, t) in trade_records.iter().enumerate() {
                let dir = match t.direction {
                    types::action::TradeDirection::Long => "Long",
                    types::action::TradeDirection::Short => "Short",
                };
                let (entry_ts, exit_ts) = scores
                    .get(idx)
                    .cloned()
                    .unwrap_or_default();
                let reason = entry_reasons.get(idx).cloned().unwrap_or_default();
                // decode candle pattern from indicator metadata (propagated as candle_5min.pattern_name)
                let candle_pattern = entry_ts.indicator_scores.as_ref()
                    .and_then(|m| m.get("candle_5min.pattern_name"))
                    .and_then(|v| *v)
                    .map(|v| match v as i64 {
                        1 => "bullish_engulfing",
                        -1 => "bearish_engulfing",
                        2 => "hammer",
                        -2 => "shooting_star",
                        -3 => "evening_star",
                        4 => "three_outside_up",
                        -4 => "three_outside_down",
                        _ => "",
                    })
                    .unwrap_or("");
                println!(
                    "trade,{},{},{},{},{},{:.4},{:.4},{:.2},{:.2},{:.6},{},{:?},{},{},{:.4},{:.4},{},{},{},{},{},{},,,",
                    date_str,
                    ticker,
                    dir,
                    t.entry_time.to_rfc3339(),
                    t.exit_time.to_rfc3339(),
                    t.entry_price,
                    t.exit_price,
                    t.size,
                    t.pnl,
                    t.pnl_pct,
                    t.hold_duration_ms,
                    t.exit_reason,
                    reason,
                    candle_pattern,
                    entry_ts.composite,
                    exit_ts.composite,
                    entry_ts.one_minute.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    entry_ts.five_minute.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    entry_ts.one_hour.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    exit_ts.one_minute.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    exit_ts.five_minute.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    exit_ts.one_hour.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                );
            }
            // daily summary row per ticker (emitted even on zero-trade days)
            let max_time_str = max_comp_time.map(|t| t.to_rfc3339()).unwrap_or_default();
            println!(
                "summary,{},{},,,,,,,{:.2},,{},,,,,,,,,,,,{:.4},{},{},{}",
                date_str,
                ticker,
                pnl,
                num_trades,
                max_comp,
                max_time_str,
                pos_ticks,
                tot_ticks,
            );
        }
    } else {
        // human-readable summary
        for (ticker, pnl, trades, trade_records, scores, _entry_reasons, max_comp, max_comp_time, pos_ticks, tot_ticks) in &ticker_results {
            let sign = if *pnl >= 0.0 { "+" } else { "-" };
            println!("  {:<6} {}${:.2}  ({} trades)", ticker, sign, pnl.abs(), trades);

            if verbose {
                for (idx, t) in trade_records.iter().enumerate() {
                    let dir = match t.direction {
                        types::action::TradeDirection::Long => "LONG",
                        types::action::TradeDirection::Short => "SHORT",
                    };
                    let entry_et = t.entry_time.with_timezone(&chrono_tz::US::Eastern);
                    let exit_et = t.exit_time.with_timezone(&chrono_tz::US::Eastern);
                    let pnl_sign = if t.pnl >= 0.0 { "+" } else { "-" };
                    let (entry_score, exit_score) = scores
                        .get(idx)
                        .map(|(e, x)| (e.composite, x.composite))
                        .unwrap_or((0.0, 0.0));
                    println!(
                        "         #{:<2} {:<5} entry {}  ${:.2} [{:.2}]  exit {}  ${:.2} [{:.2}]  {}${:.2}  ({:?})",
                        idx + 1,
                        dir,
                        entry_et.format("%H:%M"),
                        t.entry_price,
                        entry_score,
                        exit_et.format("%H:%M"),
                        t.exit_price,
                        exit_score,
                        pnl_sign,
                        t.pnl.abs(),
                        t.exit_reason,
                    );
                }

                // per-ticker summary line after trade details
                if *trades > 0 {
                    let wins = trade_records.iter().filter(|t| t.pnl > 0.0).count();
                    let win_pct = if *trades > 0 { wins as f64 / *trades as f64 * 100.0 } else { 0.0 };
                    let gross_win: f64 = trade_records.iter().filter(|t| t.pnl > 0.0).map(|t| t.pnl).sum();
                    let gross_loss: f64 = trade_records.iter().filter(|t| t.pnl <= 0.0).map(|t| t.pnl.abs()).sum();
                    let pf = if gross_loss > 0.0 { gross_win / gross_loss } else if gross_win > 0.0 { f64::MAX } else { 0.0 };
                    let pf_str = if pf >= 1000.0 { "inf".to_string() } else { format!("{:.1}", pf) };
                    let ticker_sign = if *pnl >= 0.0 { "+" } else { "-" };
                    println!(
                        "         {:<6} {}${:.2}  ({} trades, {:.0}% win, PF {})",
                        ticker, ticker_sign, pnl.abs(), trades, win_pct, pf_str,
                    );
                }

                // no-trade day diagnostic
                if *trades == 0 {
                    let time_str = max_comp_time.map(|t| {
                        t.with_timezone(&chrono_tz::US::Eastern).format("%H:%M").to_string()
                    }).unwrap_or_else(|| "n/a".to_string());
                    println!(
                        "         (no trades: max composite {:.4} at {}, {}/{} ticks positive)",
                        max_comp, time_str, pos_ticks, tot_ticks,
                    );
                }
            }
        }

        if !ticker_results.is_empty() {
            let total_trades: usize = ticker_results.iter().map(|(_, _, t, _, _, _, _, _, _, _)| t).sum();
            let label = if total_pnl >= 0.0 { "profit" } else { "loss" };
            let sign = if total_pnl >= 0.0 { "+" } else { "-" };
            println!(
                "\n  {:<6} {}${:.2}  {}  ({} trades)",
                "total", sign, total_pnl.abs(), label, total_trades
            );
        }

        if output_equity {
            let ending_equity = capital + total_pnl;
            println!("ENDING_EQUITY={:.2}", ending_equity);
        }
    }
}

fn run_legacy_mode(args: &[String], capital: f64, cost_config: &Option<BacktestCostConfig>) {
    let config_path = get_arg(args, "--config");
    let data_path = get_arg(args, "--data");
    let ticker = get_arg(args, "--ticker");

    if config_path.is_none() || data_path.is_none() || ticker.is_none() {
        eprintln!("usage: backtest --date <YYYY-MM-DD> [options]");
        eprintln!("       backtest --config <path> --data <path> --ticker <name> [options]");
        eprintln!();
        eprintln!("cost model options:");
        eprintln!("  --slippage-bps <N>          slippage in basis points (e.g. 2.0)");
        eprintln!("  --half-spread <N>           half bid-ask spread in dollars (e.g. 0.005)");
        eprintln!("  --commission-per-share <N>  commission per share (e.g. 0.0)");
        eprintln!("  --sec-fee-per-million <N>   SEC fee per million sell-side (e.g. 20.60)");
        eprintln!("  --finra-taf-per-share <N>   FINRA TAF per share sell-side (e.g. 0.000195)");
        process::exit(1);
    }

    let config_path = config_path.unwrap();
    let data_path = data_path.unwrap();
    let ticker = ticker.unwrap();

    // load strategy config from JSON
    let config_json = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading config file {}: {}", config_path, e);
            process::exit(1);
        }
    };

    let strategy_config: StrategyConfig = match serde_json::from_str(&config_json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error parsing config JSON: {}", e);
            process::exit(1);
        }
    };

    // load candle data from CSV
    let csv_data = match fs::read_to_string(&data_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading data file {}: {}", data_path, e);
            process::exit(1);
        }
    };

    let candles = match load_candles_from_csv(csv_data.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error parsing candle CSV: {}", e);
            process::exit(1);
        }
    };

    if candles.is_empty() {
        eprintln!("error: no candle data found in {}", data_path);
        process::exit(1);
    }

    // build backtest config from strategy config
    let backtest_config = BacktestConfig {
        ticker: ticker.clone(),
        initial_capital: capital,
        indicator_configs: strategy_config.indicators,
        action_configs: strategy_config.actions,
        scoring_config: strategy_config.scoring,
        cost_config: cost_config.clone(),
        session_config: Some(strategy_config.session),
        window_exit_overrides: HashMap::new(),
        record_ticks: false,
    };

    let mut candle_map = HashMap::new();
    candle_map.insert(Timescale::FiveMinute, candles);

    let backtest_data = BacktestData {
        candles: candle_map,
        primary_timescale: Timescale::FiveMinute,
        cross_by_ts: None,
    };

    // run backtest
    match run_backtest(&backtest_config, &backtest_data) {
        Ok(result) => match to_json(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("error serializing result: {}", e);
                process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("backtest error: {}", e);
            process::exit(1);
        }
    }
}

fn collect_timescales(config: &StrategyConfig) -> HashSet<Timescale> {
    let mut timescales = HashSet::new();
    for ind in &config.indicators {
        timescales.insert(ind.timescale);
    }
    for ts in config.scoring.timescale_weights.keys() {
        timescales.insert(*ts);
    }
    timescales
}

fn parse_cost_config(args: &[String]) -> Option<BacktestCostConfig> {
    let slippage_bps: Option<f64> = get_arg(args, "--slippage-bps").and_then(|s| s.parse().ok());
    let half_spread: Option<f64> = get_arg(args, "--half-spread").and_then(|s| s.parse().ok());
    let commission: Option<f64> = get_arg(args, "--commission-per-share").and_then(|s| s.parse().ok());
    let sec_fee: Option<f64> = get_arg(args, "--sec-fee-per-million").and_then(|s| s.parse().ok());
    let finra_taf: Option<f64> = get_arg(args, "--finra-taf-per-share").and_then(|s| s.parse().ok());

    if slippage_bps.is_some() || half_spread.is_some() || commission.is_some() || sec_fee.is_some() || finra_taf.is_some() {
        Some(BacktestCostConfig {
            slippage_bps: slippage_bps.unwrap_or(0.0),
            half_spread: half_spread.unwrap_or(0.0),
            commission_per_share: commission.unwrap_or(0.0),
            sec_fee_per_million: sec_fee.unwrap_or(0.0),
            finra_taf_per_share: finra_taf.unwrap_or(0.0),
        })
    } else {
        None
    }
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .rposition(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}


/// load 1-minute RTH candles for `[start_date, end_date]` from `<dir>/<TICKER>.csv`
/// (the file written by `--fetch-bars`; same layout as `load_candles_from_csv`).
fn load_cached_bars(dir: &str, ticker: &str, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<types::market::Candle>, String> {
    let path = format!("{}/{}.csv", dir.trim_end_matches('/'), ticker);
    let file = fs::File::open(&path).map_err(|e| format!("open {path}: {e}"))?;
    let (start_utc, _) = market_hours_utc(start_date)?;
    let (_, end_utc) = market_hours_utc(end_date)?;
    let all = load_candles_from_csv(std::io::BufReader::new(file))?;
    let mut out: Vec<types::market::Candle> = all
        .into_iter()
        .filter(|c| c.timestamp >= start_utc && c.timestamp <= end_utc)
        .collect();
    out.sort_by_key(|c| c.timestamp);
    out.dedup_by_key(|c| c.timestamp);
    Ok(out)
}

/// fetch RTH 1-minute bars in ~20-day chunks and write `<dir>/<TICKER>.csv`
/// (epoch seconds, o, h, l, c, v). existing files are merged, not clobbered.
async fn run_fetch_bars(dir: &str, start: &str, end: &str, tickers: &str) {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID not set");
    let api_secret = std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY not set");
    let start = NaiveDate::parse_from_str(start, "%Y-%m-%d").expect("bad --start");
    let end = NaiveDate::parse_from_str(end, "%Y-%m-%d").expect("bad --end");
    fs::create_dir_all(dir).expect("create bars dir");

    for ticker in tickers.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()) {
        let path = format!("{}/{}.csv", dir.trim_end_matches('/'), ticker);
        let mut by_ts: std::collections::BTreeMap<i64, types::market::Candle> = std::collections::BTreeMap::new();
        if let Ok(f) = fs::File::open(&path) {
            if let Ok(existing) = load_candles_from_csv(std::io::BufReader::new(f)) {
                for c in existing {
                    by_ts.insert(c.timestamp.timestamp(), c);
                }
            }
        }
        let before = by_ts.len();
        let mut chunk_start = start;
        while chunk_start <= end {
            let chunk_end = std::cmp::min(chunk_start + Duration::days(19), end);
            match fetch_bars_range(&api_key, &api_secret, ticker, chunk_start, chunk_end).await {
                Ok(bars) => {
                    let n = bars.len();
                    for c in bars {
                        by_ts.insert(c.timestamp.timestamp(), c);
                    }
                    eprintln!("[fetch-bars] {ticker} {chunk_start}..{chunk_end}: {n} bars");
                }
                Err(e) => {
                    eprintln!("[fetch-bars] {ticker} {chunk_start}..{chunk_end}: ERROR {e}");
                }
            }
            chunk_start = chunk_end + Duration::days(1);
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
        let mut out = String::from("timestamp,open,high,low,close,volume\n");
        for (ts, c) in &by_ts {
            out.push_str(&format!("{},{},{},{},{},{}\n", ts, c.open, c.high, c.low, c.close, c.volume));
        }
        fs::write(&path, out).expect("write bars csv");
        eprintln!("[fetch-bars] {ticker}: {} -> {} bars in {path}", before, by_ts.len());
    }
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// append the target day's tick rows to `path` (header written once).
fn append_tick_dump(
    path: &str,
    date_str: &str,
    ticker: &str,
    ticks: &[backtest::replay::TickRow],
    open_utc: chrono::DateTime<chrono::Utc>,
    close_utc: chrono::DateTime<chrono::Utc>,
) -> Result<(), String> {
    use std::io::Write;
    let need_header = fs::metadata(path).map(|m| m.len() == 0).unwrap_or(true);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open {path}: {e}"))?;
    let mut buf = String::new();
    if need_header {
        buf.push_str("date,ticker,ts,open,high,low,close,volume,vwap,composite,s1m,s5m,s1h,position,unrealized_pct,hold_min,event,entry_reason,blocked_by,near_miss,indicators\n");
    }
    let opt = |v: Option<f64>| v.map(|x| format!("{:.4}", x)).unwrap_or_default();
    for t in ticks.iter().filter(|t| t.timestamp >= open_utc && t.timestamp <= close_utc) {
        buf.push_str(&format!(
            "{},{},{},{:.4},{:.4},{:.4},{:.4},{},{:.4},{:.4},{},{},{},{},{},{},{},{},{},{},{}\n",
            date_str,
            ticker,
            t.timestamp.to_rfc3339(),
            t.open, t.high, t.low, t.close, t.volume, t.session_vwap,
            t.composite,
            opt(t.one_minute), opt(t.five_minute), opt(t.one_hour),
            t.position,
            opt(t.unrealized_pct),
            t.hold_min.map(|h| h.to_string()).unwrap_or_default(),
            t.event,
            csv_quote(&t.entry_reason),
            csv_quote(&t.entry_blocked_by),
            csv_quote(&t.near_miss),
            csv_quote(&indicator_json(&t.indicator_scores)),
        ));
    }
    f.write_all(buf.as_bytes()).map_err(|e| format!("write {path}: {e}"))
}

/// compact json of the per-indicator scores: `{"ofi_1m":0.12,"vpin_5m":null,...}`, keys sorted.
fn indicator_json(m: &HashMap<String, Option<f64>>) -> String {
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    let mut out = String::from("{");
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        match m[*k] {
            Some(v) if v.is_finite() => out.push_str(&format!("\"{}\":{:.4}", k, v)),
            _ => out.push_str(&format!("\"{}\":null", k)),
        }
    }
    out.push('}');
    out
}

/// per-bar session state of one symbol, for cross-ticker context.
#[derive(Debug, Clone, Copy)]
struct SessionPoint {
    session_ret: f64,
    ret_5m: f64,
    ret_15m: f64,
}

/// session return (vs the first RTH bar's open of that eastern date) and short lookback
/// returns for every bar of a 1-minute series.
fn session_points(candles: &[types::market::Candle]) -> HashMap<chrono::DateTime<chrono::Utc>, SessionPoint> {
    use chrono_tz::US::Eastern;
    let mut out = HashMap::with_capacity(candles.len());
    let mut day: Option<chrono::NaiveDate> = None;
    let mut day_open = 0.0_f64;
    let mut closes: Vec<f64> = Vec::new();
    for c in candles {
        let d = c.timestamp.with_timezone(&Eastern).date_naive();
        if day != Some(d) {
            day = Some(d);
            day_open = c.open;
            closes.clear();
        }
        closes.push(c.close);
        let n = closes.len();
        let r = |k: usize| if n > k { c.close / closes[n - 1 - k] - 1.0 } else { 0.0 };
        out.insert(
            c.timestamp,
            SessionPoint { session_ret: c.close / day_open - 1.0, ret_5m: r(5), ret_15m: r(15) },
        );
    }
    out
}

/// cross context for `ticker`: index series + the other traded tickers as peers.
fn build_cross_context(
    series: &HashMap<String, HashMap<chrono::DateTime<chrono::Utc>, SessionPoint>>,
    index: &str,
    ticker: &str,
    tickers: &[String],
) -> HashMap<chrono::DateTime<chrono::Utc>, types::market::CrossContext> {
    let Some(idx) = series.get(index) else { return HashMap::new() };
    // --cross-lag N: serve the index/peer state from N minutes earlier (live sees each
    // symbol's latest bar, which within a minute may still be the previous one)
    let lag_min: i64 = std::env::var("BACKTEST_CROSS_LAG").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let peers: Vec<&HashMap<_, SessionPoint>> = tickers
        .iter()
        .filter(|t| t.as_str() != ticker)
        .filter_map(|t| series.get(t))
        .collect();
    let mut out = HashMap::with_capacity(idx.len());
    for (ts0, ip) in idx {
        let ts = &(*ts0 + chrono::Duration::minutes(lag_min));
        let mut red = 0usize;
        let mut n = 0usize;
        let mut sum = 0.0;
        for p in &peers {
            if let Some(sp) = p.get(ts0) {
                n += 1;
                sum += sp.session_ret;
                if sp.session_ret < 0.0 {
                    red += 1;
                }
            }
        }
        out.insert(
            *ts,
            types::market::CrossContext {
                index_session_ret: ip.session_ret,
                index_ret_5m: ip.ret_5m,
                index_ret_15m: ip.ret_15m,
                peers_red_frac: if n > 0 { red as f64 / n as f64 } else { 0.5 },
                peers_mean_session_ret: if n > 0 { sum / n as f64 } else { 0.0 },
            },
        );
    }
    out
}
