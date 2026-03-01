CREATE TABLE config_changelog (
    id                  BIGSERIAL PRIMARY KEY,
    config_version_id   BIGINT NOT NULL REFERENCES config_versions(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    changed_by          agent_type NOT NULL,
    change_category     change_category NOT NULL,
    target_timescale    timescale,
    target_tool_id      VARCHAR(100),
    target_tool_type    VARCHAR(100),
    target_param        VARCHAR(100),
    old_value           JSONB,
    new_value           JSONB NOT NULL,
    reason              TEXT NOT NULL,
    evidence_trade_ids  BIGINT[],
    evidence_period     TSTZRANGE,
    source_memo_id      BIGINT REFERENCES agent_memos(id),
    reverts_changelog_id BIGINT REFERENCES config_changelog(id)
);

CREATE INDEX idx_changelog_config ON config_changelog(config_version_id);
CREATE INDEX idx_changelog_timescale ON config_changelog(target_timescale);
CREATE INDEX idx_changelog_tool ON config_changelog(target_tool_id);
CREATE INDEX idx_changelog_category ON config_changelog(change_category);
CREATE INDEX idx_changelog_created ON config_changelog(created_at DESC);
CREATE INDEX idx_changelog_agent ON config_changelog(changed_by);

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
