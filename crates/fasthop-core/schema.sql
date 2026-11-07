CREATE TABLE IF NOT EXISTS users (
  id BIGSERIAL PRIMARY KEY,
  telegram_id BIGINT UNIQUE NOT NULL,
  first_name VARCHAR(255),
  username VARCHAR(255),
  token VARCHAR(64) UNIQUE NOT NULL,
  status VARCHAR(32) NOT NULL DEFAULT 'trial',
  trial_expires_at TIMESTAMPTZ,
  subscription_expires_at TIMESTAMPTZ,
  last_active_ip VARCHAR(64),
  ip_change_counter INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS query_logs (
  id BIGSERIAL PRIMARY KEY,
  user_token VARCHAR(64) REFERENCES users(token) ON DELETE CASCADE,
  domain VARCHAR(255) NOT NULL,
  client_ip VARCHAR(64),
  resolved_ip VARCHAR(64),
  timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS query_logs_ts_idx ON query_logs(timestamp);
