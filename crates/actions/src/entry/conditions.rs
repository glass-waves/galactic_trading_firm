use std::collections::HashMap;

use types::market::Timescale;
use types::scoring::TimescaleScores;

/// a single boolean condition evaluated against timescale scores and/or indicator scores.
/// designed as a standalone, reusable module with no dependency on the Action trait.
#[derive(Debug, Clone)]
pub enum WindowCondition {
    /// timescale score >= min_score
    TimescaleMin {
        timescale: Timescale,
        min_score: f64,
    },
    /// timescale score <= max_score
    TimescaleMax {
        timescale: Timescale,
        max_score: f64,
    },
    /// timescale score in [min_score, max_score]
    TimescaleRange {
        timescale: Timescale,
        min_score: f64,
        max_score: f64,
    },
    /// timescale leads all other available timescales by >= lead_by
    TimescaleLead {
        timescale: Timescale,
        lead_by: f64,
    },
    /// max(all available) - min(all available) <= max_spread
    TimescaleSpreadMax {
        max_spread: f64,
    },
    /// all available timescale scores >= min_score
    TimescaleAllMin {
        min_score: f64,
    },
    /// specific indicator score >= min_score
    IndicatorMin {
        instance_id: String,
        min_score: f64,
    },
    /// specific indicator score <= max_score
    IndicatorMax {
        instance_id: String,
        max_score: f64,
    },
    /// specific indicator score in [min_score, max_score]
    IndicatorRange {
        instance_id: String,
        min_score: f64,
        max_score: f64,
    },
    /// composite score >= min_score (floor gate)
    CompositeMin {
        min_score: f64,
    },
    /// all available timescale scores <= max_score (mirror of TimescaleAllMin)
    TimescaleAllMax {
        max_score: f64,
    },
    /// composite score <= max_score (ceiling gate, for short windows)
    CompositeMax {
        max_score: f64,
    },
    /// timescale lags all other available timescales by >= lag_by
    /// (mirror of TimescaleLead, for short windows)
    TimescaleLag {
        timescale: Timescale,
        lag_by: f64,
    },
}

fn get_ts_score(scores: &TimescaleScores, ts: &Timescale) -> Option<f64> {
    match ts {
        Timescale::OneMinute => scores.one_minute,
        Timescale::FiveMinute => scores.five_minute,
        Timescale::OneHour => scores.one_hour,
        Timescale::OneDay => scores.one_day,
        Timescale::OneMonth => scores.one_month,
    }
}

fn available_ts_scores(scores: &TimescaleScores) -> Vec<f64> {
    [
        scores.one_minute,
        scores.five_minute,
        scores.one_hour,
        scores.one_day,
        scores.one_month,
    ]
    .iter()
    .filter_map(|s| *s)
    .collect()
}

fn get_indicator_score(
    indicator_scores: Option<&HashMap<String, Option<f64>>>,
    instance_id: &str,
) -> Option<f64> {
    indicator_scores
        .and_then(|m| m.get(instance_id))
        .and_then(|v| *v)
}

