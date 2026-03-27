import { describe, it, expect, vi, beforeEach } from "vitest";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROMPTS_DIR = join(__dirname, "..", "prompts");

// mock the Anthropic SDK before importing agent-base
const mockCreate = vi.fn();
vi.mock("@anthropic-ai/sdk", () => {
  return {
    default: vi.fn(() => ({
      messages: { create: mockCreate },
    })),
  };
});

import {
  loadPrompt,
  buildTools,
  PM_TOOL_NAMES,
  ANALYSIS_TOOL_NAMES,
  runPmAgent,
  runAnalysisAgent,
  OPUS_MODEL,
} from "../src/agent-base.js";
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

  it("should have description and input_schema on all tools", () => {
    const tools = buildTools(mockPool, 1, "agent_analysis", "analysis");
    for (const t of tools) {
      expect(t.name).toBeTruthy();
      expect(t.description).toBeTruthy();
      expect(t.input_schema).toBeDefined();
      expect(t.input_schema.type).toBe("object");
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

  it("should have handler functions on all tools", () => {
    const tools = buildTools(mockPool, 1, "agent_pm", "pm");
    for (const t of tools) {
      expect(typeof t.handler).toBe("function");
    }
  });
});

// --- API integration mock tests ---

function makeApiResponse(
  inputTokens = 500,
  outputTokens = 200,
  stopReason: "end_turn" | "tool_use" = "end_turn",
) {
  return {
    id: "msg_test",
    type: "message" as const,
    role: "assistant" as const,
    content: [{ type: "text" as const, text: "Analysis complete." }],
    model: "claude-opus-4-6",
    stop_reason: stopReason,
    usage: { input_tokens: inputTokens, output_tokens: outputTokens },
  };
}

describe("runPmAgent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return null version when no proposal", async () => {
    mockCreate.mockResolvedValueOnce(makeApiResponse(1500, 700));

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
    mockCreate.mockResolvedValueOnce(makeApiResponse(1200, 600));

    const mockPool = {} as any;
    const usage = await runAnalysisAgent(mockPool, 1);

    expect(usage.input_tokens).toBe(1200);
    expect(usage.output_tokens).toBe(600);
    expect(usage.model).toBe(OPUS_MODEL);

    // verify the model was passed to the API
    expect(mockCreate).toHaveBeenCalledWith(
      expect.objectContaining({ model: OPUS_MODEL }),
    );
  });
});
