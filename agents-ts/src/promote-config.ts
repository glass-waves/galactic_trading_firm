/**
 * manual config promotion CLI.
 *
 * reads a config JSON file, proposes it as a new config version, computes
 * the diff against the current promoted config, writes changelog entries,
 * and promotes it — all with proper audit trail as if the PM agent did it.
 *
 * usage:
 *   npx tsx src/promote-config.ts --config <path.json> --reason "description of changes"
 *   npx tsx src/promote-config.ts --config <path.json> --reason "..." --created-by orchestrator
 *   npx tsx src/promote-config.ts --config <path.json> --reason "..." --dry-run
 */

import { parseArgs } from "node:util";
import pg from "pg";
import {
  getCurrentConfig,
  proposeConfig,
  updateConfigStatus,
  writeChangelogEntries,
} from "./tools/config-ops.js";
import { computeConfigDiff } from "./tools/config-diff.js";
import { readFileSync } from "node:fs";

const { Pool } = pg;

async function main(): Promise<void> {
  const { values } = parseArgs({
    options: {
      config: { type: "string" },
      reason: { type: "string" },
      "created-by": { type: "string", default: "orchestrator" },
      "dry-run": { type: "boolean", default: false },
    },
    strict: true,
  });

  if (!values.config || !values.reason) {
    console.error(
      "usage: npx tsx src/promote-config.ts --config <path.json> --reason <description>",
    );
    console.error("options:");
    console.error("  --created-by <agent_type>  default: orchestrator");
    console.error("  --dry-run                  show diff without applying");
    process.exit(1);
  }

  const configPath = values.config;
  const reason = values.reason;
  const createdBy = values["created-by"] ?? "orchestrator";
  const dryRun = values["dry-run"] ?? false;

  // load new config from file
  let newConfigBlob: Record<string, unknown>;
  try {
    const raw = readFileSync(configPath, "utf-8");
    newConfigBlob = JSON.parse(raw) as Record<string, unknown>;
  } catch (err) {
    console.error(`error: failed to read config from ${configPath}:`, err);
    process.exit(1);
  }

  // connect to postgres
  const databaseUrl =
    process.env.DATABASE_URL ??
    "postgresql://postgres:postgres@localhost:5433/adaptive_trading";
  const pool = new Pool({ connectionString: databaseUrl });

  try {
    // fetch current promoted config
    const current = await getCurrentConfig(pool);
    if (!current) {
      console.error("error: no currently promoted config found in database");
      process.exit(1);
    }

    const currentConfig = current.config as Record<string, unknown>;
    const parentVersionId = current.config_version_id as number;

    console.log(`current promoted config: v${parentVersionId}`);

    // compute diff
    const changes = computeConfigDiff(currentConfig, newConfigBlob);

    if (changes.length === 0) {
      console.log("no changes detected between current and new config.");
      process.exit(0);
    }

    console.log(`\nchanges detected: ${changes.length}`);
    console.log("---");
    for (const c of changes) {
      const scope = [c.target_tool_type, c.target_tool_id, c.target_timescale]
        .filter(Boolean)
        .join(" / ");
      console.log(
        `  [${c.change_category}] ${c.target_param}: ${JSON.stringify(c.old_value)} → ${JSON.stringify(c.new_value)}${scope ? ` (${scope})` : ""}`,
      );
    }

    if (dryRun) {
      console.log("\n--dry-run: no changes applied.");
      process.exit(0);
    }

    // propose
    console.log(`\nproposing new config (parent: v${parentVersionId})...`);
    const newVersionId = await proposeConfig(
      pool,
      newConfigBlob,
      parentVersionId,
      reason,
      createdBy,
    );
    console.log(`  created config version v${newVersionId} (status: proposed)`);

    // promote (supersedes current)
    await updateConfigStatus(pool, newVersionId, "promoted");
    console.log(`  promoted v${newVersionId}`);

    // write changelog
    const changelogIds = await writeChangelogEntries(
      pool,
      newVersionId,
      changes,
      createdBy,
    );
    console.log(
      `  wrote ${changelogIds.length} changelog entries: [${changelogIds.join(", ")}]`,
    );

    console.log("\ndone.");
  } finally {
    await pool.end();
  }
}

main().catch((err) => {
  console.error("fatal:", err);
  process.exit(1);
});
