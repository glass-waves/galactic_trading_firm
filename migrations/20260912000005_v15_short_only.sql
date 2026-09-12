-- v15: short-only. the three long windows are disabled; everything else is v14.
--
-- evidence (corrected replay, five years, 36 % sizing, real short-only runs):
--   short book:  2022 +$3,564 (PF 1.56)  2023 +$4xx (PF ~1.2)  2024 +$395 (PF 1.13)
--                2025 +$699 (PF 1.20)    2026 +$665 (PF 1.26)   — positive every year
--   long book:   2022 -$2,049  2023 +$512  2024 -$690  2025 -$2,659  2026 +$2,095
--                — net -$2,790 over five years, positive in two.
-- the long windows are a bet on positive morning drift in the mega-caps; the short
-- windows work in every regime tested. both short twins contribute ("5m thrust short"
-- +$3,357 / 4 of 5 years, "strong core short" +$2,369 / 5 of 5).
-- max drawdown of the short book at 36 %: $412–$870 (4–9 %); at 30 % proportionally less.
-- expect ~1 trade/day.
--
-- the long windows are disabled, not removed, so a later config can re-enable them
-- (e.g. behind a regime gate) without re-specifying them.

INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted',
    now(),
    'human',
    cv.id,
    'v15: short-only — long windows disabled. five-year corrected-replay evidence: short book positive every year (+$5.8k at 36%), long book net negative (-$2.8k). sizing unchanged (0.30).',
    (
        cv.config_blob
        || jsonb_build_object(
            'config_id', 15,
            'parent_config_id', cv.config_blob->'config_id',
            'created_at', to_jsonb(now()),
            'created_by', 'human'
        )
        || jsonb_build_object(
            'actions',
            (
                SELECT jsonb_agg(
                    CASE WHEN a->>'instance_id' IN ('window_5m_thrust', 'window_strong_core', 'window_candle_reversal')
                         THEN jsonb_set(
                                jsonb_set(a, '{enabled}', 'false'::jsonb),
                                '{modification_reason}', '"v15: long book disabled (net negative over 2022-2026)"'::jsonb)
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
