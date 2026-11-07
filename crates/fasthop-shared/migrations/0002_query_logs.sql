CREATE TABLE query_logs (
    id BIGSERIAL PRIMARY KEY,
    user_token TEXT REFERENCES subscriptions(token) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    client_ip TEXT,
    resolved_ip TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX query_logs_timestamp_idx ON query_logs(timestamp);
