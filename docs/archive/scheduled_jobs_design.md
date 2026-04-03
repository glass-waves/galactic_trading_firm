# scheduled claude jobs — design sketch

replaces the agents-ts orchestrator + adds system watchdog using claude cloud scheduled tasks.

---

## current architecture (agents-ts)

```
orchestrator.ts → cron 12:30 ET → runAnalysisAgent(sonnet) → write memo
                → cron 15:30 ET → runAnalysisAgent(opus) + runPmAgent(opus) → propose/promote config
```

- 2 typescript agents with custom tool wrappers
- ~1500 lines of orchestration code (orchestrator.ts, agent-base.ts, tools/*.ts)
- budget tracking, token cost accounting
- requires `agents-ts` docker service, node_modules, anthropic SDK

## proposed architecture (claude scheduled jobs)

three cloud scheduled tasks. all use postgres MCP server for DB access.

---

### job 1: mid-day analysis

**schedule:** `30 12 * * 1-5` (12:30 ET, weekdays)
**model:** sonnet 4.6
**MCP servers:** postgres
**network:** full (for any market data lookups)

**prompt:**
```
you are the mid-day analysis agent for an intraday trading system.

connect to the postgres database and:

1. query today's trades so far:
   SELECT * FROM trades WHERE DATE(exit_time AT TIME ZONE 'US/Eastern') = CURRENT_DATE ORDER BY exit_time

2. query daily performance:
   SELECT * FROM daily_performance WHERE trading_date = CURRENT_DATE

3. query the current promoted config:
   SELECT config_blob FROM config_versions WHERE status = 'promoted' ORDER BY id DESC LIMIT 1

4. query recent beliefs:
   SELECT * FROM beliefs WHERE status = 'active' ORDER BY created_at DESC LIMIT 10

analyze:
- how many trades, win rate, P&L so far today
- which tickers are performing, which are lagging
- exit reason distribution (are ScoreExits dominating? that's a warning)
- any unusual patterns vs the strategy's expected behavior

write a structured analysis memo to the database:
INSERT INTO agent_memos (agent, memo_type, content, confidence, trading_date, ...)

keep it concise. observation only — no config authority. flag anything that
warrants PM attention at end of day.
```

---

### job 2: end-of-day PM cycle

**schedule:** `30 15 * * 1-5` (15:30 ET, weekdays)
**model:** opus 4.6
**MCP servers:** postgres
**network:** full (needs to run backtest binary for validation)

**prompt:**
```
you are the PM agent for an intraday trading system. you have full config authority.

connect to the postgres database and:

1. read today's mid-day analysis memo:
   SELECT * FROM agent_memos WHERE memo_type = 'analysis'
   AND trading_date = CURRENT_DATE ORDER BY created_at DESC LIMIT 1

2. read recent trade performance (last 5 trading days):
   SELECT * FROM daily_performance ORDER BY trading_date DESC LIMIT 5

3. read performance by exit reason:
   SELECT * FROM performance_by_exit_reason

4. read active beliefs:
   SELECT * FROM beliefs WHERE status = 'active'

5. read the current promoted config:
   SELECT id, config_blob FROM config_versions WHERE status = 'promoted'
   ORDER BY id DESC LIMIT 1

evaluate whether any config changes are warranted. your decision framework:
- "hold steady" is the default. only change if evidence is strong.
- any proposed change must be validated via backtest before promotion.
- consider the mid-day analysis memo and accumulated beliefs.
- small, targeted changes only. one parameter at a time.

if proposing a change:
1. INSERT the new config as status='proposed' into config_versions
2. run backtest validation (shell command)
3. if backtest passes (PF > current, no year negative), UPDATE status to 'promoted'
4. INSERT changelog entry explaining the change

if holding steady:
- write a PM memo explaining why no changes were warranted

always write or update beliefs based on today's observations (CVRF framework).
```

---

### job 3: system watchdog

**schedule:** `*/5 9-16 * * 1-5` (every 5 min, 9am-4pm ET, weekdays)
**model:** haiku 4.5 (cheapest, fastest — this is a health check, not analysis)
**MCP servers:** postgres
**network:** full (needs broker API access for emergency liquidation)

**prompt:**
```
you are the system watchdog for an intraday trading system.
this runs every 5 minutes during market hours. be fast and concise.

check the following (query postgres):

1. engine heartbeat: is the paper_trader process writing recent data?
   SELECT MAX(exit_time) FROM trades WHERE exit_time > NOW() - INTERVAL '30 minutes'
   — if no recent activity AND it's between 9:45am-3:30pm ET, flag as WARNING

2. position check: are there any stuck positions?
   — the engine is intraday-only. no position should be held past 4:00pm ET.
   — if a position exists after 4pm, this is CRITICAL.

3. daily P&L check:
   SELECT SUM(pnl) FROM trades WHERE DATE(exit_time AT TIME ZONE 'US/Eastern') = CURRENT_DATE
   — if daily loss exceeds 3% of capital, flag as WARNING
   — if daily loss exceeds 5% of capital, flag as CRITICAL

4. trade count anomaly:
   — if today's trade count is 3x the daily average, flag as WARNING (possible churn)

responses:
- if all OK: write nothing (save tokens)
- if WARNING: write a brief note to agent_memos with memo_type='watchdog_warning'
- if CRITICAL:
  1. write to agent_memos with memo_type='watchdog_critical'
  2. attempt emergency position liquidation via broker API:
     POST https://paper-api.alpaca.markets/v2/positions (DELETE to close all)
     with headers: APCA-API-KEY-ID, APCA-API-SECRET-KEY
  3. flag for human review
```

---

## rough edges and mitigations

| issue | impact | mitigation |
|-------|--------|-----------|
| cloud tasks have 1hr minimum interval | watchdog needs 5-min | use desktop task or /loop for watchdog; cloud for analysis/PM |
| no inter-run conversation memory | each run is fresh | all state in postgres. prompt includes full context. |
| cloud tasks get fresh repo clone | startup overhead | keep repo lean. all config in DB, not files. |
| jitter up to 15 min on cloud tasks | PM cycle might run at 3:35 instead of 3:30 | acceptable — market doesn't close until 4:00 |
| no guaranteed ordering between jobs | PM might run before analysis finishes | PM prompt checks for today's analysis memo; if missing, runs its own analysis first |
| no native alerting | critical issues need human attention | watchdog writes to slack via MCP connector or webhook |

## cost estimate

| job | model | frequency | est tokens/run | daily cost |
|-----|-------|-----------|---------------|-----------|
| mid-day analysis | sonnet | 1x/day | ~5k in + ~2k out | ~$0.05 |
| PM cycle | opus | 1x/day | ~10k in + ~5k out | ~$0.18 |
| watchdog | haiku | ~84x/day (5min × 7hrs) | ~1k in + ~100 out | ~$0.09 |
| **total** | | | | **~$0.32/day** |

current agents-ts budget: $5/day. this is 15x cheaper.

## migration path

1. set up postgres MCP server connection in claude cloud environment
2. create watchdog job first (lowest risk, highest value — safety net)
3. create mid-day analysis job, compare output quality vs agents-ts for 1 week
4. create PM cycle job, run in "dry run" mode (propose but don't promote) for 1 week
5. once validated, disable agents-ts docker service
6. remove agents-ts from docker-compose.yml

## what stays the same

- rust engine (paper_trader) — unchanged, still runs as docker service
- postgres — unchanged, still the state store
- config hot-reload — unchanged, ConfigWatcher polls for promoted configs
- all the backtest infrastructure — unchanged

## what gets removed

- `agents-ts/` directory (~1500 lines of typescript)
- `agents-ts` docker service
- anthropic SDK dependency
- custom tool definitions (sql-queries.ts, config-ops.ts, memo-writer.ts, etc.)
- budget tracking code
- agent-base.ts prompt loading
