import { query } from "@/lib/db";
import type { AgentMemo, Belief, EvolutionCycle, DailyBudget } from "@/lib/types";

export async function getEvolutionCycles(limit = 50): Promise<EvolutionCycle[]> {
  return query<EvolutionCycle>(
    `SELECT * FROM evolution_cycles ORDER BY started_at DESC LIMIT $1`,
    [limit]
  );
}

export async function getAgentMemos(limit = 50): Promise<AgentMemo[]> {
  return query<AgentMemo>(
    `SELECT * FROM agent_memos ORDER BY created_at DESC LIMIT $1`,
    [limit]
  );
}

export async function getBeliefs(status?: string): Promise<Belief[]> {
  if (status) {
    return query<Belief>(
      `SELECT * FROM beliefs WHERE status = $1 ORDER BY confidence DESC`,
      [status]
    );
  }
  return query<Belief>(`SELECT * FROM beliefs ORDER BY confidence DESC`);
}

export async function getActiveBeliefsCount(): Promise<number> {
  const rows = await query<{ count: string }>(
    `SELECT COUNT(*) AS count FROM beliefs WHERE status = 'active'`
  );
  return Number(rows[0]?.count ?? 0);
}

export async function getDailyBudget(
  date?: string
): Promise<DailyBudget | null> {
  const d = date ?? new Date().toISOString().slice(0, 10);
  const rows = await query<DailyBudget>(
    `SELECT * FROM daily_budget WHERE trading_date = $1`,
    [d]
  );
  return rows[0] ?? null;
}

export async function getDailyBudgetHistory(days = 30): Promise<DailyBudget[]> {
  return query<DailyBudget>(
    `SELECT * FROM daily_budget ORDER BY trading_date DESC LIMIT $1`,
    [days]
  );
}
