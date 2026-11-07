CREATE TYPE sub_status AS ENUM ('trial', 'active', 'paused', 'expired');

CREATE TABLE subscriptions (
    telegram_id BIGINT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    status sub_status NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE FUNCTION notify_subscription_change() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('fasthop_subscriptions', '');
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER subscriptions_notify
AFTER INSERT OR UPDATE OR DELETE ON subscriptions
FOR EACH STATEMENT EXECUTE FUNCTION notify_subscription_change();
