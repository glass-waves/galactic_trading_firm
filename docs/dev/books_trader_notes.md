# books in the live trader — developer notes (2026-09-26)

plan `docs/plans/2026-09-26_pipeline_and_books.md` §3/§9, review `…books.review.md` (B1, S1–S9).
crate `crates/data_feed` only. pre-change binary: `target/release/paper_trader.pre-books`.

## what changed

- `book.rs` (new): `BookSpec` + `load_books(pool)` (enabled `books` rows; exactly one primary or the
  process refuses to start), `Limits` (`MAX_BOOKS` 8 / `MAX_SYMBOLS` 25: excess *shadows* skipped with
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
