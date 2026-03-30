use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position};
use types::market::MarketState;
use types::scoring::TimescaleScores;

/// a profit tier: when unrealized_pnl_pct >= threshold, grant extension_ms of extra hold time.
#[derive(Debug, Clone)]
pub struct ProfitTier {
    pub pnl_pct: f64,
    pub extension_ms: i64,
}

#[allow(dead_code)]
pub struct MaxHoldTimeout {
    max_hold_ms: i64,
    /// legacy flat extension when profitable (used when no tiers match or no score gate met).
    profit_extension_ms: i64,
    /// ms to subtract from hold when position is losing (floor at 0).
    loss_reduction_ms: i64,
    /// minimum composite score required for tiered profit extension.
    /// when composite < score_gate, falls back to legacy profit_extension_ms.
    score_gate: f64,
    /// profit tiers sorted by pnl_pct ascending. highest matching tier wins.
    profit_tiers: Vec<ProfitTier>,
    instance_id: String,
}

impl MaxHoldTimeout {
    pub fn new(
        max_hold_ms: i64,
        profit_extension_ms: i64,
        loss_reduction_ms: i64,
        instance_id: String,
    ) -> Self {
        Self {
            max_hold_ms,
            profit_extension_ms,
            loss_reduction_ms,
            score_gate: 0.0,
            profit_tiers: Vec::new(),
            instance_id,
        }
    }

    pub fn with_tiers(
        max_hold_ms: i64,
        profit_extension_ms: i64,
        loss_reduction_ms: i64,
        score_gate: f64,
        mut profit_tiers: Vec<ProfitTier>,
        instance_id: String,
    ) -> Self {
        profit_tiers.sort_by(|a, b| a.pnl_pct.partial_cmp(&b.pnl_pct).unwrap());
        Self {
            max_hold_ms,
            profit_extension_ms,
            loss_reduction_ms,
            score_gate,
            profit_tiers,
            instance_id,
        }
    }

    /// compute extension from profit tiers. returns the extension_ms of the highest
    /// matching tier, or None if no tier is reached.
    fn tier_extension(&self, pnl_pct: f64) -> Option<i64> {
        self.profit_tiers
            .iter()
            .rev()
            .find(|t| pnl_pct >= t.pnl_pct)
            .map(|t| t.extension_ms)
    }
}

impl Action for MaxHoldTimeout {
    fn name(&self) -> &str {
        "max_hold_timeout"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Exit
    }

    fn evaluate(
        &self,
        position: Option<&Position>,
        _market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal {
        let pos = match position {
            Some(p) => p,
            None => return ActionSignal::Hold,
        };

        let effective_hold = if pos.unrealized_pnl > 0.0 {
            // profitable: check tiered extension with score gate
            let tier_ext = if !self.profit_tiers.is_empty()
                && scores.composite >= self.score_gate
            {
                self.tier_extension(pos.unrealized_pnl_pct)
            } else {
                None
            };
            // use tier extension if available, otherwise legacy flat extension
            self.max_hold_ms + tier_ext.unwrap_or(self.profit_extension_ms)
        } else if pos.unrealized_pnl < 0.0 {
            (self.max_hold_ms - self.loss_reduction_ms).max(0)
        } else {
            self.max_hold_ms
        };

        if pos.hold_duration_ms >= effective_hold {
            ActionSignal::Exit {
                reason: ExitReason::MaxHoldTimeout,
            }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn max_hold_timeout_factory(config: &ActionConfig) -> Box<dyn Action> {
    let ms = config
        .params
        .get("max_hold_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(3_600_000); // default 1 hour
    let profit_ext = config
        .params
        .get("profit_extension_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let loss_red = config
        .params
        .get("loss_reduction_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let score_gate = config
        .params
        .get("score_gate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // parse profit tiers: tier1_pnl_pct/tier1_extension_ms, tier2_..., tier3_...
    let mut tiers = Vec::new();
    for i in 1..=5 {
        let pnl_key = format!("tier{i}_pnl_pct");
        let ext_key = format!("tier{i}_extension_ms");
        if let (Some(pnl), Some(ext)) = (
            config.params.get(&pnl_key).and_then(|v| v.as_f64()),
            config.params.get(&ext_key).and_then(|v| v.as_i64()),
        ) {
            tiers.push(ProfitTier {
                pnl_pct: pnl,
                extension_ms: ext,
            });
        }
    }

    if tiers.is_empty() {
        Box::new(MaxHoldTimeout::new(ms, profit_ext, loss_red, config.instance_id.clone()))
    } else {
        Box::new(MaxHoldTimeout::with_tiers(
            ms, profit_ext, loss_red, score_gate, tiers, config.instance_id.clone(),
        ))
    }
}
