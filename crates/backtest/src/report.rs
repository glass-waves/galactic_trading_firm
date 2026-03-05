use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Write;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use engine::TradeRecord;
use types::action::ExitReason;
use types::scoring::TimescaleScores;

use crate::replay::TickEquityPoint;

/// a point on the equity curve, captured after each trade closes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: DateTime<Utc>,
    pub equity: f64,
    pub drawdown_pct: f64,
}

/// breakdown of performance by exit reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitReasonBreakdown {
    pub count: usize,
    pub total_pnl: f64,
    pub avg_pnl: f64,
    pub win_rate: f64,
}

/// computed performance metrics from a backtest run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub win_rate: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,
    /// daily-return sharpe ratio (annualized, sqrt(252)).
    pub sharpe_ratio: f64,
    /// per-trade return sharpe ratio (legacy metric for comparison).
    #[serde(default)]
    pub trade_sharpe_ratio: f64,
    /// max drawdown from trade-exit-level equity curve.
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,
    /// max drawdown from tick-level equity curve (captures intra-trade drawdowns).
    #[serde(default)]
    pub max_tick_drawdown: f64,
    #[serde(default)]
    pub max_tick_drawdown_pct: f64,
    pub avg_hold_duration_ms: i64,
    pub trades_per_day: f64,
    pub by_exit_reason: HashMap<ExitReason, ExitReasonBreakdown>,
}

/// complete result of a backtest run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub config_id: String,
    pub ticker: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub initial_capital: f64,
    pub trades: Vec<TradeRecord>,
    /// entry and exit scores for each trade, parallel to `trades`.
    #[serde(default)]
    pub trade_scores: Vec<(TimescaleScores, TimescaleScores)>,
    pub equity_curve: Vec<EquityPoint>,
    pub metrics: BacktestMetrics,
}

