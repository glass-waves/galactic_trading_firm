import pg from "pg";

const { Pool } = pg;

let pool: pg.Pool | null = null;

export function getPool(): pg.Pool {
  if (!pool) {
    pool = new Pool({
      connectionString:
        process.env.DATABASE_URL ??
        "postgres://postgres:postgres@localhost:5433/adaptive_trading",
      max: 10,
      idleTimeoutMillis: 30_000,
    });
  }
  return pool;
}

// typed query helper
export async function query<T extends pg.QueryResultRow = pg.QueryResultRow>(
  text: string,
  params?: unknown[]
): Promise<T[]> {
  const result = await getPool().query<T>(text, params);
  return result.rows;
}
