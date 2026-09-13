-- v17: two entry filters on v16's short windows, from the entry-feature screen + replay
-- (docs/paper_trading_plan_2026-09.md §11, docs/analysis/2026-09-12_entry_screen.md).
--
--   require SPY session return within ±0.2 % on the entry bar   (cross_1m score in [-0.4, 0.4],
--                                                                 scale_pct 0.005)
--   require VPIN (1-minute, 20 buckets) raw value >= 0.217        (top quintile, fixed threshold)
--
-- five-year cached replay, honest costs, 36 % sizing (tag e_spy02_vpin_cond):
--   2022 +1,304 (PF 2.31)  2023 +60 (1.06)  2024 +158 (1.18)  2025 +588 (1.74)  2026 +675 (1.96)
--   total +2,785 / 469 trades / PF 1.64 / maxDD $401 / positive months 38 of 56
--   v16 for reference: +1,684 / 1,179 / PF 1.12 / maxDD $702.
-- reading: short idiosyncratic weakness with informed flow; do not chase a market-wide move.
-- expect ~0.4 trades/day. no feature in the screen worked as a trigger; these are filters.
--
-- live requirement: MarketState.cross must be populated by the feed (SPY subscription); if it
-- is None the cross_1m indicator returns None and the windows cannot fire (fail safe).

INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted',
    now(),
    'human',
    cv.id,
    'v17: v16 + entry filters: SPY session return within ±0.2% (cross_1m) and VPIN raw >= 0.217 (vpin_1m) on both short windows. honest five-year replay +2,785 / 469 trades / PF 1.64 / maxDD 401 vs v16 +1,684 / PF 1.12.',
    (
        cv.config_blob
        || jsonb_build_object(
            'config_id', 17,
            'parent_config_id', cv.config_blob->'config_id',
            'created_at', to_jsonb(now()),
            'created_by', 'human'
        )
        || jsonb_build_object(
            'indicators',
            cv.config_blob->'indicators'
            || '[
                {"indicator_type": "cross_context", "instance_id": "cross_1m", "timescale": "OneMinute", "weight": 0.0, "enabled": true,
                 "params": {"field": "index_session_ret", "scale_pct": 0.005}},
                {"indicator_type": "vpin", "instance_id": "vpin_1m", "timescale": "OneMinute", "weight": 0.0, "enabled": true,
                 "params": {"bucket_count": 20}}
            ]'::jsonb
        )
        || jsonb_build_object(
            'actions',
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN a->>'instance_id' IN ('window_5m_thrust_short', 'window_strong_core_short') THEN
                            jsonb_set(
                                jsonb_set(a, '{params,conditions}',
                                    (a->'params'->'conditions')
                                    || '[
                                        {"type": "indicator_min", "instance_id": "cross_1m", "min_score": -0.4},
                                        {"type": "indicator_max", "instance_id": "cross_1m", "max_score": 0.4},
                                        {"type": "indicator_min", "instance_id": "vpin_1m.raw_vpin", "min_score": 0.217}
                                    ]'::jsonb),
                                '{modification_reason}', '"v17: SPY-flat (±0.2%) and VPIN top-quintile filters (entry screen 2026-09-12)"'::jsonb)
                        ELSE a
                    END
                    ORDER BY ord
                )
                FROM jsonb_array_elements(cv.config_blob->'actions') WITH ORDINALITY AS t(a, ord)
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
