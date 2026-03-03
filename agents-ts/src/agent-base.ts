/**
 * shared agent invocation logic.
 *
 * provides a base for all agents with common functionality:
 * - claude agent sdk integration via in-process MCP tools
 * - token/cost tracking per invocation
 * - structured output parsing
 */

import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type pg from "pg";
import {
  query,
  createSdkMcpServer,
  tool,
  type Options,
  type SDKResultMessage,
  type SdkMcpToolDefinition,
} from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod/v4";
import type { CallToolResult } from "@modelcontextprotocol/sdk/types.js";

// the SDK uses `any` for the generic tool array type
/* eslint-disable @typescript-eslint/no-explicit-any */

import {
  type AgentType,
  type BeliefStatus,
  type Suggestion,
  type TokenUsage,
  createAgentMemo,
  createBelief,
  createTokenUsage,
} from "./models.js";
import { logger } from "./logger.js";
import { writeMemo } from "./tools/memo-writer.js";
import {
  getRecentTrades,
  getDailyPerformance,
  getConfigChangelog,
  getPerformanceByExitReason,
  getCheckinMemosSinceLastPm,
  getAnalysisMemos,
  getActiveBeliefs,
  writeBelief,
} from "./tools/sql-queries.js";
import { getCurrentConfig, proposeConfig } from "./tools/config-ops.js";
import { readLogs } from "./tools/log-reader.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROMPTS_DIR = join(__dirname, "..", "prompts");

// model ids
export const HAIKU_MODEL = "claude-haiku-4-5-20251001";
export const SONNET_MODEL = "claude-sonnet-4-6";
export const OPUS_MODEL = "claude-opus-4-6";

// --- tool set definitions ---
// these constants map tool names to tool set membership

export const ANALYSIS_TOOL_NAMES = [
  "get_recent_trades",
  "get_daily_performance",
  "get_current_config",
  "get_config_changelog",
  "get_performance_by_exit_reason",
  "get_beliefs",
  "write_analysis_memo",
] as const;

export const PM_TOOL_NAMES = [
  "get_recent_trades",
  "get_daily_performance",
  "get_current_config",
  "get_config_changelog",
  "get_performance_by_exit_reason",
  "get_prior_memos",
  "get_analysis_memos",
  "get_beliefs",
  "propose_config_mutation",
  "write_pm_memo",
  "write_belief",
  "read_logs",
] as const;

export type ToolSetName = "analysis" | "pm";

export function loadPrompt(promptName: string): string {
  const promptPath = join(PROMPTS_DIR, `${promptName}.md`);
  try {
    return readFileSync(promptPath, "utf-8");
  } catch {
    throw new Error(`prompt not found: ${promptPath}`);
  }
}

// --- helper to make a text CallToolResult ---
function textResult(text: string): CallToolResult {
  return { content: [{ type: "text", text }] };
}

// --- tool builder ---
// builds MCP tool definitions with inline handlers that close over pool/cycleId

