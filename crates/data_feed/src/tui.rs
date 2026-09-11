use std::collections::{BTreeMap, HashMap};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::symbols::Marker;
use ratatui::widgets::canvas::{Canvas, Line, Context};
use ratatui::widgets::*;

use types::action::{ActionPhase, TradeDirection};
use types::market::{Candle, Timescale};
use types::scoring::AggregationMethod;
use types::StrategyConfig;

// ---------------------------------------------------------------------------
// cyberpunk theme
// ---------------------------------------------------------------------------
mod theme {
    use ratatui::prelude::*;
    use ratatui::widgets::{Block, BorderType, Borders};

    pub const NEON_GREEN: Color = Color::Rgb(0, 255, 128);
    pub const MATRIX_GREEN: Color = Color::Rgb(0, 230, 118);
    pub const DIM_GREEN: Color = Color::Rgb(0, 140, 70);
    pub const CYAN: Color = Color::Rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::Rgb(255, 0, 80);
    pub const AMBER: Color = Color::Rgb(255, 191, 0);
    pub const CANDLE_UP: Color = Color::Rgb(0, 255, 128);
    pub const CANDLE_DOWN: Color = Color::Rgb(255, 0, 80);
    pub const BG: Color = Color::Black;

    pub fn neon_block(title: &str) -> Block<'_> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(NEON_GREEN))
            .title(title)
            .title_style(Style::default().fg(NEON_GREEN).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(BG))
    }

    pub fn selected_block(title: &str) -> Block<'_> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(CYAN))
            .title(title)
            .title_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(BG))
    }

    pub fn pnl_color(value: f64) -> Color {
        if value >= 0.0 {
            NEON_GREEN
        } else {
            MAGENTA
        }
    }
}

// ---------------------------------------------------------------------------
// page enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiPage {
    Dashboard,
    ConfigViewer,
}

// ---------------------------------------------------------------------------
// data models
// ---------------------------------------------------------------------------

/// state for a single ticker displayed in the dashboard.
#[derive(Debug, Clone)]
pub struct TickerState {
    pub ticker: String,
    pub last_price: f64,
    pub composite_score: f64,
    pub one_minute_score: Option<f64>,
    pub five_minute_score: Option<f64>,
    pub one_hour_score: Option<f64>,
    /// position info: direction, entry price, unrealized P&L, hold duration
    pub position: Option<PositionDisplay>,
    /// sparkline of recent prices (last 60 ticks)
    pub price_history: Vec<f64>,
    /// last 60 OHLC candles for candlestick chart
    pub candle_history: Vec<Candle>,
}

#[derive(Debug, Clone)]
pub struct PositionDisplay {
    pub direction: TradeDirection,
    pub entry_price: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub hold_duration_ms: i64,
}

/// a completed trade for the trade log.
#[derive(Debug, Clone)]
pub struct TradeLogEntry {
    pub ticker: String,
    pub direction: String,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub exit_reason: String,
    pub time: DateTime<Utc>,
}

/// shared dashboard state updated by the main loop, read by the TUI renderer.
#[derive(Debug, Clone)]
pub struct DashboardState {
    pub tickers: HashMap<String, TickerState>,
    pub config_version: i64,
    pub daily_pnl: f64,
    pub total_ticks: u64,
    pub start_time: Instant,
    pub trade_log: Vec<TradeLogEntry>,
    /// index into `ticker_order` for tab selection
    pub selected_ticker_idx: usize,
    /// sorted ticker names for stable navigation
    pub ticker_order: Vec<String>,
    /// which page is currently displayed
    pub active_page: TuiPage,
    /// full strategy config for the config viewer page
    pub strategy_config: Option<StrategyConfig>,
    /// vertical scroll offset for the config viewer
    pub config_scroll_offset: u16,
    /// total scrollable lines in the config viewer (computed on config set)
    pub config_total_lines: u16,
}

impl DashboardState {
    pub fn new(config_version: i64) -> Self {
        Self {
            tickers: HashMap::new(),
            config_version,
            daily_pnl: 0.0,
            total_ticks: 0,
            start_time: Instant::now(),
            trade_log: Vec::new(),
            selected_ticker_idx: 0,
            ticker_order: Vec::new(),
            active_page: TuiPage::Dashboard,
            strategy_config: None,
            config_scroll_offset: 0,
            config_total_lines: 0,
        }
    }

    /// toggle between dashboard and config viewer pages.
    pub fn toggle_page(&mut self) {
        self.active_page = match self.active_page {
            TuiPage::Dashboard => TuiPage::ConfigViewer,
            TuiPage::ConfigViewer => TuiPage::Dashboard,
        };
        self.config_scroll_offset = 0;
    }

    /// scroll config viewer up by one line.
    pub fn scroll_up(&mut self) {
        self.config_scroll_offset = self.config_scroll_offset.saturating_sub(1);
    }

    /// scroll config viewer down by one line, clamped to max.
    pub fn scroll_down(&mut self, max: u16) {
        if self.config_scroll_offset < max {
            self.config_scroll_offset += 1;
        }
    }

    /// scroll config viewer up by half a viewport.
    pub fn scroll_page_up(&mut self, viewport_h: u16) {
        let jump = viewport_h / 2;
        self.config_scroll_offset = self.config_scroll_offset.saturating_sub(jump);
    }

    /// scroll config viewer down by half a viewport, clamped to max.
    pub fn scroll_page_down(&mut self, viewport_h: u16, max: u16) {
        let jump = viewport_h / 2;
        self.config_scroll_offset = (self.config_scroll_offset + jump).min(max);
    }

