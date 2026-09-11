-- v12: make the session times mean what they say, and add the position-size cap.
--
-- context: session_close compared the UTC hour to an Eastern-intended "15:55",
-- so every backtest behind v11 (and the live engine) was force-flat at 11:55 ET,
-- with entries still permitted until 15:30 ET and closed on the very next bar.
-- the code is now fixed (session_close.rs uses US/Eastern). to keep the
-- backtested behaviour — a morning-only strategy — this config states it
-- explicitly: no new entries after 11:30 ET, flat by 11:55 ET.
--
-- avoid_first_minutes was measured from the first tick the engine ever saw
-- (in backtests: the first lookback bar days earlier), i.e. it was effectively
-- 0 on the scored day. the engine now anchors it to the 09:30 ET open, so it
-- is set to 0 here to reproduce the validated behaviour. re-tune via backtest.
--
-- max_position_pct is a hard clamp on any single position (36% = the v11 sizing),
-- so a mis-set sizing fraction can never drive available capital negative.
--
-- everything else (indicators, windows, exits, sizing, tickers) is copied from v11.

INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted',
    now(),
    'human',
    cv.id,
    'v12: explicit morning session (entries until 11:30 ET, flat by 11:55 ET) matching what v11 actually backtested; avoid_first_minutes 0 (was a no-op); max_position_pct 0.36 safety clamp',
    jsonb_set(
        jsonb_set(
            cv.config_blob || jsonb_build_object(
                'config_id', 12,
                'parent_config_id', cv.config_blob->'config_id',
                'created_at', to_jsonb(now()),
                'created_by', 'human'
            ),
            '{session}',
            (cv.config_blob->'session') || '{
                "force_exit_by": "11:55",
                "no_new_entries_after": "11:30",
                "avoid_first_minutes": 0,
                "max_position_pct": 0.36
            }'::jsonb
        ),
        '{actions}',
        (
            SELECT jsonb_agg(
                CASE
                    WHEN a->>'instance_id' = 'exit_session'
                    THEN jsonb_set(a, '{params,force_exit_by}', '"11:55"'::jsonb)
                    ELSE a
                END
            )
            FROM jsonb_array_elements(cv.config_blob->'actions') a
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