export function buildTools(
  pool: pg.Pool,
  cycleId: number,
  agentType: AgentType,
  toolSet: ToolSetName,
  capture?: { proposed_version_id: number | null },
): Array<SdkMcpToolDefinition<any>> {
  const tools: Array<SdkMcpToolDefinition<any>> = [];

  const toolNamesMap: Record<ToolSetName, readonly string[]> = {
    analysis: ANALYSIS_TOOL_NAMES,
    pm: PM_TOOL_NAMES,
  };
  const toolNames = toolNamesMap[toolSet];

  const nameSet = new Set<string>(toolNames);

  // --- read-only tools ---

  if (nameSet.has("get_recent_trades")) {
    tools.push(
      tool(
        "get_recent_trades",
        "Fetch recent completed trades. Returns trade records with entry/exit prices, P&L, hold duration, and exit reason.",
        {
          ticker: z.string().optional().describe("Filter by ticker symbol (e.g., 'SPY'). Omit for all tickers."),
          limit: z.number().optional().default(20).describe("Maximum number of trades to return. Default: 20."),
        },
        async (args) => {
          const trades = await getRecentTrades(pool, args.ticker, args.limit);
          return textResult(JSON.stringify(trades));
        },
      ),
    );
  }

  if (nameSet.has("get_daily_performance")) {
    tools.push(
      tool(
        "get_daily_performance",
        "Fetch daily performance summary with total trades, win rate, total P&L, and average hold time.",
        {
          ticker: z.string().optional().describe("Filter by ticker symbol. Omit for all tickers."),
        },
        async (args) => {
          const perf = await getDailyPerformance(pool, undefined, args.ticker);
          return textResult(JSON.stringify(perf));
        },
      ),
    );
  }

  if (nameSet.has("get_current_config")) {
    tools.push(
      tool(
        "get_current_config",
        "Read the currently active (promoted) strategy configuration.",
        {},
        async () => {
          const config = await getCurrentConfig(pool);
          return textResult(
            config
              ? JSON.stringify(config)
              : JSON.stringify({ error: "no promoted config found" }),
          );
        },
      ),
    );
  }

  if (nameSet.has("get_config_changelog")) {
    tools.push(
      tool(
        "get_config_changelog",
        "Fetch recent config changes. Shows what parameters were modified and why.",
        {
          limit: z.number().optional().default(20).describe("Maximum changelog entries to return. Default: 20."),
        },
        async (args) => {
          const changelog = await getConfigChangelog(pool, undefined, args.limit);
          return textResult(JSON.stringify(changelog));
        },
      ),
    );
  }

  if (nameSet.has("get_performance_by_exit_reason")) {
    tools.push(
      tool(
        "get_performance_by_exit_reason",
        "Fetch trade performance broken down by exit reason (trailing stop, hard stop, session close, etc.).",
        {},
        async () => {
          const perf = await getPerformanceByExitReason(pool);
          return textResult(JSON.stringify(perf));
        },
      ),
    );
  }

  // --- write tools (memo types) ---

  if (nameSet.has("write_analysis_memo")) {
    tools.push(
      tool(
        "write_analysis_memo",
        "Write your structured analysis memo with evidence-backed parameter suggestions. Call this once you have completed your full analysis.",
        {
          confidence_score: z.number().describe("Your confidence in this analysis, 0.0 to 1.0."),
          volatility_regime: z.enum(["low", "normal", "high", "extreme"]).describe("Current volatility regime assessment."),
          directional_bias: z.enum(["strong_long", "lean_long", "neutral", "lean_short", "strong_short"]).describe("Current directional bias assessment."),
          signal_quality: z.enum(["strong", "moderate", "weak", "conflicting"]).describe("Quality of current trading signals."),
          flags: z.record(z.string(), z.unknown()).optional().describe("Boolean flags for notable patterns."),
          reasoning: z.string().describe("Your detailed analysis covering all timescales with specific parameter change suggestions."),
          suggestions: z.array(z.object({
            target_tool_id: z.string().describe("The instance_id of the indicator/action to change."),
            param: z.string().nullable().describe("The specific parameter to change, or null for tool-level changes."),
            current_value: z.unknown().describe("The current value of the parameter."),
            proposed_value: z.unknown().describe("The proposed new value."),
            confidence: z.number().describe("Confidence in this suggestion, 0.0 to 1.0."),
            evidence_summary: z.string().describe("Brief evidence supporting this change."),
          })).optional().describe("Structured parameter change suggestions (max 5)."),
          trades_reviewed: z.number().optional().describe("Number of trades you reviewed."),
          period_win_rate: z.number().optional().describe("Win rate for the period you reviewed."),
          period_pnl: z.number().optional().describe("Total P&L for the period you reviewed."),
        },
        async (args) => {
          const suggestions: Suggestion[] = (args.suggestions ?? []).map((s: any) => ({
            target_tool_id: s.target_tool_id,
            param: s.param ?? null,
            current_value: s.current_value,
            proposed_value: s.proposed_value,
            confidence: s.confidence,
            evidence_summary: s.evidence_summary,
          }));
          const memo = createAgentMemo({
            agent: agentType,
            evolution_cycle_id: cycleId,
            memo_type: "analysis",
            confidence_score: args.confidence_score,
            volatility_regime: args.volatility_regime,
            directional_bias: args.directional_bias,
            signal_quality: args.signal_quality,
            flags: args.flags ?? {},
            reasoning: args.reasoning,
            suggestions,
            trades_reviewed: args.trades_reviewed ?? null,
            period_win_rate: args.period_win_rate ?? null,
            period_pnl: args.period_pnl ?? null,
          });
          const memoId = await writeMemo(pool, memo);
          return textResult(JSON.stringify({ status: "success", memo_id: memoId }));
        },
      ),
    );
  }

  // --- PM-specific tools ---

  if (nameSet.has("get_prior_memos")) {
    tools.push(
      tool(
        "get_prior_memos",
        "Fetch analysis memos from earlier today (mid-day analysis). Excludes the current cycle's analysis memo.",
        {},
        async () => {
          const memos = await getCheckinMemosSinceLastPm(pool, cycleId);
          return textResult(JSON.stringify(memos));
        },
      ),
    );
  }

  if (nameSet.has("get_analysis_memos")) {
    tools.push(
      tool(
        "get_analysis_memos",
        "Fetch analysis memos from this PM cycle's analysis agent.",
        {},
        async () => {
          const memos = await getAnalysisMemos(pool, cycleId);
          return textResult(JSON.stringify(memos));
        },
      ),
    );
  }

  if (nameSet.has("propose_config_mutation")) {
    tools.push(
      tool(
        "propose_config_mutation",
        "Propose a new strategy config. Provide a complete config blob with your changes.",
        {
          config_blob: z.record(z.string(), z.unknown()).describe("The complete new strategy config blob."),
          mutation_reason: z.string().describe("Explanation of what you changed and why."),
        },
        async (args) => {
          const current = await getCurrentConfig(pool);
          const parentId = current
            ? (current.config_version_id as number)
            : null;
          const versionId = await proposeConfig(
            pool,
            args.config_blob,
            parentId,
            args.mutation_reason,
            agentType,
          );
          // capture proposed version id for orchestrator
          if (capture) {
            capture.proposed_version_id = versionId;
          }
          return textResult(JSON.stringify({ status: "success", version_id: versionId }));
        },
      ),
    );
  }

  if (nameSet.has("write_pm_memo")) {
    tools.push(
      tool(
        "write_pm_memo",
        "Write a PM decision memo explaining your analysis and decision.",
        {
          confidence_score: z.number().describe("Your confidence in this decision, 0.0 to 1.0."),
          reasoning: z.string().describe("Your detailed analysis, evidence reviewed, and decision rationale."),
          trades_reviewed: z.number().optional().describe("Number of trades you reviewed."),
          period_win_rate: z.number().optional().describe("Win rate for the period you reviewed."),
          period_pnl: z.number().optional().describe("Total P&L for the period you reviewed."),
        },
        async (args) => {
          const memo = createAgentMemo({
            agent: agentType,
            evolution_cycle_id: cycleId,
            memo_type: "recommendation",
            confidence_score: args.confidence_score,
            reasoning: args.reasoning,
            trades_reviewed: args.trades_reviewed ?? null,
            period_win_rate: args.period_win_rate ?? null,
            period_pnl: args.period_pnl ?? null,
          });
          const memoId = await writeMemo(pool, memo);
          return textResult(JSON.stringify({ status: "success", memo_id: memoId }));
        },
      ),
    );
  }

  if (nameSet.has("get_beliefs")) {
    tools.push(
      tool(
        "get_beliefs",
        "Fetch active investment beliefs accumulated from past analysis cycles. These represent confirmed patterns and principles.",
        {
          category: z.string().optional().describe("Filter by category (e.g., 'entry_timing', 'risk_management', 'indicator_tuning'). Omit for all."),
        },
        async (args) => {
          const beliefs = await getActiveBeliefs(pool, args.category);
          return textResult(JSON.stringify(beliefs));
        },
      ),
    );
  }

  if (nameSet.has("write_belief")) {
    tools.push(
      tool(
        "write_belief",
        "Record a new investment belief based on accumulated evidence. Beliefs persist across cycles and inform future analysis.",
        {
          belief_text: z.string().describe("The investment belief or principle, stated clearly and concisely."),
          confidence: z.number().describe("Confidence in this belief, 0.0 to 1.0."),
          category: z.string().describe("Category for grouping (e.g., 'entry_timing', 'risk_management', 'indicator_tuning', 'market_regime', 'exit_strategy')."),
          evidence_summary: z.string().describe("Brief summary of evidence supporting this belief."),
          source_memo_ids: z.array(z.number()).optional().describe("IDs of memos that support this belief."),
        },
        async (args) => {
          const belief = createBelief({
            belief_text: args.belief_text,
            confidence: args.confidence,
            category: args.category,
            source_memo_ids: args.source_memo_ids ?? [],
          });
          const beliefId = await writeBelief(pool, belief);
          return textResult(JSON.stringify({ status: "success", belief_id: beliefId }));
        },
      ),
    );
  }

  if (nameSet.has("read_logs")) {
    tools.push(
      tool(
        "read_logs",
        "Read system logs from paper_trader, backtest, or agents. Use to review execution details, indicator scores, trade entries/exits, and errors.",
        {
          date: z.string().describe("Date in YYYY-MM-DD format."),
          source: z.enum(["paper_trader", "backtest", "agents"]).describe("Log source to read."),
          ticker: z.string().optional().describe("Filter lines containing this ticker symbol."),
          level: z.string().optional().describe("Filter by log level: INFO, WARN, or ERROR."),
          tail: z.number().optional().default(200).describe("Return last N matching lines. Default: 200."),
        },
        async (args) => {
          return textResult(readLogs(args));
        },
      ),
    );
  }

  return tools;
}

