import { describe, it, expect, vi, beforeEach } from "vitest";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROMPTS_DIR = join(__dirname, "..", "prompts");

// mock the SDK before importing agent-base
vi.mock("@anthropic-ai/claude-agent-sdk", () => ({
  query: vi.fn(),
  createSdkMcpServer: vi.fn(() => ({ type: "sdk", name: "trading" })),
  tool: vi.fn(
    (
      name: string,
      description: string,
      schema: unknown,
      handler: Function,
    ) => ({
      name,
      description,
      inputSchema: schema,
      handler,
    }),
  ),
}));

import {
  loadPrompt,
  buildTools,
  PM_TOOL_NAMES,
  ANALYSIS_TOOL_NAMES,
  runPmAgent,
  runAnalysisAgent,
  OPUS_MODEL,
} from "../src/agent-base.js";
import { query, createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import type { AgentType } from "../src/models.js";

// --- prompt loading tests ---

describe("loadPrompt", () => {
  it("should throw for missing prompt", () => {
    expect(() => loadPrompt("nonexistent_prompt")).toThrow();
  });

  it("should load agent_pm", () => {
    const prompt = loadPrompt("agent_pm");
    expect(prompt.toLowerCase()).toContain("portfolio manager");
    expect(prompt.toLowerCase()).toContain("config");
  });

  it("should load analysis prompt", () => {
    const prompt = loadPrompt("analysis");
    expect(prompt.toLowerCase()).toContain("analysis agent");
    expect(prompt.toLowerCase()).toContain("zero config authority");
  });

});

// --- tool set structure tests ---

describe("tool set names", () => {
  it("should have 12 PM tools", () => {
    expect(PM_TOOL_NAMES).toHaveLength(12);
    expect(PM_TOOL_NAMES).toContain("get_recent_trades");
    expect(PM_TOOL_NAMES).toContain("get_prior_memos");
    expect(PM_TOOL_NAMES).toContain("get_analysis_memos");
    expect(PM_TOOL_NAMES).toContain("get_beliefs");
    expect(PM_TOOL_NAMES).toContain("propose_config_mutation");
    expect(PM_TOOL_NAMES).toContain("write_pm_memo");
    expect(PM_TOOL_NAMES).toContain("write_belief");
    expect(PM_TOOL_NAMES).toContain("read_logs");
  });

  it("should have 7 analysis tools", () => {
    expect(ANALYSIS_TOOL_NAMES).toHaveLength(7);
    expect(ANALYSIS_TOOL_NAMES).toContain("get_recent_trades");
    expect(ANALYSIS_TOOL_NAMES).toContain("get_daily_performance");
    expect(ANALYSIS_TOOL_NAMES).toContain("get_current_config");
    expect(ANALYSIS_TOOL_NAMES).toContain("get_config_changelog");
    expect(ANALYSIS_TOOL_NAMES).toContain("get_performance_by_exit_reason");
    expect(ANALYSIS_TOOL_NAMES).toContain("get_beliefs");
    expect(ANALYSIS_TOOL_NAMES).toContain("write_analysis_memo");
    expect(ANALYSIS_TOOL_NAMES).not.toContain("propose_config_mutation");
    expect(ANALYSIS_TOOL_NAMES).not.toContain("write_belief");
  });

});

// --- buildTools tests ---

describe("buildTools", () => {
  const mockPool = {} as any;

  it("should create correct number of PM tools", () => {
    const tools = buildTools(mockPool, 1, "agent_pm", "pm");
    expect(tools).toHaveLength(PM_TOOL_NAMES.length);
    const names = tools.map((t) => t.name);
    for (const name of PM_TOOL_NAMES) {
      expect(names).toContain(name);
    }
  });

  it("should have description on all tools", () => {
    const tools = buildTools(mockPool, 1, "agent_analysis", "analysis");
    for (const t of tools) {
      expect(t.name).toBeTruthy();
      expect(t.description).toBeTruthy();
    }
  });

  it("should create correct number of analysis tools", () => {
    const tools = buildTools(mockPool, 1, "agent_analysis", "analysis");
    expect(tools).toHaveLength(ANALYSIS_TOOL_NAMES.length);
    const names = tools.map((t) => t.name);
    for (const name of ANALYSIS_TOOL_NAMES) {
      expect(names).toContain(name);
    }
  });

  it("should capture version id for propose_config_mutation", async () => {
    const capture = { proposed_version_id: null as number | null };
    const tools = buildTools(mockPool, 1, "agent_pm", "pm", capture);
    const proposeTool = tools.find(
      (t) => t.name === "propose_config_mutation",
    );
    expect(proposeTool).toBeDefined();

    // mock the DB calls used by the handler
    const { getCurrentConfig } = await import(
      "../src/tools/config-ops.js"
    );
    const { proposeConfig } = await import("../src/tools/config-ops.js");

    vi.mock("../src/tools/config-ops.js", async (importOriginal) => ({
      ...(await importOriginal()),
      getCurrentConfig: vi.fn(async () => ({
        config_version_id: 10,
        config: {},
      })),
      proposeConfig: vi.fn(async () => 42),
    }));

    // reimport to get mocked version
    const { buildTools: buildToolsFresh } = await import(
      "../src/agent-base.js"
    );
    const freshCapture = { proposed_version_id: null as number | null };
    const freshTools = buildToolsFresh(
      mockPool,
      1,
      "agent_pm",
      "pm",
      freshCapture,
    );
    const freshPropose = freshTools.find(
      (t) => t.name === "propose_config_mutation",
    );

    if (freshPropose) {
      const result = await freshPropose.handler(
        { config_blob: {}, mutation_reason: "test" },
        {},
      );
      expect(result.content[0].type).toBe("text");
      // the capture is updated by the handler
      expect(freshCapture.proposed_version_id).toBe(42);
    }
  });
});

// --- SDK query() mock tests ---

function makeResultMessage(inputTokens = 500, outputTokens = 200) {
  return {
    type: "result" as const,
    subtype: "success" as const,
    duration_ms: 1000,
    duration_api_ms: 800,
    is_error: false,
    num_turns: 2,
    session_id: "test-session",
    total_cost_usd: 0.001,
    usage: { input_tokens: inputTokens, output_tokens: outputTokens },
    modelUsage: {
      "claude-sonnet-4-6": {
        inputTokens,
        outputTokens,
        cacheReadInputTokens: 0,
        cacheCreationInputTokens: 0,
        webSearchRequests: 0,
        costUSD: 0.001,
        contextWindow: 200000,
        maxOutputTokens: 16384,
      },
    },
    result: "Analysis complete.",
    structured_output: null,
  };
}

describe("runPmAgent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return null version when no proposal", async () => {
    const mockQuery = vi.mocked(query);
    mockQuery.mockImplementation(async function* () {
      yield makeResultMessage(1500, 700) as any;
    });

    const mockPool = {} as any;
    const [usage, versionId] = await runPmAgent(mockPool, 1);

    expect(versionId).toBeNull();
    expect(usage.input_tokens).toBe(1500);
    expect(usage.output_tokens).toBe(700);
    expect(usage.model).toBe(OPUS_MODEL);
  });
});

describe("runAnalysisAgent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should use OPUS_MODEL by default", async () => {
    let capturedOptions: any = null;

    const mockQuery = vi.mocked(query);
    mockQuery.mockImplementation(async function* (params: any) {
      capturedOptions = params.options;
      yield makeResultMessage(1200, 600) as any;
    });

    const mockPool = {} as any;
    const usage = await runAnalysisAgent(mockPool, 1);

    expect(usage.input_tokens).toBe(1200);
    expect(usage.output_tokens).toBe(600);
    expect(usage.model).toBe(OPUS_MODEL);
    expect(capturedOptions?.model).toBe(OPUS_MODEL);
  });
});


