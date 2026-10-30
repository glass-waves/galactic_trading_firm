# books in the live trader — developer notes (2026-09-26)

plan `docs/plans/2026-09-26_pipeline_and_books.md` §3/§9, review `…books.review.md` (B1, S1–S9).
crate `crates/data_feed` only. pre-change binary: `target/release/paper_trader.pre-books`.

## what changed

- `book.rs` (new): `BookSpec` + `load_books(pool)` (enabled `books` rows; exactly one primary or the
  process refuses to start), `Limits` (`MAX_BOOKS` 8 by default, 12 in `paper-trader.service` since 2026-10-23 / `MAX_SYMBOLS` 25: excess *shadows* skipped with
  `error!`, never the primary), `Book` = engines per ticker + broker + sink + capacity + daily P&L +
  pending config + gate/near-miss dedupe. `Book::new_shadow` builds its own `SimulatedBroker` (5 bps);
  `new_primary` is the only constructor taking a broker and `main` the only `AlpacaBroker::new` site,
  so a shadow cannot reach Alpaca whatever `BROKER_MODE` says.
- `main.rs`: `Runtime` = per-symbol state (cross tracker, candle windows, last bar/price/stale, ticks)
  + `books` (primary first). one `MarketState` per ticker per bar, cloned per extra book, with
  `cross = context_for(ticker, book.tickers(), ts)` (peers = that book's tickers). subscriptions,
  warm-up and the demo feed = union of the books' tickers + SPY. hot reload: books with
  `config_version_id IS NULL` take the new promoted row (ticker override re-applied); a shared candle
  window is dropped only when no book trades the ticker. startup deletes `engine_state` rows for
  (book, ticker) pairs not hosted. shutdown / ctrl-c / SIGTERM / clock safety net flatten every book,
  each with its own `force_exit_by`. TUI = primary only.
- `broker.rs`: `set_last_price_for(ticker, price)` (per-ticker simulated fills) and `kind()`.
- writers: `TradeWriter::new(pool, source, book)`; `EngineStateRow.book` + `ON CONFLICT (book, ticker)`;
  `delete_engine_state(pool, book, ticker)`; `delete_unhosted_engine_state`; `BlockEvent.book`;
  `write_book_session` on each book's first bar of the eastern date. sources: primary `paper`/`demo`
  as before, shadows `shadow` (`demo` under `--demo`). `config_loader::load_config_by_id` for pinned rows.

primary with zero shadows — touched paths: `set_last_price` → `set_last_price_for` (no-op on Alpaca;
per-ticker fills in simulated mode, so a flatten no longer fills every name at the last-arrived
ticker's price), `book='primary'` on rows/events/log lines, a `book_sessions` row per day, startup
cleanup of unhosted state rows, demo feed also emits SPY, a closed bar channel logs one warning (was
silent). capacity, daily P&L, clocks, pending swap, dedupe and TUI are the same logic moved into `Book`.

## add / retire a book (SQL; effective at the next trader start)

```sql
INSERT INTO books (name, role, tickers, purpose) VALUES ('shadow:amd', 'shadow', '{AMD}', 'ticker trial');
INSERT INTO books (name, role, config_version_id, purpose) VALUES ('shadow:bb', 'shadow', 14, 'config trial');
UPDATE books SET enabled = false, retired_at = now(), retire_reason = '…' WHERE name = 'shadow:amd';
```
`capital` (NULL = process capital) / `tickers` may be set on either kind. a shadow that fails to build
or exceeds the limits is logged `shadow book NOT hosted` and writes no state row (watchdog sees it).

## replay harness

`paper_trader --replay-bars data/bars_iex --replay-date 2025-04-04 --replay-out out.csv [--replay-slippage-bps 0]`
— same loop and books, bars from `<dir>/<SYMBOL>.csv` (unix-second ts,o,h,l,c,v; RTH), warm-up = the
per-symbol calendar days before the date (see below; 8 unless a book asks for more), SPY first within each minute (backtest lag 0), simulated brokers for
every book, **no database writes**, heartbeat / staleness / reload arms off, every `Utc::now()` stamp =
bar time, flatten at the last bar, exit 0. output = the backtest `--output-trades-csv` trade rows
(same quirk: one field fewer than the header) + `book`. log: `$LOG_DIR/replay_<date>.log`.

## per-symbol warm-up (2026-10-07)

`SessionConfig.warmup_days: Option<u32>` (serde default, not serialized when unset). at start-up
`book::warmup_plan` gives each traded symbol max over the books trading it of (`warmup_days` or 8) and
the untraded index (SPY) max over all books; a traded index follows the traded rule. live fetches and
`--replay-bars` use the same per-symbol days; computed once per start (a reloaded value applies at the
next start). the index only seeds the cross tracker (today's session + prior close), so its length
changes nothing a book computes; live, its seed went from 1 day to that max (≥ 8). a primary name raised
above 8 logs `symbol X warm-up raised to N days by book Y — primary parity with 8-day replays no longer
holds for X`. backtest: `--lookback-days` when given, else the (patched) config's `warmup_days`, else 5.
check 2026-10-07: shadow on promoted row 12 + `qqq_noise_pm_vol` patch (`warmup_days` 30) → log QQQ 30 /
SPY 30 / primary names 8; primary (and every other shadow) trade rows byte-identical with and without the
book on 2025-02-03/24/25/27/28 and 04-04 (6 primary trades); QQQ 2025-02-27 Short 16:59Z @ 509.90 →
SessionClose 20:58Z, scores equal to `backtest --lookback-days 22|30 --config-id` (17:00Z @ 510.125, next-bar
fill); with `--lookback-days 8` the backtest takes no trade.

## parity (2025-04-04, capital 10000, blob sizing 0.30, zero costs)

backtest `--date 2025-04-04 --lookback-days 8 --bars-dir data/bars_iex --cross-index SPY --capital 10000
--slippage-bps 0 --half-spread 0 --output-trades-csv`: 1 trade, NVDA Short, size 30, entry 13:33Z @ 98.19,
exit 15:04Z @ 95.32, MaxHoldTimeout, pnl 86.10, entry scores −0.6010/−0.7959/−0.6654/−0.4071
(composite/1m/5m/1h), `window:strong core short`, `three_outside_down`.
harness (primary only): 1 trade, NVDA Short, size 30, entry 13:32Z @ 98.21, exit 15:02Z @ 95.32,
MaxHoldTimeout, pnl 86.70, identical entry scores and reason. only the fill model differs: the backtest
defers fills to the next bar's open (entry +1 bar, hold clock starts a bar later, exit +1 bar again); the
live engine fills at the signal bar's close. with a shadow on config row 10 both books produced the same
trade under their own `book` tag; 0 rows reached the database.
pre-existing gap, not fixed (`crates/backtest/src/replay.rs`): the backtest's `session_vwap` never
resets across the lookback days (107.5 vs live ≈ 98.9 at the entry bar); on this day both saturate
`vwap_dist_1hr` at −1, so the scores agree — on a calm day they will not.

## demo smoke

`demo-shadow` on `{AAPL}`, `BROKER_MODE=simulated paper_trader --demo` 90 s: `engine_state` rows primary × 4 +
demo-shadow × AAPL, `book_sessions` for both, SIGTERM flattened both, exit 0; every demo row deleted afterwards.

## rollback (Monday go/no-go before 06:10 PT)

the old binary's `ON CONFLICT (ticker)` upsert fails on the `(book, ticker)` key (review B1):
`systemctl --user stop paper-trader.service; cp target/release/paper_trader.pre-books target/release/paper_trader;
./scripts/psql.sh < scripts/rollback_books.sql; systemctl --user start paper-trader.service`.
the sql deletes non-primary `engine_state` rows and restores `PRIMARY KEY (ticker)`; verified on a scratch
copy both ways (old upsert fails before, works after with `book` defaulting to `'primary'`). forward
again: `ALTER TABLE engine_state DROP CONSTRAINT engine_state_pkey, ADD PRIMARY KEY (book, ticker);`.

## broker ↔ engine invariant (2026-10-15 incident)

alpaca started rejecting apca 0.30's order paths: `order::Get` / `Delete` / `Change` format
`/v2/orders/{id}` with `Uuid::as_simple()` (no hyphens) and alpaca answers 422 `invalid order_id`
(`curl /v2/orders/4f807b122af249ad952bc39c2b212234` → 422; the hyphenated id → 200). the fill poll
errored, the book undid the engine position, the window re-fired on the next bars: five fills
orphaned (−48 AMZN, −14 NVDA) the engine never knew about.

- `broker.rs`: `OrderGetRaw` / `OrderCancelRaw` send the id string exactly as returned
  (`order_id_string`, hyphenated); every order carries `client_order_id = gtf-<book>-<ticker>-<unix_ms>`.
  `wait_for_fill_with` (generic over `OrderApi`, unit-tested with a scripted mock): a failed poll is
  retried 3× with backoff, then resolved by `GET /v2/orders:by_client_order_id`, then by the positions
  list; at the deadline the order is cancelled and re-read (a fill that raced the cancel is returned).
  `Err` now means the broker holds nothing for the order — or `BrokerError::Unconfirmed` when it could
  not be asked. `close_orphan` = market order (RTH) or extended-hours day limit at the last trade ±0.3 %.
- `book.rs`: `handle_failed_entry` undoes the engine, gates the ticker (`order_failed_cooldown`,
  max(`entry_cooldown_ms`, 5 min), engine `suppress_entries_until`) and, primary only, asks the broker:
  a held position is adopted (`LiveSession::adopt_broker_position` → `force_open_position` with the
  broker's qty/fill, entry_reason `adopted:failed-order (...)`) and the human gets a CRITICAL via
  `scripts/notify.sh` (`notify.rs`, detached thread); an unreachable broker is re-asked on the next bars.
  `reconcile_orphans` at start-up follows `ORPHAN_POLICY` (`adopt` default: adopt traded tickers,
  close the rest; `close`; `ignore` = the old log-only behaviour) and notifies per orphan.
- verified 2026-10-15 17:08 ET on the paper account: 1 AAPL extended-hours limit round trip
  (`live_order_roundtrip`): open filled @ 342.27 through the fixed poll, close filled @ 342.18,
  `/v2/positions` flat for AAPL.

## exits are never pretended (2026-10-16 incident)

09:31:01 ET the primary's AMZN trailing stop fired on the adopted −48. `close_position`
(DELETE /v2/positions/AMZN → market BUY 48, order `7d819398-…`, alpaca's own uuid
`client_order_id`) was created 13:31:00.62Z, `submitted_at` 13:31:01.78Z, and sat
`accepted` with `filled_qty 0`; at the 15 s deadline the fill-wait **cancelled it**
(`canceled_at` 13:31:18.69Z), re-read it as still working, returned `Unconfirmed`, and the
book logged "broker close FAILED (engine already closed)" and wrote trade 22 (−245.77)
anyway. the broker kept AMZN −48 all day; the engine thought it was flat. NVDA's close at
09:52 filled in 0.9 s. lesson: a paper market order at the open can stay unfilled past
15 s, and the old path turned that into a cancel plus a recorded exit that never happened.

- `broker.rs`: `FillWait::close()` = `CLOSE_FILL_TIMEOUT_MS` 120 s, `cancel_on_timeout
  false`. a close still working at the deadline comes back as `BrokerError::Working
  { order_id }` and is left working — never cancelled for lack of confirmation, also when
  the broker cannot be asked. a partial fill keeps waiting for the remainder. the one
  cancel on the close path is after an explicit rejection / terminal status (idempotent,
  before the book re-submits). `Broker::order_status(order_id)` reads a working order
  (hyphenated id); the simulated broker has none. `close_orphan` during RTH is the same path.
- `book.rs` **exit pending**: on `Working` / any error but `NoPosition`, the book
  reverts the engine's close (`TradingEngine::revert_last_close` puts the position back
  with its water marks and undoes realized P&L, the loss breaker and the cooldown clock;
  `LiveSession::reopen_after_unconfirmed_close` restores the entry attribution), records
  **no trade**, and tracks `ExitPending { order_id, reason, engine_exit_price, since }`.
  every bar of the ticker and every 30 s heartbeat re-checks: order filled → trade at
  the real fill with the original exit reason; order ended but the broker still holds →
  a new close is submitted; broker flat (the human closed it) → trade at the last known
  price, WARNING; still working after `EXIT_PENDING_ALERT_SECS` (5 min) → one CRITICAL.
  while pending the exit stack re-fires every bar and is reverted silently; the
  force-exit / session-close safety net (`close_and_record`) takes the same path and
  only re-checks a ticker that is already pending. shutdown with an exit pending logs +
  notifies CRITICAL and records nothing (startup reconciliation adopts what is held).
- `book.rs` **intraday reconciliation** `reconcile_live` (primary, every
  `LIVE_RECONCILE_SECS` = 5 min during regular hours, from the heartbeat): the startup
  orphan policy for broker positions no engine knows (`adopted:orphan-at-intraday`,
  untraded → close, notify CRITICAL), and the reverse — an engine position the broker
  does not hold is recorded closed at the last price, `ManualOverride`, log/notify
  "broker-flat" WARNING. either mismatch needs `MISMATCH_SIGHTINGS` = 2 consecutive
  checks (an order can be in flight); tickers with an exit pending or an unreconciled
  entry are skipped; the count resets when a check agrees.
- tests (mock `OrderApi` / scripted `Broker`): close unfilled past the deadline → Working,
  no cancel; partial then full; partial at the deadline stays Working; rejection →
  cancel + `OrderRejected`; unreachable broker → Working; exit-pending lifecycle (pending →
  filled on a later bar with the fill price; pending → broker flat; unconfirmed force close →
  kept → re-submitted → filled; the 5-min alert fires once); reconciliation adopt / close
  / broker-flat after two sightings, debounce reset, pending tickers skipped; engine
  `revert_last_close`. live after hours: `live_order_roundtrip` and
  `live_close_path_roundtrip` (1 AAPL; the queued market close must come back Working and
  un-cancelled, the test cancels it and flattens with a limit).

### lessons

1. 2026-10-15: an order id sent in a format the broker rejects turned every fill into
   "failed" — five orphans. poll by the id exactly as returned, carry a `client_order_id`,
   and never conclude "not filled" from a failed call.
2. 2026-10-16: a close that times out must stay working and the engine must stay in the
   position until the broker says otherwise. cancelling an exit and recording it anyway
   is the one combination that leaves a position nobody is managing.
3. the broker's positions list is the ground truth; compare it to the engines every
   few minutes in both directions, with a debounce, instead of only at startup.

## 2026-10-23 audit changes and known gaps

changed (`crates/data_feed`, `crates/engine`):
- `CLOSE_FILL_TIMEOUT_MS` 120 s → 20 s and `HTTP_TIMEOUT` 15 s on every broker call. a broker wait runs inline in the bar
  loop (every other ticker and book waits), so a close still working at 20 s becomes exit-pending — the heartbeat (30 s) and
  the next bar re-check it — instead of stalling the loop for two minutes. a timed-out submit is `Unconfirmed` (the broker may
  have the order), which routes through `handle_failed_entry` → `reconcile_ticker`.
- shutdown flattens the shadows first and the primary last; `TimeoutStopSec` 90 → 300 s.
- a ticker with an unreconciled entry blocks new entries until the broker answers (before, only the 5-minute suppression did).
- a partial entry fill resizes the engine position to the filled quantity (`LiveSession::resize_open`).
- a hot-reload rebuild carries the per-day state (`TradingEngine::carry_day_state`): breaker, realised P&L, cooldown clocks.
- a trade the database refuses is appended to `$LOG_DIR/trade_spool.jsonl`; the pool acquire timeout is 5 s (was sqlx's 30 s).
- actions run under `catch_unwind` like indicators (a panicking action is `Hold`).

known gaps, not yet closed (ordered by how likely they are to bite):
1. an exit still pending at shutdown loses its trade record if the order fills after the process exits (the next start sees a
   flat broker and adopts nothing). fix: persist `ExitPending` (order id, reason, entry stash) and resolve it at startup.
2. startup never lists open orders: a working close left by the previous process is adopted as an orphan and closed a second
   time. fix: list `gtf-*` open orders at start, seed exit-pending from them, cancel stale entry orders.
3. an `Unconfirmed` cancel ("still working after cancel") clears the ticker as flat as soon as the positions list is empty,
   although the order is alive; when it fills later it is found as an orphan 5–10 min on. fix: carry the order id and clear only
   on a terminal status.
4. `reconcile_live` compares presence, not quantity/direction.
5. startup picks the promoted row by `promoted_at`, the watcher by `id > current`; with the documented supersede-first SQL only
   one row is promoted, so this is moot unless two rows are left promoted.
6. an orphan adopted outside regular hours is flattened by the heartbeat with a market order (queues to the next open) instead
   of `close_orphan`'s after-hours limit path.
7. the IEX websocket sometimes never delivers the 15:59 ET bar (engine_state.last_bar_at 15:58 on 10-20, 10-22, 10-28,
   10-29; 15:59 on 10-23/26/27 — same binary either way, so it is the stream, not the trader: Alpaca closes a minute bar on
   the next trade and IEX can go quiet after the bell). no book trades after 15:55, so nothing depends on it; the history
   cache has the bar and the replay harness will differ by that one bar.
8. max-hold clock: live starts the hold at the SIGNAL bar, the backtest at the FILL bar (next bar's open,
   `crates/backtest/src/replay.rs` deferred fill), so live max-hold exits land one bar earlier than the replay
   (10-29: 11:08/11:17/11:34/11:36 live vs 11:09/11:18/11:35/11:37). a parity fix moves every max-hold exit in the
   record by a bar, so it goes into the next restatement bundle (research_queue RQ-14), not a quiet patch.
9. warm-up: 8 calendar days leaves the 1h ADX (29 hourly candles) missing until ~11:00 ET on the session after a
   holiday weekend (2025-12-01), and the 1h sum silently re-weights; `session.warmup_days` 12 fixes it (RQ-14).
10. half-day closes are not modelled (stale-feed alerts and `reconcile_live` keep running 13:00–16:00 ET).
