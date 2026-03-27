/**
 * shared agent invocation logic.
 *
 * provides a base for all agents with common functionality:
 * - anthropic API integration with tool use
 * - token/cost tracking per invocation
 * - structured output parsing
 */

import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type pg from "pg";
import Anthropic from "@anthropic-ai/sdk";
import type {
  MessageParam,
  ToolResultBlockParam,
  ToolUseBlock,
  Tool,
} from "@anthropic-ai/sdk/resources/messages";

import {
  type AgentType,
  type VolatilityRegime,
  type DirectionalBias,
  type SignalQuality,
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
import { type ReadLogsParams, readLogs } from "./tools/log-reader.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROMPTS_DIR = join(__dirname, "..", "prompts");

// model ids
export const HAIKU_MODEL = "claude-haiku-4-5-20251001";
export const SONNET_MODEL = "claude-sonnet-4-6";
export const OPUS_MODEL = "claude-opus-4-6";

// --- tool definition type ---

export interface ToolDefinition {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
  handler: (args: Record<string, unknown>) => Promise<string>;
}

// --- tool set definitions ---

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

// --- tool builder ---

export function buildTools(
  pool: pg.Pool,
  cycleId: number,
  agentType: AgentType,
  toolSet: ToolSetName,
  capture?: { proposed_version_id: number | null },
): ToolDefinition[] {
  const tools: ToolDefinition[] = [];

  const toolNamesMap: Record<ToolSetName, readonly string[]> = {
    analysis: ANALYSIS_TOOL_NAMES,
    pm: PM_TOOL_NAMES,
  };
  const toolNames = toolNamesMap[toolSet];
  const nameSet = new Set<string>(toolNames);

  // --- read-only tools ---

  if (nameSet.has("get_recent_trades")) {
    tools.push({
      name: "get_recent_trades",
      description: "Fetch recent completed trades. Returns trade records with entry/exit prices, P&L, hold duration, and exit reason.",
      input_schema: {
        type: "object",
        properties: {
          ticker: { type: "string", description: "Filter by ticker symbol (e.g., 'SPY'). Omit for all tickers." },
          limit: { type: "number", description: "Maximum number of trades to return. Default: 20." },
        },
        required: [],
      },
      handler: async (args) => {
        const trades = await getRecentTrades(pool, args.ticker as string | undefined, args.limit as number | undefined);
        return JSON.stringify(trades);
      },
    });
  }

  if (nameSet.has("get_daily_performance")) {
    tools.push({
      name: "get_daily_performance",
      description: "Fetch daily performance summary with total trades, win rate, total P&L, and average hold time.",
      input_schema: {
        type: "object",
        properties: {
          ticker: { type: "string", description: "Filter by ticker symbol. Omit for all tickers." },
        },
        required: [],
      },
      handler: async (args) => {
        const perf = await getDailyPerformance(pool, undefined, args.ticker as string | undefined);
        return JSON.stringify(perf);
      },
    });
  }

  if (nameSet.has("get_current_config")) {
    tools.push({
      name: "get_current_config",
      description: "Read the currently active (promoted) strategy configuration.",
      input_schema: {
        type: "object",
        properties: {},
        required: [],
      },
      handler: async () => {
        const config = await getCurrentConfig(pool);
        return config
          ? JSON.stringify(config)
          : JSON.stringify({ error: "no promoted config found" });
      },
    });
  }

  if (nameSet.has("get_config_changelog")) {
    tools.push({
      name: "get_config_changelog",
      description: "Fetch recent config changes. Shows what parameters were modified and why.",
      input_schema: {
        type: "object",
        properties: {
          limit: { type: "number", description: "Maximum changelog entries to return. Default: 20." },
        },
        required: [],
      },
      handler: async (args) => {
        const changelog = await getConfigChangelog(pool, undefined, args.limit as number | undefined);
        return JSON.stringify(changelog);
      },
    });
  }

  if (nameSet.has("get_performance_by_exit_reason")) {
    tools.push({
      name: "get_performance_by_exit_reason",
      description: "Fetch trade performance broken down by exit reason (trailing stop, hard stop, session close, etc.).",
      input_schema: {
        type: "object",
        properties: {},
        required: [],
      },
      handler: async () => {
        const perf = await getPerformanceByExitReason(pool);
        return JSON.stringify(perf);
      },
    });
  }

  // --- write tools (memo types) ---

  if (nameSet.has("write_analysis_memo")) {
    tools.push({
      name: "write_analysis_memo",
      description: "Write your structured analysis memo with evidence-backed parameter suggestions. Call this once you have completed your full analysis.",
      input_schema: {
        type: "object",
        properties: {
          confidence_score: { type: "number", description: "Your confidence in this analysis, 0.0 to 1.0." },
          volatility_regime: { type: "string", enum: ["low", "normal", "high", "extreme"], description: "Current volatility regime assessment." },
          directional_bias: { type: "string", enum: ["strong_long", "lean_long", "neutral", "lean_short", "strong_short"], description: "Current directional bias assessment." },
          signal_quality: { type: "string", enum: ["strong", "moderate", "weak", "conflicting"], description: "Quality of current trading signals." },
          flags: { type: "object", description: "Boolean flags for notable patterns." },
          reasoning: { type: "string", description: "Your detailed analysis covering all timescales with specific parameter change suggestions." },
          suggestions: {
            type: "array",
            items: {
              type: "object",
              properties: {
                target_tool_id: { type: "string", description: "The instance_id of the indicator/action to change." },
                param: { type: ["string", "null"], description: "The specific parameter to change, or null for tool-level changes." },
                current_value: { description: "The current value of the parameter." },
                proposed_value: { description: "The proposed new value." },
                confidence: { type: "number", description: "Confidence in this suggestion, 0.0 to 1.0." },
                evidence_summary: { type: "string", description: "Brief evidence supporting this change." },
              },
              required: ["target_tool_id", "param", "current_value", "proposed_value", "confidence", "evidence_summary"],
            },
            description: "Structured parameter change suggestions (max 5).",
          },
          trades_reviewed: { type: "number", description: "Number of trades you reviewed." },
          period_win_rate: { type: "number", description: "Win rate for the period you reviewed." },
          period_pnl: { type: "number", description: "Total P&L for the period you reviewed." },
        },
        required: ["confidence_score", "volatility_regime", "directional_bias", "signal_quality", "reasoning"],
      },
      handler: async (args) => {
        const rawSuggestions = (args.suggestions ?? []) as Array<Record<string, unknown>>;
        const suggestions: Suggestion[] = rawSuggestions.map((s) => ({
          target_tool_id: s.target_tool_id as string,
          param: (s.param as string) ?? null,
          current_value: s.current_value,
          proposed_value: s.proposed_value,
          confidence: s.confidence as number,
          evidence_summary: s.evidence_summary as string,
        }));
        const memo = createAgentMemo({
          agent: agentType,
          evolution_cycle_id: cycleId,
          memo_type: "analysis",
          confidence_score: args.confidence_score as number,
          volatility_regime: args.volatility_regime as VolatilityRegime,
          directional_bias: args.directional_bias as DirectionalBias,
          signal_quality: args.signal_quality as SignalQuality,
          flags: (args.flags as Record<string, unknown>) ?? {},
          reasoning: args.reasoning as string,
          suggestions,
          trades_reviewed: (args.trades_reviewed as number) ?? null,
          period_win_rate: (args.period_win_rate as number) ?? null,
          period_pnl: (args.period_pnl as number) ?? null,
        });
        const memoId = await writeMemo(pool, memo);
        return JSON.stringify({ status: "success", memo_id: memoId });
      },
    });
  }

  // --- PM-specific tools ---

  if (nameSet.has("get_prior_memos")) {
    tools.push({
      name: "get_prior_memos",
      description: "Fetch analysis memos from earlier today (mid-day analysis). Excludes the current cycle's analysis memo.",
      input_schema: {
        type: "object",
        properties: {},
        required: [],
      },
      handler: async () => {
        const memos = await getCheckinMemosSinceLastPm(pool, cycleId);
        return JSON.stringify(memos);
      },
    });
  }

  if (nameSet.has("get_analysis_memos")) {
    tools.push({
      name: "get_analysis_memos",
      description: "Fetch analysis memos from this PM cycle's analysis agent.",
      input_schema: {
        type: "object",
        properties: {},
        required: [],
      },
      handler: async () => {
        const memos = await getAnalysisMemos(pool, cycleId);
        return JSON.stringify(memos);
      },
    });
  }

  if (nameSet.has("propose_config_mutation")) {
    tools.push({
      name: "propose_config_mutation",
      description: "Propose a new strategy config. Provide a complete config blob with your changes.",
      input_schema: {
        type: "object",
        properties: {
          config_blob: { type: "object", description: "The complete new strategy config blob." },
          mutation_reason: { type: "string", description: "Explanation of what you changed and why." },
        },
        required: ["config_blob", "mutation_reason"],
      },
      handler: async (args) => {
        const current = await getCurrentConfig(pool);
        const parentId = current
          ? (current.config_version_id as number)
          : null;
        const versionId = await proposeConfig(
          pool,
          args.config_blob as Record<string, unknown>,
          parentId,
          args.mutation_reason as string,
          agentType,
        );
        if (capture) {
          capture.proposed_version_id = versionId;
        }
        return JSON.stringify({ status: "success", version_id: versionId });
      },
    });
  }

  if (nameSet.has("write_pm_memo")) {
    tools.push({
      name: "write_pm_memo",
      description: "Write a PM decision memo explaining your analysis and decision.",
      input_schema: {
        type: "object",
        properties: {
          confidence_score: { type: "number", description: "Your confidence in this decision, 0.0 to 1.0." },
          reasoning: { type: "string", description: "Your detailed analysis, evidence reviewed, and decision rationale." },
          trades_reviewed: { type: "number", description: "Number of trades you reviewed." },
          period_win_rate: { type: "number", description: "Win rate for the period you reviewed." },
          period_pnl: { type: "number", description: "Total P&L for the period you reviewed." },
        },
        required: ["confidence_score", "reasoning"],
      },
      handler: async (args) => {
        const memo = createAgentMemo({
          agent: agentType,
          evolution_cycle_id: cycleId,
          memo_type: "recommendation",
          confidence_score: args.confidence_score as number,
          reasoning: args.reasoning as string,
          trades_reviewed: (args.trades_reviewed as number) ?? null,
          period_win_rate: (args.period_win_rate as number) ?? null,
          period_pnl: (args.period_pnl as number) ?? null,
        });
        const memoId = await writeMemo(pool, memo);
        return JSON.stringify({ status: "success", memo_id: memoId });
      },
    });
  }

  if (nameSet.has("get_beliefs")) {
    tools.push({
      name: "get_beliefs",
      description: "Fetch active investment beliefs accumulated from past analysis cycles. These represent confirmed patterns and principles.",
      input_schema: {
        type: "object",
        properties: {
          category: { type: "string", description: "Filter by category (e.g., 'entry_timing', 'risk_management', 'indicator_tuning'). Omit for all." },
        },
        required: [],
      },
      handler: async (args) => {
        const beliefs = await getActiveBeliefs(pool, args.category as string | undefined);
        return JSON.stringify(beliefs);
      },
    });
  }

  if (nameSet.has("write_belief")) {
    tools.push({
      name: "write_belief",
      description: "Record a new investment belief based on accumulated evidence. Beliefs persist across cycles and inform future analysis.",
      input_schema: {
        type: "object",
        properties: {
          belief_text: { type: "string", description: "The investment belief or principle, stated clearly and concisely." },
          confidence: { type: "number", description: "Confidence in this belief, 0.0 to 1.0." },
          category: { type: "string", description: "Category for grouping (e.g., 'entry_timing', 'risk_management', 'indicator_tuning', 'market_regime', 'exit_strategy')." },
          evidence_summary: { type: "string", description: "Brief summary of evidence supporting this belief." },
          source_memo_ids: { type: "array", items: { type: "number" }, description: "IDs of memos that support this belief." },
        },
        required: ["belief_text", "confidence", "category", "evidence_summary"],
      },
      handler: async (args) => {
        const belief = createBelief({
          belief_text: args.belief_text as string,
          confidence: args.confidence as number,
          category: args.category as string,
          source_memo_ids: (args.source_memo_ids as number[]) ?? [],
        });
        const beliefId = await writeBelief(pool, belief);
        return JSON.stringify({ status: "success", belief_id: beliefId });
      },
    });
  }

  if (nameSet.has("read_logs")) {
    tools.push({
      name: "read_logs",
      description: "Read system logs from paper_trader, backtest, or agents. Use to review execution details, indicator scores, trade entries/exits, and errors.",
      input_schema: {
        type: "object",
        properties: {
          date: { type: "string", description: "Date in YYYY-MM-DD format." },
          source: { type: "string", enum: ["paper_trader", "backtest", "agents"], description: "Log source to read." },
          ticker: { type: "string", description: "Filter lines containing this ticker symbol." },
          level: { type: "string", description: "Filter by log level: INFO, WARN, or ERROR." },
          tail: { type: "number", description: "Return last N matching lines. Default: 200." },
        },
        required: ["date", "source"],
      },
      handler: async (args) => {
        const params: ReadLogsParams = {
          date: args.date as string,
          source: args.source as ReadLogsParams["source"],
          ticker: args.ticker as string | undefined,
          level: args.level as string | undefined,
          tail: args.tail as number | undefined,
        };
        return readLogs(params);
      },
    });
  }

  return tools;
}

// --- agent runner ---

async function runAgentLoop(
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

  const toolDefs = buildTools(pool, cycleId, agentType, toolSet, capture);

  // build Anthropic API tool definitions
  const apiTools: Tool[] = toolDefs.map((t) => ({
    name: t.name,
    description: t.description,
    input_schema: t.input_schema as Tool["input_schema"],
  }));

  // build handler lookup
  const handlerMap = new Map<string, (args: Record<string, unknown>) => Promise<string>>();
  for (const t of toolDefs) {
    handlerMap.set(t.name, t.handler);
  }

  const client = new Anthropic();
  const messages: MessageParam[] = [
    { role: "user", content: initialMessage },
  ];

  for (let turn = 0; turn < maxTurns; turn++) {
    const response = await client.messages.create({
      model,
      max_tokens: 16384,
      system: systemPrompt,
      tools: apiTools,
      messages,
    });

    // accumulate token usage
    totalUsage.input_tokens += response.usage.input_tokens;
    totalUsage.output_tokens += response.usage.output_tokens;

    // if the model didn't request tool use, we're done
    if (response.stop_reason !== "tool_use") {
      break;
    }

    const toolUseBlocks = response.content.filter(
      (block): block is ToolUseBlock => block.type === "tool_use",
    );

    // add assistant message to conversation
    messages.push({ role: "assistant", content: response.content });

    // execute each tool and build results
    const toolResults: ToolResultBlockParam[] = [];
    for (const toolUse of toolUseBlocks) {
      const handler = handlerMap.get(toolUse.name);
      if (!handler) {
        toolResults.push({
          type: "tool_result",
          tool_use_id: toolUse.id,
          content: JSON.stringify({ error: `unknown tool: ${toolUse.name}` }),
          is_error: true,
        });
        continue;
      }

      try {
        const result = await handler(toolUse.input as Record<string, unknown>);
        toolResults.push({
          type: "tool_result",
          tool_use_id: toolUse.id,
          content: result,
        });
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        logger.error(`tool ${toolUse.name} failed: ${errMsg}`);
        toolResults.push({
          type: "tool_result",
          tool_use_id: toolUse.id,
          content: JSON.stringify({ error: errMsg }),
          is_error: true,
        });
      }
    }

    // add tool results as user message
    messages.push({ role: "user", content: toolResults });
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
  return runAgentLoop(
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

  const usage = await runAgentLoop(
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
