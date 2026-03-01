use std::collections::HashMap;

use crate::action::{Action, ActionConfig};
use crate::indicator::{Indicator, IndicatorConfig};

pub type IndicatorFactory = Box<dyn Fn(&IndicatorConfig) -> Box<dyn Indicator>>;
pub type ActionFactory = Box<dyn Fn(&ActionConfig) -> Box<dyn Action>>;

/// registry that maps type names to factory functions.
pub struct IndicatorRegistry {
    pub factories: HashMap<String, IndicatorFactory>,
}

impl IndicatorRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn(&IndicatorConfig) -> Box<dyn Indicator> + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
    }
}

impl Default for IndicatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ActionRegistry {
    pub factories: HashMap<String, ActionFactory>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn(&ActionConfig) -> Box<dyn Action> + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// runtime state: the loaded tool belt.
pub struct ToolBelt {
    /// active indicator instances, keyed by instance_id.
    pub indicators: HashMap<String, Box<dyn Indicator>>,

    /// active action instances, grouped by phase.
    pub entry_actions: Vec<Box<dyn Action>>,
    pub monitor_actions: Vec<Box<dyn Action>>,
    pub exit_actions: Vec<Box<dyn Action>>,
    pub sizing_actions: Vec<Box<dyn Action>>,
}