    /// store the strategy config and compute total scrollable lines.
    pub fn set_strategy_config(&mut self, config: StrategyConfig) {
        let total = compute_config_lines(&config);
        self.strategy_config = Some(config);
        self.config_total_lines = total;
        // clamp scroll if config shrank
        if self.config_scroll_offset > self.config_total_lines {
            self.config_scroll_offset = self.config_total_lines;
        }
    }

    /// add a trade to the log (keep last 10).
    pub fn add_trade(&mut self, entry: TradeLogEntry) {
        self.trade_log.push(entry);
        if self.trade_log.len() > 10 {
            self.trade_log.remove(0);
        }
    }

    /// return the currently selected ticker name, if any.
    pub fn selected_ticker(&self) -> Option<&str> {
        self.ticker_order
            .get(self.selected_ticker_idx)
            .map(|s| s.as_str())
    }

    /// advance to the next ticker (wraps around).
    pub fn next_ticker(&mut self) {
        if !self.ticker_order.is_empty() {
            self.selected_ticker_idx = (self.selected_ticker_idx + 1) % self.ticker_order.len();
        }
    }

    /// go to the previous ticker (wraps around).
    pub fn prev_ticker(&mut self) {
        if !self.ticker_order.is_empty() {
            if self.selected_ticker_idx == 0 {
                self.selected_ticker_idx = self.ticker_order.len() - 1;
            } else {
                self.selected_ticker_idx -= 1;
            }
        }
    }

    /// ensure a ticker exists in the sorted order list.
    pub fn ensure_ticker_order(&mut self, ticker: &str) {
        if !self.ticker_order.contains(&ticker.to_string()) {
            self.ticker_order.push(ticker.to_string());
            self.ticker_order.sort();
        }
    }
}

// ---------------------------------------------------------------------------
// TUI main loop
// ---------------------------------------------------------------------------

