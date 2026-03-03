# agentic system recommendations

post-analysis of the oct 1–14 walk-forward run (10 full PM cycles, 509 trades, 40 agent memos, $9.57 total cost). this document captures both the recommendations and the evidence trail — the specific queries and data that led to each finding, so a fresh agent can verify and build on these conclusions.

---

## how to reproduce these findings

all evidence lives in postgres (`postgresql://postgres:postgres@localhost:5433/adaptive_trading`). the key tables are:

- `trades` — 509 rows from the oct 1–14 backtest run
- `agent_memos` — 40 rows (10 per agent × 4 agents per cycle)
- `evolution_cycles` — 10 rows (1 per trading day)
- `config_versions` — 3 rows (seed, v1 default, v3 post-analysis)
- `config_changelog` — 3 rows (manual v3 changes applied post-analysis)

### verification queries

**cycle type distribution** — confirms no check-in cycles ran:
```sql
SELECT cycle_type, count(*), sum(estimated_cost_usd) as total_cost
FROM evolution_cycles GROUP BY cycle_type;
-- result: full_pm | 10 | 9.573216 (zero check-in rows)
```

**memo type distribution** — confirms all memos are "recommendation", zero "observation":
```sql
SELECT agent, memo_type, count(*)
FROM agent_memos GROUP BY agent, memo_type ORDER BY agent;
-- result: agent_1min/recommendation/10, agent_5min/recommendation/10,
--         agent_hourly/recommendation/10, agent_pm/recommendation/10
```

**score column nullity** — confirms all scores were zero (the bug):
```sql
SELECT count(*) as total_trades,
  count(*) FILTER (WHERE entry_score_composite IS NULL OR entry_score_composite = 0) as zero_composite,
  count(*) FILTER (WHERE entry_score_1min IS NULL OR entry_score_1min = 0) as zero_1min,
  count(*) FILTER (WHERE entry_score_5min IS NULL OR entry_score_5min = 0) as zero_5min
FROM trades;
-- result: 509 total, 509 zero_composite, 509 zero_1min, 509 zero_5min
```

**config proposals per cycle** — confirms PM never successfully proposed:
```sql
SELECT id, trading_date, configs_proposed, configs_promoted, configs_rejected, estimated_cost_usd
FROM evolution_cycles ORDER BY id;
-- result: all 10 rows show 0/0/0 for proposed/promoted/rejected
```

**PM memos mentioning the enum bug** — the PM agent itself flagged the issue:
```sql
SELECT agent, LEFT(reasoning, 200) FROM agent_memos
WHERE agent = 'agent_pm' AND reasoning LIKE '%config_status%';
-- result: multiple PM memos contain "propose_config_mutation returned error:
--         type 'config_status' does not exist"
```

**5-min agent memos flagging zero scores** — agents noticed the score bug:
```sql
SELECT agent, LEFT(reasoning, 200) FROM agent_memos
WHERE agent = 'agent_5min' ORDER BY created_at LIMIT 3;
-- result: all begin with "DATA QUALITY WARNING: Entry Score Anomaly" noting
--         entry_score_composite = 0 across all reviewed trades
```

**`get_recent_trades` SQL** — confirms it only returns composite score:
```
-- from agents-ts/src/tools/sql-queries.ts, getRecentTrades():
SELECT id, ticker, direction, entry_price, exit_price, position_size,
       pnl_dollars, pnl_percent, hold_duration_ms, exit_reason,
       entry_fill_at, exit_fill_at, entry_score_composite, config_version_id
-- note: no entry_score_1min, entry_score_5min, entry_score_hourly columns
```

---

## what worked well

