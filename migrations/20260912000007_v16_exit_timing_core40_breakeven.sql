-- v16: exit timing + strong-core hourly condition + working breakeven stop. everything else is v15.
--
-- three changes, each tested alone and together under the CORRECTED cost model
-- (direction-aware slippage; the model before 2026-09-12 credited shorts ~$4.5/trade —
-- see docs/paper_trading_plan_2026-09.md §10). five-year cached replay, 36 % sizing:
--
--   variant                              2022    2023    2024    2025    2026     5y    maxDD
--   v15 as promoted (honest costs)     +1,852    -463    -496    -470    -312    +112   2,333
--   losing-side hold 40 min, win 90    +1,577    -449     -56    -254     +57    +875   1,554
--   strong-core s1h <= -0.40           +1,791     +48    -374     -71    -310  +1,084   1,471
--   breakeven 0.5 % (now functional)   +1,732    -646     -83    -363    -246    +394   1,951
--   all three (v16)                    +1,380    -177    +229     +99    +153  +1,684     702
--
-- why: losers are identifiable by 30-40 min (74 % of eventual losers are underwater at
-- 15 min, 90 % at 45) while winners are positive early; the losing-side hold limit is the
-- knob. the strong-core window's hourly condition at -0.30 admitted base-rate bars. the
-- breakeven monitor emitted ModifyStop that the engine ignored until commit 3e60bee.
-- expect: fewer winners (win rate ~34 %), many small breakeven exits, drawdown ~70 % lower.
-- the honest edge is thin (PF 1.12); week 1 remains a plumbing test.
--
--   max_hold.loss_reduction_ms   900,000 -> 3,000,000   (losing limit 75 -> 40 min)
--   max_hold.profit_extension_ms 1,800,000 -> 0         (winning limit 120 -> 90 min)
--   window_strong_core_short conditions[2] (OneHour timescale_max) -0.30 -> -0.40
--   breakeven.trigger_pct / breakeven_trigger_pct 0.015 -> 0.005

INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted',
    now(),
    'human',
    cv.id,
    'v16: losing-side hold 40 min / winning 90 (loss_reduction_ms 3000000, profit_extension_ms 0); strong-core short s1h <= -0.40; breakeven stop 0.5% (now functional). honest-cost five-year replay +1,684 vs +112 for v15, maxDD 702 vs 2,333.',
    (
        cv.config_blob
        || jsonb_build_object(
            'config_id', 16,
            'parent_config_id', cv.config_blob->'config_id',
            'created_at', to_jsonb(now()),
            'created_by', 'human'
        )
        || jsonb_build_object(
            'actions',
            (
                SELECT jsonb_agg(
                    CASE
                        WHEN a->>'instance_id' = 'max_hold' THEN
                            jsonb_set(
                                jsonb_set(
                                    jsonb_set(a, '{params,loss_reduction_ms}', '3000000'::jsonb),
                                    '{params,profit_extension_ms}', '0'::jsonb),
                                '{modification_reason}', '"v16: losing-side hold 40 min, winning 90 (exit-timing analysis 2026-09-12)"'::jsonb)
                        WHEN a->>'instance_id' = 'breakeven' THEN
                            jsonb_set(
                                jsonb_set(
                                    jsonb_set(a, '{params,trigger_pct}', '0.005'::jsonb),
                                    '{params,breakeven_trigger_pct}', '0.005'::jsonb),
                                '{modification_reason}', '"v16: breakeven stop 0.5% (engine now honours ModifyStop)"'::jsonb)
                        WHEN a->>'instance_id' = 'window_strong_core_short' THEN
                            jsonb_set(
                                jsonb_set(a, '{params,conditions,2,max_score}', '-0.40'::jsonb),
                                '{modification_reason}', '"v16: hourly condition -0.30 -> -0.40 (missed-setup analysis 2026-09-12)"'::jsonb)
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
