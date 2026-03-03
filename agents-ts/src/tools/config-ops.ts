/**
 * config version read/write operations.
 *
 * provides tools for agents to:
 * - read the current promoted config
 * - propose new config versions (creates config_versions row with status='proposed')
 * - update config version status (backtesting → promoted/rejected)
 * - write changelog entries from computed diffs
 */

import type pg from "pg";
import type { ConfigChange } from "./config-diff.js";

export async function getCurrentConfig(
  pool: pg.Pool,
): Promise<Record<string, unknown> | null> {
  const result = await pool.query(`
    SELECT id, config_blob, promoted_at, created_by::text, mutation_reason
    FROM config_versions
    WHERE status = 'promoted'
    ORDER BY promoted_at DESC
    LIMIT 1
  `);

  const row = result.rows[0];
  if (!row) return null;

  return {
    config_version_id: row.id,
    config: row.config_blob,
    promoted_at: String(row.promoted_at),
    created_by: row.created_by,
    mutation_reason: row.mutation_reason,
  };
}

export async function getConfigVersion(
  pool: pg.Pool,
  versionId: number,
): Promise<Record<string, unknown> | null> {
  const result = await pool.query(
    `
    SELECT id, config_blob, status::text, created_at, promoted_at,
           created_by::text, mutation_reason,
           backtest_sharpe, backtest_win_rate, backtest_total_trades
    FROM config_versions
    WHERE id = $1
    `,
    [versionId],
  );

  const row = result.rows[0];
  return row ? (row as Record<string, unknown>) : null;
}

export async function proposeConfig(
  pool: pg.Pool,
  configBlob: Record<string, unknown>,
  parentVersionId: number | null,
  mutationReason: string,
  createdBy = "agent_pm",
): Promise<number> {
  const result = await pool.query(
    `
    INSERT INTO config_versions (
      config_blob, status, parent_version_id,
      mutation_reason, created_by
    ) VALUES (
      $1::jsonb, 'proposed'::mutation_status,
      $2, $3, $4::agent_type
    )
    RETURNING id
    `,
    [JSON.stringify(configBlob), parentVersionId, mutationReason, createdBy],
  );
  return result.rows[0].id as number;
}

export async function updateConfigStatus(
  pool: pg.Pool,
  versionId: number,
  newStatus: string,
  backtestResults?: Record<string, unknown>,
): Promise<void> {
  const client = await pool.connect();
  try {
    await client.query("BEGIN");

    if (newStatus === "promoted") {
      // supersede any currently promoted config
      await client.query(`
        UPDATE config_versions
        SET status = 'superseded'::mutation_status
        WHERE status = 'promoted'
      `);

      if (backtestResults) {
        await client.query(
          `
          UPDATE config_versions SET
            status = $1::mutation_status,
            promoted_at = now(),
            backtest_sharpe = $2,
            backtest_win_rate = $3,
            backtest_total_trades = $4
          WHERE id = $5
          `,
          [
            newStatus,
            backtestResults.sharpe_ratio,
            backtestResults.win_rate,
            backtestResults.total_trades,
            versionId,
          ],
        );
      } else {
        await client.query(
          `
          UPDATE config_versions SET
            status = $1::mutation_status,
            promoted_at = now()
          WHERE id = $2
          `,
          [newStatus, versionId],
        );
      }
    } else {
      await client.query(
        `
        UPDATE config_versions SET
          status = $1::mutation_status
        WHERE id = $2
        `,
        [newStatus, versionId],
      );
    }

    await client.query("COMMIT");
  } catch (err) {
    await client.query("ROLLBACK");
    throw err;
  } finally {
    client.release();
  }
}

export async function writeChangelogEntries(
  pool: pg.Pool,
  versionId: number,
  changes: ConfigChange[],
  changedBy = "agent_pm",
  sourceMemoId: number | null = null,
): Promise<number[]> {
  if (changes.length === 0) return [];

  const client = await pool.connect();
  const ids: number[] = [];

  try {
    await client.query("BEGIN");

    for (const change of changes) {
      const result = await client.query(
        `
        INSERT INTO config_changelog (
          config_version_id, changed_by,
          change_category, target_timescale,
          target_tool_id, target_tool_type, target_param,
          old_value, new_value, reason, source_memo_id
        ) VALUES (
          $1, $2::agent_type,
          $3::change_category, $4,
          $5, $6, $7,
          $8, $9, $10, $11
        )
        RETURNING id
        `,
        [
          versionId,
          changedBy,
          change.change_category,
          change.target_timescale,
          change.target_tool_id,
          change.target_tool_type,
          change.target_param,
          JSON.stringify(change.old_value),
          JSON.stringify(change.new_value),
          change.reason,
          sourceMemoId,
        ],
      );
      ids.push(result.rows[0].id as number);
    }

    await client.query("COMMIT");
  } catch (err) {
    await client.query("ROLLBACK");
    throw err;
  } finally {
    client.release();
  }

  return ids;
}
