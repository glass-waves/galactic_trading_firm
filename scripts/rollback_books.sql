-- rollback for the multi-book trader (docs/plans/2026-09-26_pipeline_and_books.md §9, review B1).
--
-- the pre-books binary (target/release/paper_trader.pre-books) upserts engine_state with
-- ON CONFLICT (ticker). migration 20260926000001 made the primary key (book, ticker), so
-- that upsert fails (42P10: no unique constraint on ticker alone) and the old binary never
-- heartbeats. rollback = restore the old binary AND run this file:
--
--   systemctl --user stop paper-trader.service
--   cp target/release/paper_trader.pre-books target/release/paper_trader
--   ./scripts/psql.sh < scripts/rollback_books.sql      # stdin: psql may run inside the container
--   systemctl --user start paper-trader.service
--
-- the `book` columns stay (DEFAULT 'primary' keeps the old binary's INSERTs valid); only the
-- shadow rows and the composite key go. re-running the new binary later needs the composite
-- key back:  ALTER TABLE engine_state DROP CONSTRAINT engine_state_pkey,
--            ADD PRIMARY KEY (book, ticker);
--
-- test it on a scratch copy before touching the real table:
--   CREATE TABLE engine_state_scratch AS SELECT * FROM engine_state;
--   ALTER TABLE engine_state_scratch ADD PRIMARY KEY (book, ticker);
--   then run these statements with engine_state_scratch in place of engine_state.

BEGIN;

DELETE FROM engine_state WHERE book <> 'primary';

ALTER TABLE engine_state DROP CONSTRAINT engine_state_pkey;
ALTER TABLE engine_state ADD PRIMARY KEY (ticker);

COMMIT;