/// run the TUI rendering loop. reads from the shared dashboard state.
/// sets `shutdown` to true when the user presses 'q' or esc.
pub fn run_tui(state: Arc<RwLock<DashboardState>>, shutdown: Arc<AtomicBool>) -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let tick_rate = Duration::from_millis(250); // 4 Hz

    loop {
        // check if main loop triggered shutdown (e.g. ctrl-c)
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        terminal.draw(|frame| {
            let state = state.read().unwrap();
            match state.active_page {
                TuiPage::Dashboard => render_dashboard(frame, &state),
                TuiPage::ConfigViewer => render_config_page(frame, &state),
            }
        })?;

        // poll for input events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let page = state.read().unwrap().active_page;
                    match key.code {
                        // global keys
                        KeyCode::Char('q') | KeyCode::Esc => {
                            shutdown.store(true, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('c') => {
                            let mut s = state.write().unwrap();
                            s.toggle_page();
                        }
                        // dashboard-only keys
                        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l')
                            if page == TuiPage::Dashboard =>
                        {
                            let mut s = state.write().unwrap();
                            s.next_ticker();
                        }
                        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h')
                            if page == TuiPage::Dashboard =>
                        {
                            let mut s = state.write().unwrap();
                            s.prev_ticker();
                        }
                        // config viewer scroll keys
                        KeyCode::Char('j') | KeyCode::Down
                            if page == TuiPage::ConfigViewer =>
                        {
                            let mut s = state.write().unwrap();
                            let max = s.config_total_lines;
                            s.scroll_down(max);
                        }
                        KeyCode::Char('k') | KeyCode::Up
                            if page == TuiPage::ConfigViewer =>
                        {
                            let mut s = state.write().unwrap();
                            s.scroll_up();
                        }
                        KeyCode::PageDown if page == TuiPage::ConfigViewer => {
                            let mut s = state.write().unwrap();
                            let max = s.config_total_lines;
                            // use a reasonable viewport estimate
                            s.scroll_page_down(terminal.size()?.height.saturating_sub(9), max);
                        }
                        KeyCode::PageUp if page == TuiPage::ConfigViewer => {
                            let mut s = state.write().unwrap();
                            s.scroll_page_up(terminal.size()?.height.saturating_sub(9));
                        }
                        KeyCode::Home if page == TuiPage::ConfigViewer => {
                            let mut s = state.write().unwrap();
                            s.config_scroll_offset = 0;
                        }
                        KeyCode::End if page == TuiPage::ConfigViewer => {
                            let mut s = state.write().unwrap();
                            s.config_scroll_offset = s.config_total_lines;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// rendering
// ---------------------------------------------------------------------------

fn render_dashboard(frame: &mut Frame, state: &DashboardState) {
    let area = frame.area();

    // fill background
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BG)),
        area,
    );

    // main layout: header, tabs, body, footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // ticker tabs
            Constraint::Min(10),  // body
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(frame, layout[0], state);
    render_ticker_tabs(frame, layout[1], state);
    render_body(frame, layout[2], state);
    render_footer(frame, layout[3], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let uptime = state.start_time.elapsed();
    let uptime_str = format!(
        "{:02}:{:02}:{:02}",
        uptime.as_secs() / 3600,
        (uptime.as_secs() % 3600) / 60,
        uptime.as_secs() % 60
    );

    // show selected ticker price in header
    let price_str = state
        .selected_ticker()
        .and_then(|t| state.tickers.get(t))
        .map(|ts| format!("{} ${:.2}", ts.ticker, ts.last_price))
        .unwrap_or_else(|| "---".to_string());

    let header_text = format!(
        " {} \u{2500}\u{2500} config v{} \u{2500}\u{2500} uptime {} ",
        price_str, state.config_version, uptime_str,
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(theme::MATRIX_GREEN))
        .block(theme::neon_block(" GALACTIC TRADING FIRM "));
    frame.render_widget(header, area);
}

fn render_ticker_tabs(frame: &mut Frame, area: Rect, state: &DashboardState) {
    if state.ticker_order.is_empty() {
        let empty = Paragraph::new("  waiting for data...")
            .style(Style::default().fg(theme::DIM_GREEN))
            .block(theme::neon_block(" tickers "));
        frame.render_widget(empty, area);
        return;
    }

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::raw(" "));

    for (i, ticker) in state.ticker_order.iter().enumerate() {
        let price_str = state
            .tickers
            .get(ticker)
            .map(|ts| format!("{} ${:.2}", ticker, ts.last_price))
            .unwrap_or_else(|| ticker.clone());

        if i == state.selected_ticker_idx {
            spans.push(Span::styled(
                format!("[{price_str}]"),
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {price_str} "),
                Style::default().fg(theme::DIM_GREEN),
            ));
        }

        if i < state.ticker_order.len() - 1 {
            spans.push(Span::styled(
                " \u{2502} ",
                Style::default().fg(theme::DIM_GREEN),
            ));
        }
    }

    let tabs_line = ratatui::text::Line::from(spans);
    let tabs = Paragraph::new(tabs_line).block(theme::neon_block(" tickers "));
    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame, area: Rect, state: &DashboardState) {
    // split body: left 60% (chart) / right 40% (position + scores + trade log)
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_chart(frame, body_layout[0], state);
    render_right_panels(frame, body_layout[1], state);
}

fn render_chart(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let ticker_state = state
        .selected_ticker()
        .and_then(|t| state.tickers.get(t));

    let candles = ticker_state.map(|ts| &ts.candle_history);

    let block = theme::selected_block(" chart ");

    // need at least 3 candles to draw
    let has_enough = candles.map(|c| c.len() >= 3).unwrap_or(false);

    if !has_enough {
        let waiting = Paragraph::new("\n\n  waiting for data...")
            .style(Style::default().fg(theme::DIM_GREEN))
            .block(block);
        frame.render_widget(waiting, area);
        return;
    }

    let candles = candles.unwrap();

    // determine how many candles fit in the available width
    // each candle occupies 3.0 x-units; inner area width gives us pixel cols
    let inner = block.inner(area);
    let max_visible = ((inner.width as f64) / 3.0).floor().max(1.0) as usize;
    let visible_candles: &[Candle] = if candles.len() > max_visible {
        &candles[candles.len() - max_visible..]
    } else {
        candles
    };

    // compute price range
    let mut min_price = f64::MAX;
    let mut max_price = f64::MIN;
    for c in visible_candles {
        if c.low < min_price {
            min_price = c.low;
        }
        if c.high > max_price {
            max_price = c.high;
        }
    }
    let price_range = max_price - min_price;
    let padding = price_range * 0.01;
    let y_min = min_price - padding;
    let y_max = max_price + padding;

    let x_max = (visible_candles.len() as f64) * 3.0;

    // price labels for the right side
    let label_count = 5usize;
    let price_labels: Vec<(f64, String)> = (0..label_count)
        .map(|i| {
            let frac = i as f64 / (label_count - 1) as f64;
            let price = y_min + frac * (y_max - y_min);
            (price, format!("{price:.2}"))
        })
        .collect();

    let chart = Canvas::default()
        .block(block)
        .x_bounds([0.0, x_max])
        .y_bounds([y_min, y_max])
        .marker(Marker::HalfBlock)
        .paint(move |ctx| {
            draw_candles(ctx, visible_candles, y_min, y_max);
            // price axis labels on the right
            for (price, label) in &price_labels {
                ctx.print(
                    x_max - 1.0,
                    *price,
                    Span::styled(label.clone(), Style::default().fg(theme::DIM_GREEN)),
                );
            }
        });

    frame.render_widget(chart, area);
}

fn draw_candles(ctx: &mut Context, candles: &[Candle], _y_min: f64, _y_max: f64) {
    for (i, candle) in candles.iter().enumerate() {
        let x_center = i as f64 * 3.0 + 1.5;
        let is_up = candle.close >= candle.open;
        let color = if is_up {
            theme::CANDLE_UP
        } else {
            theme::CANDLE_DOWN
        };

        // wick: vertical line from low to high
        ctx.draw(&Line {
            x1: x_center,
            y1: candle.low,
            x2: x_center,
            y2: candle.high,
            color,
        });

        // body: multiple adjacent vertical lines to fill the body
        let body_top = candle.open.max(candle.close);
        let body_bot = candle.open.min(candle.close);
        // draw body from x_center-0.8 to x_center+0.8 in 0.4 steps
        let body_steps = [-0.8, -0.4, 0.0, 0.4, 0.8];
        for &dx in &body_steps {
            ctx.draw(&Line {
                x1: x_center + dx,
                y1: body_bot,
                x2: x_center + dx,
                y2: body_top,
                color,
            });
        }
    }
}

fn render_right_panels(frame: &mut Frame, area: Rect, state: &DashboardState) {
    // stack: position, scores, trade log
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // position
            Constraint::Length(6),  // scores
            Constraint::Min(5),    // trade log
        ])
        .split(area);

    render_position(frame, right_layout[0], state);
    render_scores(frame, right_layout[1], state);
    render_trade_log(frame, right_layout[2], state);
}