impl WindowCondition {
    /// evaluate this condition. returns true if the condition is met.
    /// missing timescale scores cause the condition to fail (conservative).
    /// missing indicator scores cause indicator conditions to fail (conservative).
    pub fn evaluate(
        &self,
        scores: &TimescaleScores,
        indicator_scores: Option<&HashMap<String, Option<f64>>>,
    ) -> bool {
        match self {
            WindowCondition::TimescaleMin { timescale, min_score } => {
                get_ts_score(scores, timescale)
                    .map(|s| s >= *min_score)
                    .unwrap_or(false)
            }
            WindowCondition::TimescaleMax { timescale, max_score } => {
                get_ts_score(scores, timescale)
                    .map(|s| s <= *max_score)
                    .unwrap_or(false)
            }
            WindowCondition::TimescaleRange { timescale, min_score, max_score } => {
                get_ts_score(scores, timescale)
                    .map(|s| s >= *min_score && s <= *max_score)
                    .unwrap_or(false)
            }
            WindowCondition::TimescaleLead { timescale, lead_by } => {
                let lead_score = match get_ts_score(scores, timescale) {
                    Some(s) => s,
                    None => return false,
                };
                let available = available_ts_scores(scores);
                if available.len() <= 1 {
                    return available.len() == 1; // trivially leads if only one
                }
                // must lead ALL other available timescales by >= lead_by
                let all_timescales = [
                    Timescale::OneMinute,
                    Timescale::FiveMinute,
                    Timescale::OneHour,
                    Timescale::OneDay,
                    Timescale::OneMonth,
                ];
                for ts in &all_timescales {
                    if ts == timescale {
                        continue;
                    }
                    if let Some(other) = get_ts_score(scores, ts) {
                        if lead_score < other + *lead_by {
                            return false;
                        }
                    }
                }
                true
            }
            WindowCondition::TimescaleSpreadMax { max_spread } => {
                let available = available_ts_scores(scores);
                if available.len() <= 1 {
                    return true; // trivially within spread
                }
                let max_val = available.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let min_val = available.iter().cloned().fold(f64::INFINITY, f64::min);
                (max_val - min_val) <= *max_spread
            }
            WindowCondition::TimescaleAllMin { min_score } => {
                let available = available_ts_scores(scores);
                if available.is_empty() {
                    return false;
                }
                available.iter().all(|s| *s >= *min_score)
            }
            WindowCondition::IndicatorMin { instance_id, min_score } => {
                get_indicator_score(indicator_scores, instance_id)
                    .map(|s| s >= *min_score)
                    .unwrap_or(false)
            }
            WindowCondition::IndicatorMax { instance_id, max_score } => {
                get_indicator_score(indicator_scores, instance_id)
                    .map(|s| s <= *max_score)
                    .unwrap_or(false)
            }
            WindowCondition::IndicatorRange { instance_id, min_score, max_score } => {
                get_indicator_score(indicator_scores, instance_id)
                    .map(|s| s >= *min_score && s <= *max_score)
                    .unwrap_or(false)
            }
            WindowCondition::CompositeMin { min_score } => {
                scores.composite >= *min_score
            }
            WindowCondition::CompositeMax { max_score } => {
                scores.composite <= *max_score
            }
            WindowCondition::TimescaleAllMax { max_score } => {
                let available = available_ts_scores(scores);
                !available.is_empty() && available.iter().all(|s| *s <= *max_score)
            }
            WindowCondition::TimescaleLag { timescale, lag_by } => {
                let lag_score = match get_ts_score(scores, timescale) {
                    Some(s) => s,
                    None => return false,
                };
                let available = available_ts_scores(scores);
                if available.len() <= 1 {
                    return available.len() == 1;
                }
                let all_timescales = [
                    Timescale::OneMinute,
                    Timescale::FiveMinute,
                    Timescale::OneHour,
                    Timescale::OneDay,
                    Timescale::OneMonth,
                ];
                for ts in &all_timescales {
                    if ts == timescale {
                        continue;
                    }
                    if let Some(other) = get_ts_score(scores, ts) {
                        if lag_score > other - *lag_by {
                            return false;
                        }
                    }
                }
                true
            }
        }
    }

