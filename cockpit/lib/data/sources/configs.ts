import { query } from "@/lib/db";
import type { ConfigVersion, ConfigChangelog } from "@/lib/types";

export async function getConfigVersions(
  status?: string
): Promise<ConfigVersion[]> {
  if (status) {
    return query<ConfigVersion>(
      `SELECT id, status, created_at, promoted_at, created_by, parent_version_id,
              mutation_reason, config_blob, backtest_sharpe, backtest_win_rate,
              backtest_total_trades
       FROM config_versions WHERE status = $1 ORDER BY id DESC`,
      [status]
    );
  }
  return query<ConfigVersion>(
    `SELECT id, status, created_at, promoted_at, created_by, parent_version_id,
            mutation_reason, config_blob, backtest_sharpe, backtest_win_rate,
            backtest_total_trades
     FROM config_versions ORDER BY id DESC`
  );
}

export async function getConfigById(id: number): Promise<ConfigVersion | null> {
  const rows = await query<ConfigVersion>(
    `SELECT * FROM config_versions WHERE id = $1`,
    [id]
  );
  return rows[0] ?? null;
}

export async function getLatestPromoted(): Promise<ConfigVersion | null> {
  const rows = await query<ConfigVersion>(
    `SELECT id, status, created_at, promoted_at, created_by, parent_version_id,
            mutation_reason, config_blob, backtest_sharpe, backtest_win_rate,
            backtest_total_trades
     FROM config_versions WHERE status = 'promoted' ORDER BY promoted_at DESC LIMIT 1`
  );
  return rows[0] ?? null;
}

export async function getConfigChangelog(
  configVersionId?: number
): Promise<ConfigChangelog[]> {
  if (configVersionId) {
    return query<ConfigChangelog>(
      `SELECT * FROM config_changelog WHERE config_version_id = $1 ORDER BY id`,
      [configVersionId]
    );
  }
  return query<ConfigChangelog>(
    `SELECT * FROM config_changelog ORDER BY id DESC LIMIT 200`
  );
}

export async function getConfigLineage(): Promise<
  { id: number; parent_version_id: number | null; status: string }[]
> {
  return query(
    `SELECT id, parent_version_id, status FROM config_versions ORDER BY id`
  );
}