fn render_position(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let ticker_state = state
        .selected_ticker()
        .and_then(|t| state.tickers.get(t));

    let text = match ticker_state.and_then(|ts| ts.position.as_ref()) {
        Some(pos) => {
            let dir_str = match pos.direction {
                TradeDirection::Long => "LONG",
                TradeDirection::Short => "SHORT",
            };
            let dir_color = match pos.direction {
                TradeDirection::Long => theme::NEON_GREEN,
                TradeDirection::Short => theme::MAGENTA,
            };
            let pnl_sign = if pos.unrealized_pnl >= 0.0 { "+" } else { "" };
            let hold_secs = pos.hold_duration_ms / 1000;

            vec![
                ratatui::text::Line::from(vec![
                    Span::styled(
                        format!(" {dir_str}"),
                        Style::default().fg(dir_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" @ {:.2}", pos.entry_price),
                        Style::default().fg(theme::MATRIX_GREEN),
                    ),
                ]),
                ratatui::text::Line::from(vec![
                    Span::raw(" P&L: "),
                    Span::styled(
                        format!(
                            "{pnl_sign}{:.2} ({pnl_sign}{:.2}%)",
                            pos.unrealized_pnl,
                            pos.unrealized_pnl_pct * 100.0
                        ),
                        Style::default()
                            .fg(theme::pnl_color(pos.unrealized_pnl))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  hold: {hold_secs}s"),
                        Style::default().fg(theme::DIM_GREEN),
                    ),
                ]),
            ]
        }
        None => {
            vec![ratatui::text::Line::from(Span::styled(
                " flat (no position)",
                Style::default().fg(theme::DIM_GREEN),
            ))]
        }
    };

    let paragraph = Paragraph::new(text).block(theme::neon_block(" position "));
    frame.render_widget(paragraph, area);
}

