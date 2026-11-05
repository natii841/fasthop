import pg from 'pg';
import { drizzle } from 'drizzle-orm/node-postgres';
import { config } from '../config';
import * as schema from './schema';

export const pool = new pg.Pool({
  connectionString: config.databaseUrl,
  max: 10,
  ssl: config.databaseSsl ? { rejectUnauthorized: false } : undefined,
});
export const db = drizzle(pool, { schema });

/** Idempotent bootstrap so a fresh database works with zero migration tooling. */
export async function ensureSchema(): Promise<void> {
  await pool.query(`
    CREATE TABLE IF NOT EXISTS users (
      id SERIAL PRIMARY KEY,
      telegram_id BIGINT UNIQUE NOT NULL,
      first_name VARCHAR(255),
      username VARCHAR(255),
      token VARCHAR(64) UNIQUE NOT NULL,
      status VARCHAR(32) NOT NULL DEFAULT 'trial',
      trial_started_at TIMESTAMPTZ,
      trial_expires_at TIMESTAMPTZ,
      subscription_expires_at TIMESTAMPTZ,
      last_active_ip VARCHAR(64),
      ip_change_counter INT NOT NULL DEFAULT 0,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );
    CREATE TABLE IF NOT EXISTS query_logs (
      id BIGSERIAL PRIMARY KEY,
      user_token VARCHAR(64) REFERENCES users(token) ON DELETE CASCADE ON UPDATE CASCADE,
      domain VARCHAR(255) NOT NULL,
      client_ip VARCHAR(64),
      resolved_ip VARCHAR(64),
      timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );
    CREATE INDEX IF NOT EXISTS query_logs_ts_idx ON query_logs (timestamp);
    CREATE INDEX IF NOT EXISTS query_logs_token_idx ON query_logs (user_token);
  `);
}