/// compute metrics from a list of completed trades.
/// `initial_capital` is used for P&L percentage and drawdown calculations.
/// `start_time` and `end_time` define the backtest period for trades_per_day.
/// `tick_equity` provides tick-level equity points for intra-trade drawdown calculation.
pub fn compute_metrics(
    trades: &[TradeRecord],
    initial_capital: f64,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    tick_equity: &[TickEquityPoint],
) -> (BacktestMetrics, Vec<EquityPoint>) {
    if trades.is_empty() {
        return (
            BacktestMetrics {
                total_pnl: 0.0,
                total_pnl_pct: 0.0,
                win_rate: 0.0,
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                avg_win: 0.0,
                avg_loss: 0.0,
                profit_factor: 0.0,
                sharpe_ratio: 0.0,
                trade_sharpe_ratio: 0.0,
                max_drawdown: 0.0,
                max_drawdown_pct: 0.0,
                max_tick_drawdown: 0.0,
                max_tick_drawdown_pct: 0.0,
                avg_hold_duration_ms: 0,
                trades_per_day: 0.0,
                by_exit_reason: HashMap::new(),
            },
            vec![],
        );
    }

    let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
    let total_pnl_pct = total_pnl / initial_capital;

    let winning: Vec<&TradeRecord> = trades.iter().filter(|t| t.pnl > 0.0).collect();
    let losing: Vec<&TradeRecord> = trades.iter().filter(|t| t.pnl <= 0.0).collect();

    let winning_trades = winning.len();
    let losing_trades = losing.len();
    let total_trades = trades.len();
    let win_rate = winning_trades as f64 / total_trades as f64;

    let avg_win = if winning.is_empty() {
        0.0
    } else {
        winning.iter().map(|t| t.pnl).sum::<f64>() / winning.len() as f64
    };

    let avg_loss = if losing.is_empty() {
        0.0
    } else {
        losing.iter().map(|t| t.pnl).sum::<f64>() / losing.len() as f64
    };

    let gross_profit: f64 = winning.iter().map(|t| t.pnl).sum();
    let gross_loss: f64 = losing.iter().map(|t| t.pnl.abs()).sum();
    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else if gross_profit > 0.0 {
        f64::MAX
    } else {
        0.0
    };

    // trade-exit-level equity curve and drawdown
    let mut equity = initial_capital;
    let mut peak_equity = initial_capital;
    let mut max_drawdown = 0.0_f64;
    let mut max_drawdown_pct = 0.0_f64;
    let mut equity_curve = Vec::with_capacity(trades.len());

    for trade in trades {
        equity += trade.pnl;
        if equity > peak_equity {
            peak_equity = equity;
        }
        let dd = peak_equity - equity;
        let dd_pct = if peak_equity > 0.0 {
            dd / peak_equity
        } else {
            0.0
        };
        if dd > max_drawdown {
            max_drawdown = dd;
        }
        if dd_pct > max_drawdown_pct {
            max_drawdown_pct = dd_pct;
        }
        equity_curve.push(EquityPoint {
            timestamp: trade.exit_time,
            equity,
            drawdown_pct: dd_pct,
        });
    }

    // tick-level drawdown (captures intra-trade drawdowns)
    let mut max_tick_drawdown = 0.0_f64;
    let mut max_tick_drawdown_pct = 0.0_f64;
    let mut tick_peak = initial_capital;
    for point in tick_equity {
        if point.equity > tick_peak {
            tick_peak = point.equity;
        }
        let dd = tick_peak - point.equity;
        let dd_pct = if tick_peak > 0.0 { dd / tick_peak } else { 0.0 };
        if dd > max_tick_drawdown {
            max_tick_drawdown = dd;
        }
        if dd_pct > max_tick_drawdown_pct {
            max_tick_drawdown_pct = dd_pct;
        }
    }

    // per-trade sharpe ratio (legacy metric)
    let pnl_pcts: Vec<f64> = trades.iter().map(|t| t.pnl_pct).collect();
    let mean_return = pnl_pcts.iter().sum::<f64>() / pnl_pcts.len() as f64;
    let variance = pnl_pcts
        .iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>()
        / pnl_pcts.len() as f64;
    let std_return = variance.sqrt();

    let trading_days = (end_time - start_time).num_seconds() as f64 / 86400.0;
    let trades_per_day = if trading_days > 0.0 {
        total_trades as f64 / trading_days
    } else {
        0.0
    };

    let trade_sharpe_ratio = if std_return > 0.0 {
        let annualization = (252.0 * trades_per_day).sqrt();
        mean_return / std_return * annualization
    } else {
        0.0
    };

    // daily-return sharpe ratio: build daily P&L series from trades
    let daily_returns = compute_daily_returns(trades, initial_capital);
    let sharpe_ratio = if daily_returns.len() > 1 {
        let n = daily_returns.len() as f64;
        let mean_daily = daily_returns.iter().sum::<f64>() / n;
        let var_daily = daily_returns
            .iter()
            .map(|r| (r - mean_daily).powi(2))
            .sum::<f64>()
            / (n - 1.0); // sample variance (n-1)
        let std_daily = var_daily.sqrt();
        if std_daily > 0.0 {
            mean_daily / std_daily * 252.0_f64.sqrt()
        } else {
            0.0
        }
    } else {
        0.0
    };

    let avg_hold_duration_ms =
        trades.iter().map(|t| t.hold_duration_ms).sum::<i64>() / total_trades as i64;

    // breakdown by exit reason
    let mut by_exit_reason: HashMap<ExitReason, Vec<&TradeRecord>> = HashMap::new();
    for trade in trades {
        by_exit_reason
            .entry(trade.exit_reason.clone())
            .or_default()
            .push(trade);
    }
    let by_exit_reason = by_exit_reason
        .into_iter()
        .map(|(reason, group)| {
            let count = group.len();
            let total = group.iter().map(|t| t.pnl).sum::<f64>();
            let wins = group.iter().filter(|t| t.pnl > 0.0).count();
            (
                reason,
                ExitReasonBreakdown {
                    count,
                    total_pnl: total,
                    avg_pnl: total / count as f64,
                    win_rate: wins as f64 / count as f64,
                },
            )
        })
        .collect();

    (
        BacktestMetrics {
            total_pnl,
            total_pnl_pct,
            win_rate,
            total_trades,
            winning_trades,
            losing_trades,
            avg_win,
            avg_loss,
            profit_factor,
            sharpe_ratio,
            trade_sharpe_ratio,
            max_drawdown,
            max_drawdown_pct,
            max_tick_drawdown,
            max_tick_drawdown_pct,
            avg_hold_duration_ms,
            trades_per_day,
            by_exit_reason,
        },
        equity_curve,
    )
}