fn render_scores(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let ticker_state = state
        .selected_ticker()
        .and_then(|t| state.tickers.get(t));

    let format_score = |s: Option<f64>| -> String {
        s.map(|v| format!("{v:.3}")).unwrap_or_else(|| "n/a".to_string())
    };

    let lines = if let Some(ts) = ticker_state {
        vec![
            ratatui::text::Line::from(vec![
                Span::raw(" composite: "),
                Span::styled(
                    format!("{:.3}", ts.composite_score),
                    Style::default()
                        .fg(theme::CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            ratatui::text::Line::from(vec![
                Span::styled(" 1m: ", Style::default().fg(theme::DIM_GREEN)),
                Span::styled(
                    format_score(ts.one_minute_score),
                    Style::default().fg(theme::MATRIX_GREEN),
                ),
                Span::styled("  5m: ", Style::default().fg(theme::DIM_GREEN)),
                Span::styled(
                    format_score(ts.five_minute_score),
                    Style::default().fg(theme::MATRIX_GREEN),
                ),
                Span::styled("  1h: ", Style::default().fg(theme::DIM_GREEN)),
                Span::styled(
                    format_score(ts.one_hour_score),
                    Style::default().fg(theme::MATRIX_GREEN),
                ),
            ]),
        ]
    } else {
        vec![ratatui::text::Line::from(Span::styled(
            " waiting for data...",
            Style::default().fg(theme::DIM_GREEN),
        ))]
    };

    let paragraph = Paragraph::new(lines).block(theme::neon_block(" scores "));
    frame.render_widget(paragraph, area);
}

fn render_trade_log(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let header = Row::new(vec!["ticker", "dir", "P&L", "P&L%", "reason"])
        .style(
            Style::default()
                .fg(theme::NEON_GREEN)
                .add_modifier(Modifier::BOLD),
        );

    let rows: Vec<Row> = state
        .trade_log
        .iter()
        .rev()
        .map(|t| {
            let pnl_style = Style::default().fg(theme::pnl_color(t.pnl));
            Row::new(vec![
                Cell::from(t.ticker.clone()).style(Style::default().fg(theme::MATRIX_GREEN)),
                Cell::from(t.direction.clone()).style(Style::default().fg(theme::MATRIX_GREEN)),
                Cell::from(format!("{:.2}", t.pnl)).style(pnl_style),
                Cell::from(format!("{:.2}%", t.pnl_pct * 100.0)).style(pnl_style),
                Cell::from(t.exit_reason.clone()).style(Style::default().fg(theme::DIM_GREEN)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Fill(1),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(theme::neon_block(" trade log "));
    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let pnl_sign = if state.daily_pnl >= 0.0 { "+" } else { "" };
    let pnl_color = theme::pnl_color(state.daily_pnl);

    let footer_line = ratatui::text::Line::from(vec![
        Span::styled(" daily P&L: ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled(
            format!("{pnl_sign}{:.2}", state.daily_pnl),
            Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" \u{2500}\u{2500} ticks: {} \u{2500}\u{2500} ", state.total_ticks),
            Style::default().fg(theme::DIM_GREEN),
        ),
        Span::styled("[tab]", Style::default().fg(theme::CYAN)),
        Span::styled(" ticker  ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled("[c]", Style::default().fg(theme::CYAN)),
        Span::styled(" config  ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled("[q]", Style::default().fg(theme::CYAN)),
        Span::styled(" quit", Style::default().fg(theme::DIM_GREEN)),
    ]);

    let footer = Paragraph::new(footer_line).block(theme::neon_block(""));
    frame.render_widget(footer, area);
}

// ---------------------------------------------------------------------------
// config page
// ---------------------------------------------------------------------------

/// compute the total number of lines the config viewer would render.
fn compute_config_lines(config: &StrategyConfig) -> u16 {
    let lines = build_config_lines(config);
    lines.len() as u16
}

fn render_config_page(frame: &mut Frame, state: &DashboardState) {
    let area = frame.area();

    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BG)),
        area,
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(10),  // config body
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(frame, layout[0], state);
    render_config_body(frame, layout[1], state);
    render_config_footer(frame, layout[2], state);
}

fn render_config_body(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let block = theme::neon_block(" strategy config ");
    let inner = block.inner(area);

    let lines = match &state.strategy_config {
        Some(config) => build_config_lines(config),
        None => vec![ratatui::text::Line::from(Span::styled(
            " no config loaded",
            Style::default().fg(theme::DIM_GREEN),
        ))],
    };

    let content_height = lines.len() as u16;
    let viewport_h = inner.height;
    let max_scroll = content_height.saturating_sub(viewport_h);
    let scroll = state.config_scroll_offset.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);

    // scrollbar
    if content_height > viewport_h {
        let mut scrollbar_state = ScrollbarState::new(content_height as usize)
            .position(scroll as usize)
            .viewport_content_length(viewport_h as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(theme::DIM_GREEN)),
            inner,
            &mut scrollbar_state,
        );
    }
}

fn build_config_lines(config: &StrategyConfig) -> Vec<ratatui::text::Line<'static>> {
    let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();

    render_indicators_section(config, &mut lines);
    lines.push(ratatui::text::Line::from(""));
    render_actions_section(config, &mut lines);
    lines.push(ratatui::text::Line::from(""));
    render_scoring_section(config, &mut lines);
    lines.push(ratatui::text::Line::from(""));
    render_session_section(config, &mut lines);

    lines
}

fn timescale_label(ts: &Timescale) -> &'static str {
    match ts {
        Timescale::OneMinute => "1min",
        Timescale::FiveMinute => "5min",
        Timescale::OneHour => "1hour",
        Timescale::OneDay => "1day",
        Timescale::OneMonth => "1month",
    }
}

fn render_indicators_section(
    config: &StrategyConfig,
    lines: &mut Vec<ratatui::text::Line<'static>>,
) {
    lines.push(ratatui::text::Line::from(Span::styled(
        " INDICATORS",
        Style::default()
            .fg(theme::CYAN)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(ratatui::text::Line::from(""));

    // group by timescale using BTreeMap for stable ordering
    let mut by_timescale: BTreeMap<String, Vec<&types::IndicatorConfig>> = BTreeMap::new();
    for ic in &config.indicators {
        let key = timescale_label(&ic.timescale).to_string();
        by_timescale.entry(key).or_default().push(ic);
    }

    for (ts_label, indicators) in &by_timescale {
        lines.push(ratatui::text::Line::from(Span::styled(
            format!("  [{ts_label}]"),
            Style::default()
                .fg(theme::AMBER)
                .add_modifier(Modifier::BOLD),
        )));

        for ic in indicators {
            let status = if ic.enabled {
                Span::styled(
                    "  [ON] ",
                    Style::default()
                        .fg(theme::NEON_GREEN)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " [OFF] ",
                    Style::default()
                        .fg(theme::MAGENTA)
                        .add_modifier(Modifier::BOLD),
                )
            };

            lines.push(ratatui::text::Line::from(vec![
                status,
                Span::styled(
                    ic.instance_id.clone(),
                    Style::default().fg(theme::CYAN),
                ),
                Span::styled(
                    format!("  ({})", ic.indicator_type),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  w={:.2}", ic.weight),
                    Style::default().fg(theme::NEON_GREEN),
                ),
            ]));

            // sorted params
            let mut sorted_params: Vec<_> = ic.params.iter().collect();
            sorted_params.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in sorted_params {
                lines.push(ratatui::text::Line::from(vec![
                    Span::raw("         "),
                    Span::styled(
                        format!("{k}: "),
                        Style::default().fg(theme::DIM_GREEN),
                    ),
                    Span::styled(
                        format_json_value(v),
                        Style::default().fg(theme::MATRIX_GREEN),
                    ),
                ]));
            }

            // modification tracking
            if let Some(ref by) = ic.last_modified_by {
                let at_str = ic
                    .last_modified_at
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                let reason = ic
                    .modification_reason
                    .as_deref()
                    .unwrap_or("");
                lines.push(ratatui::text::Line::from(vec![
                    Span::raw("         "),
                    Span::styled(
                        format!("modified by {by} {at_str}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                if !reason.is_empty() {
                    lines.push(ratatui::text::Line::from(vec![
                        Span::raw("         "),
                        Span::styled(
                            format!("reason: {reason}"),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }
        }
        lines.push(ratatui::text::Line::from(""));
    }
}

fn render_actions_section(
    config: &StrategyConfig,
    lines: &mut Vec<ratatui::text::Line<'static>>,
) {
    lines.push(ratatui::text::Line::from(Span::styled(
        " ACTIONS",
        Style::default()
            .fg(theme::CYAN)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(ratatui::text::Line::from(""));

    let phase_order = [
        ActionPhase::Entry,
        ActionPhase::Monitor,
        ActionPhase::Exit,
        ActionPhase::Sizing,
    ];

    for phase in &phase_order {
        let phase_actions: Vec<&types::ActionConfig> = config
            .actions
            .iter()
            .filter(|a| a.phase == *phase)
            .collect();

        if phase_actions.is_empty() {
            continue;
        }

        let phase_label = match phase {
            ActionPhase::Entry => "Entry",
            ActionPhase::Monitor => "Monitor",
            ActionPhase::Exit => "Exit",
            ActionPhase::Sizing => "Sizing",
        };

        lines.push(ratatui::text::Line::from(Span::styled(
            format!("  [{phase_label}]"),
            Style::default()
                .fg(theme::AMBER)
                .add_modifier(Modifier::BOLD),
        )));

        for ac in &phase_actions {
            let status = if ac.enabled {
                Span::styled(
                    "  [ON] ",
                    Style::default()
                        .fg(theme::NEON_GREEN)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    " [OFF] ",
                    Style::default()
                        .fg(theme::MAGENTA)
                        .add_modifier(Modifier::BOLD),
                )
            };

            lines.push(ratatui::text::Line::from(vec![
                status,
                Span::styled(
                    ac.instance_id.clone(),
                    Style::default().fg(theme::CYAN),
                ),
                Span::styled(
                    format!("  ({})", ac.action_type),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  pri={}", ac.priority),
                    Style::default().fg(theme::NEON_GREEN),
                ),
            ]));

            let mut sorted_params: Vec<_> = ac.params.iter().collect();
            sorted_params.sort_by_key(|(k, _)| k.as_str());
            for (k, v) in sorted_params {
                lines.push(ratatui::text::Line::from(vec![
                    Span::raw("         "),
                    Span::styled(
                        format!("{k}: "),
                        Style::default().fg(theme::DIM_GREEN),
                    ),
                    Span::styled(
                        format_json_value(v),
                        Style::default().fg(theme::MATRIX_GREEN),
                    ),
                ]));
            }

            if let Some(ref by) = ac.last_modified_by {
                let at_str = ac
                    .last_modified_at
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                let reason = ac.modification_reason.as_deref().unwrap_or("");
                lines.push(ratatui::text::Line::from(vec![
                    Span::raw("         "),
                    Span::styled(
                        format!("modified by {by} {at_str}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                if !reason.is_empty() {
                    lines.push(ratatui::text::Line::from(vec![
                        Span::raw("         "),
                        Span::styled(
                            format!("reason: {reason}"),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }
        }
        lines.push(ratatui::text::Line::from(""));
    }
}

fn render_scoring_section(
    config: &StrategyConfig,
    lines: &mut Vec<ratatui::text::Line<'static>>,
) {
    lines.push(ratatui::text::Line::from(Span::styled(
        " SCORING",
        Style::default()
            .fg(theme::CYAN)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(ratatui::text::Line::from(""));

    let agg_str = match config.scoring.aggregation {
        AggregationMethod::WeightedSum => "WeightedSum",
        AggregationMethod::WeightedSumWithGates => "WeightedSumWithGates",
        AggregationMethod::MinScore => "MinScore",
        AggregationMethod::DynamicFusion => "DynamicFusion",
    };
    lines.push(ratatui::text::Line::from(vec![
        Span::styled("  aggregation: ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled(
            agg_str.to_string(),
            Style::default().fg(theme::MATRIX_GREEN),
        ),
    ]));

    lines.push(ratatui::text::Line::from(vec![
        Span::styled("  entry_threshold: ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled(
            format!("{:.3}", config.scoring.entry_threshold),
            Style::default()
                .fg(theme::NEON_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(ratatui::text::Line::from(vec![
        Span::styled("  exit_threshold: ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled(
            format!("{:.3}", config.scoring.exit_threshold),
            Style::default()
                .fg(theme::MAGENTA)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(Span::styled(
        "  timescale weights:",
        Style::default().fg(theme::DIM_GREEN),
    )));

    // sort timescale weights for stable display
    let mut tw: Vec<_> = config.scoring.timescale_weights.iter().collect();
    tw.sort_by_key(|(ts, _)| timescale_label(ts));
    for (ts, w) in tw {
        lines.push(ratatui::text::Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{}: ", timescale_label(ts)),
                Style::default().fg(theme::DIM_GREEN),
            ),
            Span::styled(
                format!("{w:.3}"),
                Style::default().fg(theme::MATRIX_GREEN),
            ),
        ]));
    }

    if !config.scoring.hard_gate_timescales.is_empty() {
        lines.push(ratatui::text::Line::from(""));
        let gates: Vec<&str> = config
            .scoring
            .hard_gate_timescales
            .iter()
            .map(|ts| timescale_label(ts))
            .collect();
        lines.push(ratatui::text::Line::from(vec![
            Span::styled("  hard gates: ", Style::default().fg(theme::DIM_GREEN)),
            Span::styled(
                gates.join(", "),
                Style::default()
                    .fg(theme::MAGENTA)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }
}

fn render_session_section(
    config: &StrategyConfig,
    lines: &mut Vec<ratatui::text::Line<'static>>,
) {
    lines.push(ratatui::text::Line::from(Span::styled(
        " SESSION",
        Style::default()
            .fg(theme::CYAN)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));
    lines.push(ratatui::text::Line::from(""));

    let s = &config.session;
    let session_rows: Vec<(&str, String)> = vec![
        ("no_new_entries_after", s.no_new_entries_after.clone()),
        ("force_exit_by", s.force_exit_by.clone()),
        ("avoid_first_minutes", s.avoid_first_minutes.to_string()),
        (
            "max_concurrent_positions",
            s.max_concurrent_positions.to_string(),
        ),
        (
            "max_capital_deployed_pct",
            format!("{:.1}%", s.max_capital_deployed_pct * 100.0),
        ),
    ];

    for (label, value) in session_rows {
        lines.push(ratatui::text::Line::from(vec![
            Span::styled(
                format!("  {label}: "),
                Style::default().fg(theme::DIM_GREEN),
            ),
            Span::styled(value, Style::default().fg(theme::MATRIX_GREEN)),
        ]));
    }
}

fn render_config_footer(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let version_str = match &state.strategy_config {
        Some(config) => format!("v{}", config.config_id),
        None => "---".to_string(),
    };

    let footer_line = ratatui::text::Line::from(vec![
        Span::styled(
            format!(" config {version_str} "),
            Style::default().fg(theme::MATRIX_GREEN),
        ),
        Span::styled("\u{2500}\u{2500} ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled("[j/k]", Style::default().fg(theme::CYAN)),
        Span::styled(" scroll  ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled("[PgUp/PgDn]", Style::default().fg(theme::CYAN)),
        Span::styled(" page  ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled("[c]", Style::default().fg(theme::CYAN)),
        Span::styled(" dashboard  ", Style::default().fg(theme::DIM_GREEN)),
        Span::styled("[q]", Style::default().fg(theme::CYAN)),
        Span::styled(" quit", Style::default().fg(theme::DIM_GREEN)),
    ]);

    let footer = Paragraph::new(footer_line).block(theme::neon_block(""));
    frame.render_widget(footer, area);
}

fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: Utc::now(),
            open,
            high,
            low,
            close,
            volume: 10000.0,
        }
    }

    #[test]
    fn dashboard_state_construction() {
        let state = DashboardState::new(42);
        assert_eq!(state.config_version, 42);
        assert!(state.tickers.is_empty());
        assert!((state.daily_pnl - 0.0).abs() < f64::EPSILON);
        assert_eq!(state.total_ticks, 0);
        assert_eq!(state.selected_ticker_idx, 0);
        assert!(state.ticker_order.is_empty());
    }

    #[test]
    fn add_trade_keeps_last_10() {
        let mut state = DashboardState::new(1);
        for i in 0..15 {
            state.add_trade(TradeLogEntry {
                ticker: "SPY".to_string(),
                direction: "Long".to_string(),
                pnl: i as f64 * 10.0,
                pnl_pct: 0.01,
                exit_reason: "TrailingStop".to_string(),
                time: Utc::now(),
            });
        }
        assert_eq!(state.trade_log.len(), 10);
        // first entry should be trade #5 (indices 5-14)
        assert!((state.trade_log[0].pnl - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ticker_state_construction() {
        let ts = TickerState {
            ticker: "SPY".to_string(),
            last_price: 450.0,
            composite_score: 0.65,
            one_minute_score: Some(0.7),
            five_minute_score: Some(0.6),
            one_hour_score: None,
            position: None,
            price_history: vec![449.0, 450.0, 451.0],
            candle_history: Vec::new(),
        };
        assert_eq!(ts.ticker, "SPY");
        assert!(ts.position.is_none());
        assert_eq!(ts.price_history.len(), 3);
    }

    #[test]
    fn render_does_not_panic() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = DashboardState::new(1);

        terminal
            .draw(|frame| {
                render_dashboard(frame, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_with_data_does_not_panic() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = DashboardState::new(5);
        state.total_ticks = 100;
        state.daily_pnl = 42.50;
        state.ensure_ticker_order("SPY");
        state.tickers.insert(
            "SPY".to_string(),
            TickerState {
                ticker: "SPY".to_string(),
                last_price: 450.25,
                composite_score: 0.72,
                one_minute_score: Some(0.8),
                five_minute_score: Some(0.65),
                one_hour_score: Some(0.3),
                position: Some(PositionDisplay {
                    direction: TradeDirection::Long,
                    entry_price: 449.50,
                    unrealized_pnl: 75.0,
                    unrealized_pnl_pct: 0.0017,
                    hold_duration_ms: 120_000,
                }),
                price_history: (0..60).map(|i| 448.0 + i as f64 * 0.05).collect(),
                candle_history: Vec::new(),
            },
        );
        state.add_trade(TradeLogEntry {
            ticker: "SPY".to_string(),
            direction: "Long".to_string(),
            pnl: 25.50,
            pnl_pct: 0.005,
            exit_reason: "TrailingStop".to_string(),
            time: Utc::now(),
        });

        terminal
            .draw(|frame| {
                render_dashboard(frame, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_with_candles_does_not_panic() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let candles: Vec<Candle> = (0..60)
            .map(|i| {
                let base = 450.0 + i as f64 * 0.1;
                make_candle(base, base + 0.5, base - 0.3, base + 0.2)
            })
            .collect();

        let mut state = DashboardState::new(5);
        state.ensure_ticker_order("SPY");
        state.tickers.insert(
            "SPY".to_string(),
            TickerState {
                ticker: "SPY".to_string(),
                last_price: 456.0,
                composite_score: 0.72,
                one_minute_score: Some(0.8),
                five_minute_score: Some(0.65),
                one_hour_score: Some(0.3),
                position: None,
                price_history: Vec::new(),
                candle_history: candles,
            },
        );

        terminal
            .draw(|frame| {
                render_dashboard(frame, &state);
            })
            .unwrap();
    }

    #[test]
    fn next_ticker_cycles() {
        let mut state = DashboardState::new(1);
        state.ticker_order = vec!["AAPL".to_string(), "QQQ".to_string(), "SPY".to_string()];
        assert_eq!(state.selected_ticker_idx, 0);
        assert_eq!(state.selected_ticker(), Some("AAPL"));

        state.next_ticker();
        assert_eq!(state.selected_ticker(), Some("QQQ"));

        state.next_ticker();
        assert_eq!(state.selected_ticker(), Some("SPY"));

        state.next_ticker();
        assert_eq!(state.selected_ticker(), Some("AAPL")); // wraps around
    }

    #[test]
    fn prev_ticker_cycles() {
        let mut state = DashboardState::new(1);
        state.ticker_order = vec!["AAPL".to_string(), "QQQ".to_string(), "SPY".to_string()];
        assert_eq!(state.selected_ticker(), Some("AAPL"));

        state.prev_ticker();
        assert_eq!(state.selected_ticker(), Some("SPY")); // wraps to end

        state.prev_ticker();
        assert_eq!(state.selected_ticker(), Some("QQQ"));
    }

    #[test]
    fn ensure_ticker_order_sorted() {
        let mut state = DashboardState::new(1);
        state.ensure_ticker_order("SPY");
        state.ensure_ticker_order("AAPL");
        state.ensure_ticker_order("QQQ");
        // should not duplicate
        state.ensure_ticker_order("SPY");

        assert_eq!(
            state.ticker_order,
            vec!["AAPL".to_string(), "QQQ".to_string(), "SPY".to_string()]
        );
    }

    // -----------------------------------------------------------------------
    // config viewer tests
    // -----------------------------------------------------------------------

    fn make_test_strategy_config() -> StrategyConfig {
        use types::action::ActionConfig;
        use types::config::SessionConfig;
        use types::indicator::IndicatorConfig;
        use types::scoring::ScoringConfig;

        StrategyConfig {
            schema_version: "1.0".to_string(),
            config_id: 42,
            created_at: Utc::now(),
            created_by: "test".to_string(),
            parent_config_id: None,
            tickers: vec!["SPY".to_string()],
            indicators: vec![IndicatorConfig {
                indicator_type: "rsi".to_string(),
                instance_id: "rsi_14_5m".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight: 1.0,
                params: [
                    ("period".to_string(), serde_json::json!(14)),
                    ("overbought".to_string(), serde_json::json!(70)),
                ]
                .into_iter()
                .collect(),
                last_modified_by: Some("pm_agent".to_string()),
                last_modified_at: Some(Utc::now()),
                modification_reason: Some("tune RSI period".to_string()),
            }],
            actions: vec![ActionConfig {
                action_type: "atr_trailing_stop".to_string(),
                instance_id: "atr_ts_1".to_string(),
                phase: ActionPhase::Exit,
                enabled: true,
                priority: 1,
                params: [
                    ("atr_multiplier".to_string(), serde_json::json!(2.5)),
                    ("atr_period".to_string(), serde_json::json!(14)),
                ]
                .into_iter()
                .collect(),
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            scoring: ScoringConfig {
                timescale_weights: [(Timescale::FiveMinute, 0.6), (Timescale::OneHour, 0.4)]
                    .into_iter()
                    .collect(),
                entry_threshold: 0.5,
                exit_threshold: -0.3,
                aggregation: AggregationMethod::WeightedSumWithGates,
                hard_gate_timescales: vec![Timescale::OneHour], agreement: None, dynamic_fusion: None,
                hard_gate_indicators: std::collections::HashMap::new(),
                hourly_exit_override: None,
            },
            session: SessionConfig {
                no_new_entries_after: "15:30".to_string(),
                force_exit_by: "15:55".to_string(),
                avoid_first_minutes: 5,
                max_concurrent_positions: 3,
                max_capital_deployed_pct: 0.8,
                entry_cooldown_ms: 0,
                max_daily_loss_pct: None,
                max_position_pct: None,
            },
            ticker_overrides: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn page_toggle_cycles() {
        let mut state = DashboardState::new(1);
        assert_eq!(state.active_page, TuiPage::Dashboard);

        state.toggle_page();
        assert_eq!(state.active_page, TuiPage::ConfigViewer);
        assert_eq!(state.config_scroll_offset, 0);

        // set some scroll offset, then toggle back
        state.config_scroll_offset = 10;
        state.toggle_page();
        assert_eq!(state.active_page, TuiPage::Dashboard);
        assert_eq!(state.config_scroll_offset, 0); // reset on toggle

        state.toggle_page();
        assert_eq!(state.active_page, TuiPage::ConfigViewer);
    }

    #[test]
    fn scroll_bounds() {
        let mut state = DashboardState::new(1);

        // scroll_up from 0 stays at 0
        state.scroll_up();
        assert_eq!(state.config_scroll_offset, 0);

        // scroll_down clamps at max
        state.scroll_down(5);
        assert_eq!(state.config_scroll_offset, 1);
        state.scroll_down(5);
        state.scroll_down(5);
        state.scroll_down(5);
        state.scroll_down(5);
        assert_eq!(state.config_scroll_offset, 5);
        state.scroll_down(5);
        assert_eq!(state.config_scroll_offset, 5); // clamped

        // page scroll
        state.config_scroll_offset = 3;
        state.scroll_page_up(10);
        assert_eq!(state.config_scroll_offset, 0); // 3 - 5 = saturates to 0

        state.config_scroll_offset = 0;
        state.scroll_page_down(10, 20);
        assert_eq!(state.config_scroll_offset, 5); // 0 + 5 = 5

        state.scroll_page_down(10, 6);
        assert_eq!(state.config_scroll_offset, 6); // clamped to max
    }

    #[test]
    fn set_strategy_config_updates_total_lines() {
        let mut state = DashboardState::new(1);
        assert_eq!(state.config_total_lines, 0);
        assert!(state.strategy_config.is_none());

        let config = make_test_strategy_config();
        state.set_strategy_config(config);

        assert!(state.strategy_config.is_some());
        assert!(state.config_total_lines > 0);
    }

    #[test]
    fn render_config_page_empty_does_not_panic() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = DashboardState::new(1);
        state.active_page = TuiPage::ConfigViewer;

        terminal
            .draw(|frame| {
                render_config_page(frame, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_config_page_with_data_does_not_panic() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = DashboardState::new(1);
        state.active_page = TuiPage::ConfigViewer;
        state.set_strategy_config(make_test_strategy_config());

        terminal
            .draw(|frame| {
                render_config_page(frame, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_config_page_scrolled_does_not_panic() {
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = DashboardState::new(1);
        state.active_page = TuiPage::ConfigViewer;
        state.set_strategy_config(make_test_strategy_config());
        // scroll near the end
        state.config_scroll_offset = state.config_total_lines;

        terminal
            .draw(|frame| {
                render_config_page(frame, &state);
            })
            .unwrap();
    }
}