- **two-tier architecture is sound.** the separation between observation (check-in) → recommendation → decision (PM) is a good pattern. structured memo format (confidence, flags, volatility_regime, directional_bias, signal_quality) gives the PM agent a consistent interface to reason over.
- **config versioning and audit trail.** immutable append-only config_versions with changelog entries per atomic change is solid. rollback = promote older version. the PM agent correctly diagnosed the trailing stop issue and attempted a fix — it was blocked only by a code bug (`config_status` vs `mutation_status` enum name in `agents-ts/src/tools/config-ops.ts`).
- **cost efficiency.** ~$0.96/cycle, $9.57 total for 10 cycles. well under the $5/day budget cap. there's significant headroom to run more aggressively.
- **safety guardrails.** max 3 changes per proposal, backtest validation gates, budget enforcement before every agent invocation — all appropriate for an autonomous config mutation system.
- **agents correctly identified bugs.** the 5-min recommendation agent flagged zero scores in every single memo. the PM agent flagged the `config_status` enum error starting from cycle 3. the system's self-diagnostic capability is working — it just couldn't act on its findings.

---

## issues found

### 1. walk-forward skips check-in cycles entirely

**evidence:** `evolution_cycles` table shows 10 rows, all `cycle_type = 'full_pm'`. zero check-in cycles. `agent_memos` shows all 40 memos are `memo_type = 'recommendation'`, zero `'observation'`. the PM agent itself noted this — cycle 5 memo reads: "3 recommendation memos, and 0 checkin memos."

**root cause:** `agents-ts/src/walk-forward.ts` line 211 only calls `runFullPmCycle()`. it never calls `runCheckinCycle()`. in live trading, the cron scheduler (`0 10,12,14 * * 1-5`) would trigger check-ins, but walk-forward bypasses the scheduler.

**impact:** the PM agent flew blind without observation context. the entire evidence-building layer (check-in memos with volatility regime, directional bias, signal quality flags) was absent from every PM decision.

**fix:** add a check-in cycle call before the PM cycle in `walk-forward.ts`:

```typescript
// in walk-forward.ts, before runFullPmCycle():
await runCheckinCycle(pool, { tradingDate: date, model: SONNET_MODEL });
```

### 2. agents can't see timescale-specific data

**evidence:** `agents-ts/src/tools/sql-queries.ts`, function `getRecentTrades()` returns `entry_score_composite` as the only score column. the trades table has 12 score columns (`entry_score_1min`, `entry_score_5min`, `entry_score_hourly`, `exit_score_1min`, etc.) but none are exposed to agents.

the prompts explicitly ask for timescale analysis — `recommend_5min.md` says "analyze whether the 5-minute RSI is contributing positively" and `recommend_hourly.md` says "is the hourly hard gate too restrictive or too permissive?" — but agents have no tool to answer these questions.

**impact:** all 3 timescale-specific agents (1min, 5min, hourly) see identical data. their analyses overlap because they're all reasoning from composite score + trade outcomes. the timescale-specific framing in the prompts is aspirational but unsupported by the tool layer.

**fix:** add a `get_score_analysis` tool that returns per-timescale score statistics:

```sql
SELECT
  CASE WHEN pnl_dollars > 0 THEN 'winner' ELSE 'loser' END as outcome,
  avg(entry_score_1min) as avg_entry_1min,
  avg(entry_score_5min) as avg_entry_5min,
  avg(entry_score_hourly) as avg_entry_hourly,
  avg(entry_score_composite) as avg_entry_composite,
  percentile_cont(0.25) WITHIN GROUP (ORDER BY entry_score_composite) as p25_composite,
  percentile_cont(0.75) WITHIN GROUP (ORDER BY entry_score_composite) as p75_composite,
  count(*) as n
FROM trades
WHERE entry_fill_at >= now() - interval '3 days'
GROUP BY outcome
```

also update `getRecentTrades()` to include all 6 entry score columns and all 6 exit score columns.

### 3. scores were all zeros in the database

**evidence:** `SELECT count(*) FILTER (WHERE entry_score_composite = 0) FROM trades` → 509 (all trades). every agent memo from `agent_5min` begins with a "DATA QUALITY WARNING" about zero scores.