/// build a daily P&L return series from trades.
/// each calendar day gets the sum of trade P&L that closed on that day,
/// divided by the running equity at start of day.
/// days with no trade closings get 0.0 return.
fn compute_daily_returns(trades: &[TradeRecord], initial_capital: f64) -> Vec<f64> {
    if trades.is_empty() {
        return vec![];
    }

    // find date range
    let first_date = trades.iter().map(|t| t.exit_time.date_naive()).min().unwrap();
    let last_date = trades.iter().map(|t| t.exit_time.date_naive()).max().unwrap();

    // build daily pnl map
    let mut daily_pnl: HashMap<chrono::NaiveDate, f64> = HashMap::new();
    for trade in trades {
        let date = trade.exit_time.date_naive();
        *daily_pnl.entry(date).or_insert(0.0) += trade.pnl;
    }

    // iterate over all calendar days in range, compute returns
    let mut returns = Vec::new();
    let mut equity = initial_capital;
    let mut date = first_date;
    while date <= last_date {
        let pnl = daily_pnl.get(&date).copied().unwrap_or(0.0);
        let daily_return = if equity > 0.0 { pnl / equity } else { 0.0 };
        returns.push(daily_return);
        equity += pnl;
        date = date.succ_opt().unwrap_or(date);
    }

    returns
}

/// serialize a BacktestResult to JSON string.
pub fn to_json(result: &BacktestResult) -> Result<String, String> {
    serde_json::to_string_pretty(result).map_err(|e| format!("json serialization error: {}", e))
}

