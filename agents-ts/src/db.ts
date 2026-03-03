/**
 * database connection management.
 *
 * provides a connection pool via pg.Pool.
 * database url comes from the DATABASE_URL environment variable.
 */

import dotenv from "dotenv";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import pg from "pg";

// load .env from project root (two levels up from src/)
const projectRoot = resolve(fileURLToPath(import.meta.url), "../../../");
dotenv.config({ path: resolve(projectRoot, ".env") });

const { Pool } = pg;

let pool: pg.Pool | null = null;

export function getDatabaseUrl(): string {
  return (
    process.env.DATABASE_URL ??
    "postgres://postgres:postgres@localhost:5433/adaptive_trading"
  );
}

export function getPool(): pg.Pool {
  if (pool === null) {
    pool = new Pool({
      connectionString: getDatabaseUrl(),
      max: 5,
    });
  }
  return pool;
}

export async function closePool(): Promise<void> {
  if (pool !== null) {
    await pool.end();
    pool = null;
  }
}
