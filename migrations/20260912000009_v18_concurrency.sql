-- v18: allow up to three concurrent positions (one per ticker) and raise the deployed-capital
-- cap to 95 %. everything else is v17.
--
-- why: the replay runs each ticker independently, i.e. with no cross-ticker cap, and the v17
-- evidence (+2,785 / 469 trades / PF 1.64) was produced that way. live, max_concurrent_positions
-- was 1, which on the same five years would have kept 349 of those trades (+1,438, PF 1.40) and
-- dropped 120 worth +1,347. day 1 (2026-09-14) showed it: three tickers blocked 09:36–10:06 ET
-- while NVDA was short. with 3 concurrent: 463 trades / +2,587 / PF 1.60 (4 = the full replay).
-- exposure: three 30 % positions = 90 % of the $10k budget short at once, hence the 0.95 cap.

INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted', now(), 'human', cv.id,
    'v18: max_concurrent_positions 1 -> 3, max_capital_deployed_pct 0.50 -> 0.95. aligns live with the per-ticker replay that produced the v17 evidence (cap of 1 would have dropped 120 of 469 trades worth +1,347).',
    cv.config_blob
    || jsonb_build_object('config_id', 18, 'parent_config_id', cv.config_blob->'config_id', 'created_at', to_jsonb(now()), 'created_by', 'human')
    || jsonb_build_object('session', cv.config_blob->'session' || '{"max_concurrent_positions": 3, "max_capital_deployed_pct": 0.95}'::jsonb)
FROM config_versions cv WHERE cv.status = 'promoted' ORDER BY cv.id DESC LIMIT 1;

UPDATE config_versions SET status = 'superseded'
WHERE status = 'promoted' AND id < (SELECT max(id) FROM config_versions WHERE status = 'promoted');