    /// short description of this condition's outcome against the given scores,
    /// e.g. "FiveMinute 0.31<0.50" or "candle_5min none<0.40". used for near-miss
    /// diagnostics; never allocates unless called.
    pub fn describe(
        &self,
        scores: &TimescaleScores,
        indicator_scores: Option<&HashMap<String, Option<f64>>>,
    ) -> String {
        fn fmt(v: Option<f64>) -> String {
            v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "none".to_string())
        }
        match self {
            WindowCondition::TimescaleMin { timescale, min_score } => {
                format!("{timescale:?} {}<{min_score:.2}", fmt(get_ts_score(scores, timescale)))
            }
            WindowCondition::TimescaleMax { timescale, max_score } => {
                format!("{timescale:?} {}>{max_score:.2}", fmt(get_ts_score(scores, timescale)))
            }
            WindowCondition::TimescaleRange { timescale, min_score, max_score } => {
                format!(
                    "{timescale:?} {} not in [{min_score:.2},{max_score:.2}]",
                    fmt(get_ts_score(scores, timescale))
                )
            }
            WindowCondition::TimescaleLead { timescale, lead_by } => {
                format!("{timescale:?} {} not leading by {lead_by:.2}", fmt(get_ts_score(scores, timescale)))
            }
            WindowCondition::TimescaleSpreadMax { max_spread } => {
                format!("spread>{max_spread:.2}")
            }
            WindowCondition::TimescaleAllMin { min_score } => format!("all<{min_score:.2}"),
            WindowCondition::IndicatorMin { instance_id, min_score } => {
                format!("{instance_id} {}<{min_score:.2}", fmt(get_indicator_score(indicator_scores, instance_id)))
            }
            WindowCondition::IndicatorMax { instance_id, max_score } => {
                format!("{instance_id} {}>{max_score:.2}", fmt(get_indicator_score(indicator_scores, instance_id)))
            }
            WindowCondition::IndicatorRange { instance_id, min_score, max_score } => {
                format!(
                    "{instance_id} {} not in [{min_score:.2},{max_score:.2}]",
                    fmt(get_indicator_score(indicator_scores, instance_id))
                )
            }
            WindowCondition::CompositeMin { min_score } => {
                format!("composite {:.2}<{min_score:.2}", scores.composite)
            }
            WindowCondition::CompositeMax { max_score } => {
                format!("composite {:.2}>{max_score:.2}", scores.composite)
            }
            WindowCondition::TimescaleAllMax { max_score } => format!("all>{max_score:.2}"),
            WindowCondition::TimescaleLag { timescale, lag_by } => {
                format!("{timescale:?} {} not lagging by {lag_by:.2}", fmt(get_ts_score(scores, timescale)))
            }
        }
    }

    /// parse a condition from a JSON value (config params format).
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let cond_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("condition missing 'type' field")?;

        match cond_type {
            "timescale_min" => Ok(WindowCondition::TimescaleMin {
                timescale: parse_timescale(value)?,
                min_score: parse_f64(value, "min_score")?,
            }),
            "timescale_max" => Ok(WindowCondition::TimescaleMax {
                timescale: parse_timescale(value)?,
                max_score: parse_f64(value, "max_score")?,
            }),
            "timescale_range" => Ok(WindowCondition::TimescaleRange {
                timescale: parse_timescale(value)?,
                min_score: parse_f64(value, "min_score")?,
                max_score: parse_f64(value, "max_score")?,
            }),
            "timescale_lead" => Ok(WindowCondition::TimescaleLead {
                timescale: parse_timescale(value)?,
                lead_by: parse_f64(value, "lead_by")?,
            }),
            "timescale_spread_max" => Ok(WindowCondition::TimescaleSpreadMax {
                max_spread: parse_f64(value, "max_spread")?,
            }),
            "timescale_all_min" => Ok(WindowCondition::TimescaleAllMin {
                min_score: parse_f64(value, "min_score")?,
            }),
            "indicator_min" => Ok(WindowCondition::IndicatorMin {
                instance_id: parse_string(value, "instance_id")?,
                min_score: parse_f64(value, "min_score")?,
            }),
            "indicator_max" => Ok(WindowCondition::IndicatorMax {
                instance_id: parse_string(value, "instance_id")?,
                max_score: parse_f64(value, "max_score")?,
            }),
            "indicator_range" => Ok(WindowCondition::IndicatorRange {
                instance_id: parse_string(value, "instance_id")?,
                min_score: parse_f64(value, "min_score")?,
                max_score: parse_f64(value, "max_score")?,
            }),
            "composite_min" => Ok(WindowCondition::CompositeMin {
                min_score: parse_f64(value, "min_score")?,
            }),
            "timescale_all_max" => Ok(WindowCondition::TimescaleAllMax {
                max_score: parse_f64(value, "max_score")?,
            }),
            "composite_max" => Ok(WindowCondition::CompositeMax {
                max_score: parse_f64(value, "max_score")?,
            }),
            "timescale_lag" => Ok(WindowCondition::TimescaleLag {
                timescale: parse_timescale(value)?,
                lag_by: parse_f64(value, "lag_by")?,
            }),
            other => Err(format!("unknown condition type: '{}'", other)),
        }
    }
}

/// evaluate ALL conditions (AND logic). returns true only if every condition passes.
pub fn all_conditions_met(
    conditions: &[WindowCondition],
    scores: &TimescaleScores,
    indicator_scores: Option<&HashMap<String, Option<f64>>>,
) -> bool {
    conditions
        .iter()
        .all(|c| c.evaluate(scores, indicator_scores))
}

/// describe every failing condition, joined with "; ". empty when all pass.
pub fn failing_conditions(
    conditions: &[WindowCondition],
    scores: &TimescaleScores,
    indicator_scores: Option<&HashMap<String, Option<f64>>>,
) -> Vec<String> {
    conditions
        .iter()
        .filter(|c| !c.evaluate(scores, indicator_scores))
        .map(|c| c.describe(scores, indicator_scores))
        .collect()
}

/// parse multiple conditions from a JSON array in config params.
pub fn parse_conditions(value: &serde_json::Value) -> Result<Vec<WindowCondition>, String> {
    let arr = value
        .as_array()
        .ok_or("'conditions' must be a JSON array")?;
    arr.iter().map(WindowCondition::from_json).collect()
}

fn parse_timescale(value: &serde_json::Value) -> Result<Timescale, String> {
    let ts_str = value
        .get("timescale")
        .and_then(|v| v.as_str())
        .ok_or("condition missing 'timescale' field")?;
    match ts_str {
        "OneMinute" | "1m" => Ok(Timescale::OneMinute),
        "FiveMinute" | "5m" => Ok(Timescale::FiveMinute),
        "OneHour" | "1h" => Ok(Timescale::OneHour),
        "OneDay" | "1d" => Ok(Timescale::OneDay),
        "OneMonth" | "1M" => Ok(Timescale::OneMonth),
        other => Err(format!("unknown timescale: '{}'", other)),
    }
}

fn parse_f64(value: &serde_json::Value, field: &str) -> Result<f64, String> {
    value
        .get(field)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("condition missing or invalid '{}' field", field))
}

fn parse_string(value: &serde_json::Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("condition missing '{}' field", field))
}