**root cause (two bugs):**
1. `crates/backtest/src/config_loader.rs` — `write_backtest_trades()` hardcoded all 12 score columns to `.bind(0.0_f64)` instead of reading from `TimescaleScores`
2. `crates/data_feed/src/trade_writer.rs` — paper trading path used wrong column names entirely (nonexistent columns like `entry_scores` JSONB instead of individual `entry_score_1min` etc.)

**status:** both fixed. `write_backtest_trades()` now takes a `trade_scores: &[(TimescaleScores, TimescaleScores)]` parameter. `TradeWriter.write_trade()` now unpacks scores into 12 individual bind parameters matching the actual schema. test coverage added in `crates/backtest/tests/replay_tests.rs` (`trade_scores_length_matches_trades`, `trade_scores_contain_nonzero_values`, `trade_scores_empty_when_no_trades`).

### 4. PM agent couldn't propose config changes due to enum bug

**evidence:** PM memos from cycles 3+ contain: `"propose_config_mutation returned error: type 'config_status' does not exist"`. all 10 evolution_cycles show `configs_proposed = 0, configs_promoted = 0, configs_rejected = 0`.

**root cause:** `agents-ts/src/tools/config-ops.ts` used `::config_status` in 5 SQL casts, but the postgres enum is named `mutation_status` (defined in `migrations/20260228000001_create_enums.sql`).

**impact:** the PM agent correctly diagnosed the trailing stop miscalibration (21% win rate), identified the first-30-minutes problem, and wanted to propose changes — but every `propose_config_mutation` call failed. this is why zero configs were proposed across 10 cycles despite clear evidence warranting changes.

**status:** fixed — all 5 occurrences of `::config_status` replaced with `::mutation_status` in config-ops.ts. test coverage added in `agents-ts/tests/tools/config-ops.test.ts` (source-level verification that SQL strings reference correct enum names).

### 5. recommendation agents put suggestions in free text

**evidence:** reading any recommendation memo from `agent_memos` — suggestions are embedded in the `reasoning` text field as natural language paragraphs. example from an `agent_5min` memo: "Suggestion 1: Raise entry_threshold from 0.45 to 0.50. Evidence: trades with composite below 0.50 had 35% win rate..."

the `agent_memos` table has no `suggestions` column. the PM agent must parse prose to extract `(param, old_value, new_value)` tuples.

**fix:** add a structured `suggestions` JSONB column to the `agent_memos` table and a corresponding field to the memo writer:

```json
{
  "suggestions": [
    {
      "target_tool_id": "trailing_stop_atr",
      "param": "multiplier",
      "current_value": 2.0,
      "proposed_value": 2.5,
      "evidence": "21% WR on 24 trailing stop exits vs 64% for other exits"
    }
  ]
}
```

the PM can then iterate over concrete suggestions rather than extracting them from paragraphs.

### 6. no feedback loop on past config changes

**evidence:** the `get_config_changelog` tool returns changelog entries (what changed) but there's no tool that correlates changes with outcomes. the PM agent sees "entry_threshold changed from X to Y on date Z" but can't query "what was win rate before vs after that change?"

during this run, the PM couldn't propose changes anyway (enum bug), but even if it had, subsequent cycles would have no way to evaluate whether the change helped.

**fix:** add a `get_change_impact` tool:

```sql
WITH change_point AS (
  SELECT promoted_at FROM config_versions WHERE id = $1
)
SELECT
  'before' as period,
  count(*) as trades,
  avg(pnl_dollars) as avg_pnl,
  count(*) FILTER (WHERE pnl_dollars > 0)::float / NULLIF(count(*), 0) as win_rate
FROM trades, change_point
WHERE entry_fill_at < change_point.promoted_at
  AND entry_fill_at >= change_point.promoted_at - interval '2 days'
UNION ALL
SELECT
  'after' as period,
  count(*) as trades,
  avg(pnl_dollars) as avg_pnl,
  count(*) FILTER (WHERE pnl_dollars > 0)::float / NULLIF(count(*), 0) as win_rate
FROM trades, change_point
WHERE entry_fill_at >= change_point.promoted_at
  AND entry_fill_at < change_point.promoted_at + interval '2 days'
```