// --- agent runner ---

async function runAgentSdk(
  agentType: AgentType,
  promptName: string,
  pool: pg.Pool,
  cycleId: number,
  toolSet: ToolSetName,
  initialMessage: string,
  model: string = SONNET_MODEL,
  maxTurns = 10,
  capture?: { proposed_version_id: number | null },
): Promise<TokenUsage> {
  const systemPrompt = loadPrompt(promptName);
  const totalUsage = createTokenUsage({ model });

  const tools = buildTools(pool, cycleId, agentType, toolSet, capture);
  const server = createSdkMcpServer({ name: "trading", version: "1.0.0", tools });

  const toolNamesForAllowed: Record<ToolSetName, readonly string[]> = {
    analysis: ANALYSIS_TOOL_NAMES,
    pm: PM_TOOL_NAMES,
  };
  const allowedToolNames = toolNamesForAllowed[toolSet];

  const allowedTools = allowedToolNames.map((n) => `mcp__trading__${n}`);

  const options: Options = {
    model,
    systemPrompt,
    mcpServers: { trading: server },
    allowedTools: [...allowedTools],
    permissionMode: "bypassPermissions",
    allowDangerouslySkipPermissions: true,
    maxTurns,
    tools: [],
  };

  for await (const msg of query({ prompt: initialMessage, options })) {
    if (msg.type === "result") {
      const result = msg as SDKResultMessage;
      // use modelUsage for accurate totals (includes cached tokens)
      if (result.modelUsage) {
        for (const mu of Object.values(result.modelUsage)) {
          totalUsage.input_tokens +=
            mu.inputTokens + mu.cacheReadInputTokens + mu.cacheCreationInputTokens;
          totalUsage.output_tokens += mu.outputTokens;
        }
      } else {
        totalUsage.input_tokens += result.usage.input_tokens;
        totalUsage.output_tokens += result.usage.output_tokens;
      }
    }
  }

  logger.info(
    `agent ${agentType}: ${totalUsage.input_tokens} input tokens, ` +
      `${totalUsage.output_tokens} output tokens`,
  );

  return totalUsage;
}

