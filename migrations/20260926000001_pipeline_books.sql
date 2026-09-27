-- research pipeline + shadow books (docs/plans/2026-09-26_pipeline_and_books.md §1).
--
-- books: engines hosted by the live trader. the primary is the latest promoted config on the
-- real paper broker (unchanged behaviour). shadows run a config row or a ticker set on a
-- simulated broker with their own trade tag, on the same bars at the same moment.

CREATE TABLE books (
    name               TEXT PRIMARY KEY,
    role               TEXT NOT NULL CHECK (role IN ('primary', 'shadow')),
    config_version_id  BIGINT REFERENCES config_versions(id),  -- NULL = follow the latest promoted config
    tickers            TEXT[],                                 -- NULL = the config's own tickers
    capital            DOUBLE PRECISION,                       -- NULL = the process capital
    enabled            BOOLEAN NOT NULL DEFAULT true,
    purpose            TEXT NOT NULL DEFAULT '',
    candidate_id       BIGINT,                                 -- pipeline_candidates.id when the book is a trial
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         TEXT NOT NULL DEFAULT 'human',
    retired_at         TIMESTAMPTZ,
    retire_reason      TEXT
);
CREATE UNIQUE INDEX books_one_enabled_primary ON books (role) WHERE role = 'primary' AND enabled;
INSERT INTO books (name, role, purpose)
VALUES ('primary', 'primary', 'live book: latest promoted config on the real paper broker');

-- shadow trades are written with source = 'shadow' and book = <name>, so every existing
-- source = 'paper' query keeps its meaning without edits.
ALTER TABLE trades ADD COLUMN book TEXT NOT NULL DEFAULT 'primary';
CREATE INDEX idx_trades_book_exit ON trades (book, exit_fill_at DESC);

ALTER TABLE engine_state ADD COLUMN book TEXT NOT NULL DEFAULT 'primary';
ALTER TABLE engine_state DROP CONSTRAINT engine_state_pkey;
ALTER TABLE engine_state ADD PRIMARY KEY (book, ticker);

ALTER TABLE entry_block_events ADD COLUMN book TEXT NOT NULL DEFAULT 'primary';
CREATE INDEX idx_entry_block_events_book_ts ON entry_block_events (book, ts DESC);

-- pipeline_candidates: one row per idea moving through the lane
-- proposed → backtesting → backtest_passed | backtest_failed → shadow → shadow_passed | shadow_failed
-- → promotion_proposed → promoted. the human promotes; the runner never does.
CREATE TABLE pipeline_candidates (
    id                          BIGSERIAL PRIMARY KEY,
    name                        TEXT NOT NULL,
    kind                        TEXT NOT NULL CHECK (kind IN ('ticker', 'config')),
    base_config_version_id      BIGINT REFERENCES config_versions(id),
    patch                       JSONB,
    tickers                     TEXT[],
    stage                       TEXT NOT NULL DEFAULT 'proposed' CHECK (stage IN (
                                    'proposed', 'backtesting', 'backtest_failed', 'backtest_passed',
                                    'shadow', 'shadow_failed', 'shadow_passed',
                                    'promotion_proposed', 'promoted', 'rejected', 'withdrawn')),
    gate                        TEXT NOT NULL DEFAULT 'default',
    materialized_config_id      BIGINT REFERENCES config_versions(id),
    backtest_tag                TEXT,
    backtest_result             JSONB,
    backtest_at                 TIMESTAMPTZ,
    shadow_book                 TEXT REFERENCES books(name),
    shadow_started_at           TIMESTAMPTZ,
    shadow_min_sessions         INT NOT NULL DEFAULT 20,
    shadow_min_trades           INT NOT NULL DEFAULT 15,
    shadow_result               JSONB,
    shadow_evaluated_at         TIMESTAMPTZ,
    proposed_config_version_id  BIGINT REFERENCES config_versions(id),
    cooldown_until              DATE,
    source                      TEXT NOT NULL DEFAULT 'human',
    notes                       TEXT,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_pipeline_candidates_stage ON pipeline_candidates (stage);
-- a name may be re-proposed after a failure/cooldown; only one *active* candidate per name
CREATE UNIQUE INDEX pipeline_candidates_active_name ON pipeline_candidates (name)
    WHERE stage NOT IN ('backtest_failed', 'shadow_failed', 'rejected', 'withdrawn', 'promoted');

-- book_sessions: one row per (book, eastern date) the trader actually hosted the book.
-- written on the book's first bar of the day; the pipeline counts these as trial sessions.
CREATE TABLE book_sessions (
    book          TEXT NOT NULL REFERENCES books(name),
    session_date  DATE NOT NULL,
    first_bar_at  TIMESTAMPTZ NOT NULL,
    config_version_id BIGINT,
    PRIMARY KEY (book, session_date)
);

-- provenance for rows the pipeline inserts into config_versions (created_by is the agent_type enum)
ALTER TYPE agent_type ADD VALUE IF NOT EXISTS 'pipeline';

CREATE TABLE pipeline_events (
    id            BIGSERIAL PRIMARY KEY,
    candidate_id  BIGINT NOT NULL REFERENCES pipeline_candidates(id),
    ts            TIMESTAMPTZ NOT NULL DEFAULT now(),
    from_stage    TEXT,
    to_stage      TEXT NOT NULL,
    actor         TEXT NOT NULL,
    detail        JSONB
);
CREATE INDEX idx_pipeline_events_candidate ON pipeline_events (candidate_id, ts DESC);

ALTER TABLE books
    ADD CONSTRAINT books_candidate_fk FOREIGN KEY (candidate_id) REFERENCES pipeline_candidates(id);
