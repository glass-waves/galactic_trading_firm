-- v14: sizing 0.15 -> 0.30 (operator decision, 2026-09-11). trade selection is
-- unchanged from v13; only position size and therefore P&L scale and drawdown change.
-- at 30 % of $10k, the corrected-replay expectation is roughly 2026 +$2,280 (maxDD ~4 %),
-- 2025 -$1,610 (maxDD ~20 %).
INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted',
    now(),
    'human',
    cv.id,
    'v14: sizing 0.15 -> 0.30 per operator decision; trade selection unchanged from v13',
    (
        cv.config_blob
        || jsonb_build_object(
            'config_id', 14,
            'parent_config_id', cv.config_blob->'config_id',
            'created_at', to_jsonb(now()),
            'created_by', 'human',
            'session', (cv.config_blob->'session') || '{"max_position_pct": 0.30}'::jsonb
        )
        || jsonb_build_object(
            'actions',
            (
                SELECT jsonb_agg(
                    CASE WHEN a->>'instance_id' = 'sizing_fixed'
                         THEN jsonb_set(a, '{params,fraction}', '0.30'::jsonb)
                         ELSE a END
                )
                FROM jsonb_array_elements(cv.config_blob->'actions') a
            )
        )
    )
FROM config_versions cv
WHERE cv.status = 'promoted'
ORDER BY cv.id DESC
LIMIT 1;

UPDATE config_versions
SET status = 'superseded'
WHERE status = 'promoted'
  AND id < (SELECT max(id) FROM config_versions WHERE status = 'promoted');