export async function runAnalysisAgent(
  pool: pg.Pool,
  cycleId: number,
  model: string = OPUS_MODEL,
  maxTurns = 15,
): Promise<TokenUsage> {
  return runAgentSdk(
    "agent_analysis",
    "analysis",
    pool,
    cycleId,
    "analysis",
    "Run your full analysis now. Examine all timescales (1-min, 5-min, hourly), " +
      "gather trade data and performance metrics, then write your analysis memo " +
      "with structured suggestions.",
    model,
    maxTurns,
  );
}

export async function runPmAgent(
  pool: pg.Pool,
  cycleId: number,
  model: string = OPUS_MODEL,
  maxTurns = 15,
): Promise<[TokenUsage, number | null]> {
  const capture = { proposed_version_id: null as number | null };

  const usage = await runAgentSdk(
    "agent_pm",
    "agent_pm",
    pool,
    cycleId,
    "pm",
    "Run your PM analysis now. Review the analysis memo from this cycle's " +
      "analysis agent and recent check-in observations. Analyze trade performance " +
      "and decide whether to propose config changes or hold steady.",
    model,
    maxTurns,
    capture,
  );

  logger.info(`PM agent: proposed_version_id=${capture.proposed_version_id}`);

  return [usage, capture.proposed_version_id];
}
