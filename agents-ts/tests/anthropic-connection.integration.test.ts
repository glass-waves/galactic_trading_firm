/**
 * integration test — verifies real connection to the anthropic API.
 *
 * requires ANTHROPIC_API_KEY in environment. uses haiku for minimal cost.
 * excluded from normal test runs. run explicitly:
 *
 *   npx vitest run tests/anthropic-connection.integration.test.ts
 */

import { describe, it, expect } from "vitest";
import { config } from "dotenv";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
config({ path: join(__dirname, "..", "..", ".env") });

import Anthropic from "@anthropic-ai/sdk";
import { HAIKU_MODEL } from "../src/agent-base.js";

describe("anthropic API connection", () => {
  it("should complete a simple message", async () => {
    const client = new Anthropic();

    const response = await client.messages.create({
      model: HAIKU_MODEL,
      max_tokens: 32,
      messages: [{ role: "user", content: "Reply with exactly: ok" }],
    });

    expect(response.id).toBeTruthy();
    expect(response.role).toBe("assistant");
    expect(response.stop_reason).toBe("end_turn");
    expect(response.usage.input_tokens).toBeGreaterThan(0);
    expect(response.usage.output_tokens).toBeGreaterThan(0);
    expect(response.content.length).toBeGreaterThan(0);
    expect(response.content[0].type).toBe("text");
  }, 15_000);

  it("should handle tool use round-trip", async () => {
    const client = new Anthropic();

    const response = await client.messages.create({
      model: HAIKU_MODEL,
      max_tokens: 128,
      messages: [{ role: "user", content: "What is 2+2? Use the calculator tool." }],
      tools: [
        {
          name: "calculator",
          description: "Adds two numbers.",
          input_schema: {
            type: "object" as const,
            properties: {
              a: { type: "number" },
              b: { type: "number" },
            },
            required: ["a", "b"],
          },
        },
      ],
    });

    expect(response.stop_reason).toBe("tool_use");
    const toolUse = response.content.find((b) => b.type === "tool_use");
    expect(toolUse).toBeDefined();
    if (toolUse && toolUse.type === "tool_use") {
      expect(toolUse.name).toBe("calculator");
      expect(toolUse.id).toBeTruthy();
      const input = toolUse.input as { a: number; b: number };
      expect(input.a).toBe(2);
      expect(input.b).toBe(2);
    }
  }, 15_000);
});
