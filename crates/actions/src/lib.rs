pub mod entry;
pub mod exit;
pub mod monitor;
pub mod sizing;

use types::action::{Action, ActionConfig, ActionPhase};
use types::registry::ActionRegistry;

/// create a registry pre-populated with all action factories.
pub fn default_action_registry() -> ActionRegistry {
    let mut reg = ActionRegistry::new();
    reg.register("score_threshold_entry", entry::score_threshold::score_threshold_entry_factory);
    reg.register("atr_trailing_stop", exit::atr_trailing_stop::atr_trailing_stop_factory);
    reg.register("fixed_pct_stop", exit::fixed_pct_stop::fixed_pct_stop_factory);
    reg.register("session_close", exit::session_close::session_close_factory);
    reg.register("max_hold_timeout", exit::max_hold_timeout::max_hold_timeout_factory);
    reg.register("breakeven_stop", monitor::breakeven_stop::breakeven_stop_factory);
    reg.register("fixed_fractional", sizing::fixed_fractional::fixed_fractional_factory);
    reg.register("score_scaled", sizing::score_scaled::score_scaled_factory);
    reg.register("volatility_scaled", sizing::volatility_scaled::volatility_scaled_factory);
    reg
}

/// build action instances from configs, grouped by phase and sorted by priority.
/// disabled actions are skipped. unknown types produce an error.
pub fn build_actions(
    configs: &[ActionConfig],
    registry: &ActionRegistry,
) -> Result<ActionsByPhase, String> {
    let mut entry_actions = Vec::new();
    let mut monitor_actions = Vec::new();
    let mut exit_actions = Vec::new();
    let mut sizing_actions = Vec::new();

    for cfg in configs {
        if !cfg.enabled {
            continue;
        }
        let factory = registry
            .factories
            .get(&cfg.action_type)
            .ok_or_else(|| format!("unknown action type: '{}'", cfg.action_type))?;
        let action = factory(cfg);

        match cfg.phase {
            ActionPhase::Entry => entry_actions.push((cfg.priority, action)),
            ActionPhase::Monitor => monitor_actions.push((cfg.priority, action)),
            ActionPhase::Exit => exit_actions.push((cfg.priority, action)),
            ActionPhase::Sizing => sizing_actions.push((cfg.priority, action)),
        }
    }

    // sort by priority (lower = evaluated first)
    entry_actions.sort_by_key(|(p, _)| *p);
    monitor_actions.sort_by_key(|(p, _)| *p);
    exit_actions.sort_by_key(|(p, _)| *p);
    sizing_actions.sort_by_key(|(p, _)| *p);

    Ok(ActionsByPhase {
        entry: entry_actions.into_iter().map(|(_, a)| a).collect(),
        monitor: monitor_actions.into_iter().map(|(_, a)| a).collect(),
        exit: exit_actions.into_iter().map(|(_, a)| a).collect(),
        sizing: sizing_actions.into_iter().map(|(_, a)| a).collect(),
    })
}

pub struct ActionsByPhase {
    pub entry: Vec<Box<dyn Action>>,
    pub monitor: Vec<Box<dyn Action>>,
    pub exit: Vec<Box<dyn Action>>,
    pub sizing: Vec<Box<dyn Action>>,
}