### 7. the 3-agent-per-timescale split adds complexity without proportional value

**evidence:** comparing memos across agents within the same cycle — agent_1min, agent_5min, and agent_hourly all query `get_recent_trades` (same data), `get_daily_performance` (same data), and `get_performance_by_exit_reason` (same data). the only difference is their prompts frame the analysis around different timescales, but without per-timescale score data (issue 2), they arrive at overlapping conclusions.

sample from cycle 1: all 3 recommendation agents flagged the zero-score anomaly. all 3 noted session_close dominance. the PM then had to synthesize 3 memos that largely said the same thing.

**option A (conservative):** keep the 3-agent structure but fix the tools so each agent can actually see its timescale's data. add `get_score_analysis` filtered by timescale.

**option B (simpler):** consolidate to 1 check-in agent + 1 recommendation agent + 1 PM agent. each agent analyzes all timescales in a single pass, structured by section in the prompt. this cuts invocations from 7 to 3, reduces PM synthesis burden, and avoids the disagreement stalemate.

at ~$0.10-0.15 per agent invocation the cost difference is small. the real benefit of consolidation is simpler orchestration and more coherent analysis (one agent sees the full picture instead of three agents seeing slices).

### 8. "hold steady if agents disagree" is too conservative

**evidence:** `agent_pm.md` instructs the PM to hold steady when recommendation agents disagree. with 1 PM cycle per day (or 1 per simulated day in walk-forward), a disagreement means an entire day with no adaptation.

the PM agent's confidence scores across cycles ranged from 0.72–0.82 — consistently moderate. the recommendation agents' confidences ranged from 0.45–0.75. the PM could use these as signal weights rather than treating disagreement as a blocker.

**fix:** update `agent_pm.md` to use weighted confidence for conflict resolution:
- weight by timescale importance: 5-min agent carries 0.50 weight, 1-min carries 0.20, hourly carries 0.30
- if weighted-average confidence > 0.5 on a specific recommendation, act on it

### 9. budget headroom is massive and underutilized

**evidence:** per-cycle costs from `evolution_cycles`:
```
day 1: $0.82, day 2: $0.97, day 3: $0.92, day 4: $0.92, day 5: $1.09
day 6: $0.97, day 7: $0.94, day 8: $1.02, day 9: $0.95, day 10: $0.96
average: $0.96/cycle, total: $9.57 across 10 days
```

$0.96/day against a $5 cap = 19% utilization. options:
- run check-in cycles (which should be running anyway — see issue 1)
- add a mid-day PM cycle (e.g., 12:30 ET) for faster adaptation
- use opus for check-in agents on days where signal quality is flagged as degrading

### 10. PM auto-promotes without backtest in walk-forward mode

**evidence:** `agents-ts/src/orchestrator.ts` — when `tradingDate` is set (walk-forward mode), the `runFullPmCycle` function auto-promotes proposed configs without running backtest validation. the validation thresholds (min trades, min win rate 30%, max drawdown 15%, max sharpe degradation 0.5) defined in `agents-ts/src/tools/backtest-runner.ts` never ran.

in practice this didn't matter because the enum bug prevented any proposals. but when the bug is fixed, the PM will be able to promote configs without any validation gate in walk-forward mode.

**fix:** the backtest binary already supports `--date` mode. wire it into the walk-forward PM cycle:

1. PM proposes config → status = 'proposed'
2. run `cargo run -p backtest -- --date {nextDay} --lookback-days {lookback}` with proposed config
3. compare metrics against current config's backtest
4. promote or reject based on validation thresholds

---

## prioritized action items

| priority | item | effort | impact |
|----------|------|--------|--------|
| 1 | run check-in cycles in walk-forward | small | high — PM gets observation context |
| 2 | add `get_score_analysis` tool | medium | high — agents can see timescale data |
| 3 | add `get_change_impact` tool | medium | high — PM learns from past changes |
| 4 | add structured `suggestions` to memo schema | medium | medium — more reliable PM synthesis |
| 5 | consolidate to fewer agents (or fix tool gap) | medium | medium — simpler, more coherent |
| 6 | wire backtest validation into walk-forward | medium | medium — validation gates actually run |
| 7 | update PM conflict resolution to use weighted confidence | small | medium — faster adaptation |
| 8 | use budget headroom for additional cycles | small | low-medium — more observation data |

