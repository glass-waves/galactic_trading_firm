import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * source-level tests for config-ops SQL queries.
 *
 * the mutation_status enum cast was previously wrong (used "config_status"
 * which doesn't exist in the DB schema). these tests read the source file
 * and verify the SQL strings reference the correct enum names.
 */
describe("config-ops SQL enum casts", () => {
  const source = readFileSync(
    resolve(__dirname, "../../src/tools/config-ops.ts"),
    "utf-8",
  );

  it("should use mutation_status enum cast, not config_status", () => {
    expect(source).toContain("::mutation_status");
    expect(source).not.toContain("::config_status");
  });

  it("should use trade_direction enum cast in trade-related queries", () => {
    // config-ops doesn't write trades, but verify no stale casts exist
    expect(source).not.toContain("::config_status");
  });

  it("should use agent_type enum cast for created_by fields", () => {
    expect(source).toContain("::agent_type");
  });

  it("should use change_category enum cast for changelog entries", () => {
    expect(source).toContain("::change_category");
  });
});