/// write trade records to CSV format.
pub fn trades_to_csv<W: Write>(trades: &[TradeRecord], writer: W) -> Result<(), String> {
    let mut wtr = csv::Writer::from_writer(writer);

    // header
    wtr.write_record([
        "ticker",
        "direction",
        "entry_price",
        "exit_price",
        "size",
        "entry_time",
        "exit_time",
        "pnl",
        "pnl_pct",
        "hold_duration_ms",
        "exit_reason",
        "high_water_mark",
        "low_water_mark",
    ])
    .map_err(|e| format!("csv write error: {}", e))?;

    for t in trades {
        wtr.write_record([
            &t.ticker,
            &format!("{:?}", t.direction),
            &t.entry_price.to_string(),
            &t.exit_price.to_string(),
            &t.size.to_string(),
            &t.entry_time.to_rfc3339(),
            &t.exit_time.to_rfc3339(),
            &t.pnl.to_string(),
            &t.pnl_pct.to_string(),
            &t.hold_duration_ms.to_string(),
            &format!("{:?}", t.exit_reason),
            &t.high_water_mark.to_string(),
            &t.low_water_mark.to_string(),
        ])
        .map_err(|e| format!("csv write error: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("csv flush error: {}", e))?;
    Ok(())
}

/// produce a human-readable summary of backtest results.
pub fn summary(result: &BacktestResult) -> String {
    let m = &result.metrics;
    let mut s = String::new();

    writeln!(s, "=== backtest summary ===").unwrap();
    writeln!(s, "ticker:           {}", result.ticker).unwrap();
    writeln!(s, "config:           {}", result.config_id).unwrap();
    writeln!(s, "period:           {} to {}", result.start_time, result.end_time).unwrap();
    writeln!(s, "initial capital:  ${:.2}", result.initial_capital).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "--- performance ---").unwrap();
    writeln!(s, "total P&L:        ${:.2} ({:.2}%)", m.total_pnl, m.total_pnl_pct * 100.0).unwrap();
    writeln!(s, "total trades:     {}", m.total_trades).unwrap();
    writeln!(s, "win rate:         {:.1}%", m.win_rate * 100.0).unwrap();
    writeln!(s, "avg win:          ${:.2}", m.avg_win).unwrap();
    writeln!(s, "avg loss:         ${:.2}", m.avg_loss).unwrap();
    writeln!(s, "profit factor:    {:.2}", m.profit_factor).unwrap();
    writeln!(s, "sharpe (daily):   {:.3}", m.sharpe_ratio).unwrap();
    writeln!(s, "sharpe (trade):   {:.3}", m.trade_sharpe_ratio).unwrap();
    writeln!(s).unwrap();
    writeln!(s, "--- risk ---").unwrap();
    writeln!(s, "max drawdown:     ${:.2} ({:.2}%) [trade-level]", m.max_drawdown, m.max_drawdown_pct * 100.0).unwrap();
    if m.max_tick_drawdown > 0.0 {
        writeln!(s, "max tick dd:      ${:.2} ({:.2}%) [tick-level]", m.max_tick_drawdown, m.max_tick_drawdown_pct * 100.0).unwrap();
    }
    writeln!(s, "avg hold:         {:.0}s", m.avg_hold_duration_ms as f64 / 1000.0).unwrap();
    writeln!(s, "trades/day:       {:.1}", m.trades_per_day).unwrap();

    if !m.by_exit_reason.is_empty() {
        writeln!(s).unwrap();
        writeln!(s, "--- by exit reason ---").unwrap();
        let mut reasons: Vec<_> = m.by_exit_reason.iter().collect();
        reasons.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        for (reason, breakdown) in reasons {
            writeln!(
                s,
                "  {:?}: {} trades, ${:.2} total, {:.0}% win rate",
                reason, breakdown.count, breakdown.total_pnl, breakdown.win_rate * 100.0
            )
            .unwrap();
        }
    }

    s
}

/// comparison between two backtest results on the same data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigComparison {
    pub config_a_id: String,
    pub config_b_id: String,
    pub pnl_delta: f64,
    pub pnl_pct_delta: f64,
    pub win_rate_delta: f64,
    pub sharpe_delta: f64,
    pub max_drawdown_delta: f64,
    pub max_drawdown_pct_delta: f64,
    pub trades_delta: i64,
    pub profit_factor_delta: f64,
    pub metrics_a: BacktestMetrics,
    pub metrics_b: BacktestMetrics,
}

/// compare two backtest results to show the effect of config changes.
pub fn compare_configs(a: &BacktestResult, b: &BacktestResult) -> ConfigComparison {
    let ma = &a.metrics;
    let mb = &b.metrics;

    ConfigComparison {
        config_a_id: a.config_id.clone(),
        config_b_id: b.config_id.clone(),
        pnl_delta: mb.total_pnl - ma.total_pnl,
        pnl_pct_delta: mb.total_pnl_pct - ma.total_pnl_pct,
        win_rate_delta: mb.win_rate - ma.win_rate,
        sharpe_delta: mb.sharpe_ratio - ma.sharpe_ratio,
        max_drawdown_delta: mb.max_drawdown - ma.max_drawdown,
        max_drawdown_pct_delta: mb.max_drawdown_pct - ma.max_drawdown_pct,
        trades_delta: mb.total_trades as i64 - ma.total_trades as i64,
        profit_factor_delta: mb.profit_factor - ma.profit_factor,
        metrics_a: ma.clone(),
        metrics_b: mb.clone(),
    }
}

/// produce a human-readable comparison summary.
pub fn comparison_summary(cmp: &ConfigComparison) -> String {
    let mut s = String::new();

    writeln!(s, "=== config comparison ===").unwrap();
    writeln!(s, "config A: {}", cmp.config_a_id).unwrap();
    writeln!(s, "config B: {}", cmp.config_b_id).unwrap();
    writeln!(s).unwrap();

    let arrow = |delta: f64| if delta > 0.0 { "+" } else { "" };

    writeln!(s, "P&L:            {}{:.2}", arrow(cmp.pnl_delta), cmp.pnl_delta).unwrap();
    writeln!(s, "P&L %:          {}{:.2}%", arrow(cmp.pnl_pct_delta), cmp.pnl_pct_delta * 100.0).unwrap();
    writeln!(s, "win rate:       {}{:.1}%", arrow(cmp.win_rate_delta), cmp.win_rate_delta * 100.0).unwrap();
    writeln!(s, "sharpe:         {}{:.3}", arrow(cmp.sharpe_delta), cmp.sharpe_delta).unwrap();
    writeln!(s, "max drawdown:   {}{:.2}", arrow(cmp.max_drawdown_delta), cmp.max_drawdown_delta).unwrap();
    writeln!(s, "trades:         {}{}", arrow(cmp.trades_delta as f64), cmp.trades_delta).unwrap();
    writeln!(s, "profit factor:  {}{:.2}", arrow(cmp.profit_factor_delta), cmp.profit_factor_delta).unwrap();

    s
}