---

## bugs fixed during this analysis session

these bugs were discovered during the analysis and fixed before this document was written. they are listed here for completeness and so future agents understand what was broken.

### bug 1: `write_backtest_trades()` hardcoded scores to zero
- **file:** `crates/backtest/src/config_loader.rs`
- **symptom:** all 509 trades have `entry_score_composite = 0`, `entry_score_1min = 0`, etc.
- **cause:** the INSERT query used `.bind(0.0_f64)` for all 12 score columns
- **fix:** function now accepts `trade_scores: &[(TimescaleScores, TimescaleScores)]` and binds real values
- **tests:** `trade_scores_length_matches_trades`, `trade_scores_contain_nonzero_values`, `trade_scores_empty_when_no_trades` in `crates/backtest/tests/replay_tests.rs`

### bug 2: `TradeWriter.write_trade()` used wrong schema
- **file:** `crates/data_feed/src/trade_writer.rs`
- **symptom:** paper trading writes would fail with SQL errors (wrong column names)
- **cause:** INSERT referenced nonexistent columns (`entry_scores` JSONB, `trade_id`, etc.) instead of the actual schema (12 individual score columns, `id` RETURNING)
- **fix:** complete rewrite of the INSERT with correct column names, `exit_reason_to_str()` for snake_case enum values, and individual score bindings
- **tests:** `exit_reason_str_mapping` (all 8 variants) in `crates/data_feed/src/trade_writer.rs`

### bug 3: `config-ops.ts` used wrong enum name
- **file:** `agents-ts/src/tools/config-ops.ts`
- **symptom:** PM agent memo reads "type 'config_status' does not exist"
- **cause:** 5 SQL casts used `::config_status` but the postgres enum is `mutation_status`
- **fix:** replaced all 5 occurrences with `::mutation_status`
- **tests:** source-level enum name verification in `agents-ts/tests/tools/config-ops.test.ts`

### manual config changes applied
- **file:** `configs/v3_post_analysis.json`, promoted via `agents-ts/src/promote-config.ts`
- **changes:** ATR trailing stop multiplier 2.0→2.5, avoid_first_minutes 5→35, max_hold_timeout 2700000→5400000ms
- **evidence:** trailing stop had 21% win rate on 24 trades (-$1,732); 9 AM entries were net negative (-$431, 39% WR); max_hold_timeout had 88.5% win rate at exactly 45min suggesting winners were being cut
- **verification:** `SELECT * FROM config_changelog ORDER BY id` shows 3 entries for config_version_id=3
- **verification:** `SELECT id, status, mutation_reason FROM config_versions WHERE id=3` shows status='promoted'

---

## key data sources for future analysis

| data | table/location | what it tells you |
|------|---------------|-------------------|
| trade outcomes | `trades` (509 rows) | P&L, exit reasons, hold durations, scores (zeros until next run) |
| agent observations | `agent_memos` (40 rows) | what agents saw and recommended each cycle |
| config history | `config_versions` (3 rows) | seed → v1 default → v3 post-analysis |
| change audit trail | `config_changelog` (3 rows) | atomic parameter changes with old/new values |
| cycle metadata | `evolution_cycles` (10 rows) | per-day cost, tokens, proposal outcomes |
| budget tracking | `daily_budget` | per-day spend against cap |
| tool source code | `agents-ts/src/tools/sql-queries.ts` | what data agents can actually query |
| agent prompts | `agents-ts/prompts/` | what agents are instructed to analyze |
| walk-forward driver | `agents-ts/src/walk-forward.ts` | how simulation cycles are orchestrated |
| orchestrator | `agents-ts/src/orchestrator.ts` | live scheduling, cycle execution, budget enforcement |
