-- clean up agent_type and memo_type enums after consolidating from 7 agents to 2.
-- user approved truncating all dependent tables to allow enum recreation.

-- 1. drop views that depend on config_changelog
DROP VIEW IF EXISTS changelog_since;
DROP VIEW IF EXISTS changelog_by_timescale;

-- 2. truncate all tables that reference agent_type or memo_type
TRUNCATE agent_memos, evolution_cycles, config_changelog, daily_budget CASCADE;
-- also clear trades and indicator_snapshots for a clean slate
TRUNCATE trades, indicator_snapshots CASCADE;

-- 3. recreate agent_type with only the values we need
-- postgres cannot drop enum values, so we must drop and recreate the type.
-- alter columns to text first, then drop the old type and create the new one.

ALTER TABLE agent_memos ALTER COLUMN agent TYPE text;
ALTER TABLE evolution_cycles ALTER COLUMN agents_triggered TYPE text[];
ALTER TABLE evolution_cycles ALTER COLUMN agents_completed TYPE text[];
ALTER TABLE config_changelog ALTER COLUMN changed_by TYPE text;

DROP TYPE agent_type;

CREATE TYPE agent_type AS ENUM (
    'agent_analysis',
    'agent_pm',
    'orchestrator'
);

ALTER TABLE agent_memos ALTER COLUMN agent TYPE agent_type USING agent::agent_type;
ALTER TABLE evolution_cycles ALTER COLUMN agents_triggered TYPE agent_type[] USING agents_triggered::agent_type[];
ALTER TABLE evolution_cycles ALTER COLUMN agents_completed TYPE agent_type[] USING agents_completed::agent_type[];
ALTER TABLE config_changelog ALTER COLUMN changed_by TYPE agent_type USING changed_by::agent_type;

-- 4. add 'analysis' to memo_type
ALTER TYPE memo_type ADD VALUE IF NOT EXISTS 'analysis';

-- 5. update agent_memos default (was 'recommendation', now 'observation' is safer)
ALTER TABLE agent_memos ALTER COLUMN memo_type SET DEFAULT 'observation';

-- 6. recreate the views we dropped
CREATE VIEW changelog_by_timescale AS
SELECT
    cl.*,
    cv.promoted_at,
    cv.status AS config_status
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status IN ('promoted', 'rolled_back')
ORDER BY cl.created_at DESC;

CREATE VIEW changelog_since AS
SELECT
    cl.*,
    cv.promoted_at
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status = 'promoted'
ORDER BY cv.promoted_at ASC, cl.id ASC;
