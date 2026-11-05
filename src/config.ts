function req(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`Missing required env var: ${name}`);
  return v;
}
const num = (name: string, def: number) => Number(process.env[name] ?? def);

export const config = {
  port: num('PORT', 8080),
  botToken: req('BOT_TOKEN'),
  databaseUrl: req('DATABASE_URL'),
  databaseSsl: process.env.DATABASE_SSL === 'true',
  /** Public HTTPS origin users will put in Intra/DNSecure, no trailing slash. */
  baseUrl: req('BASE_URL').replace(/\/+$/, ''),
  /** If set, bot runs via webhook at WEBHOOK_URL/tg/<WEBHOOK_SECRET>; otherwise long polling. */
  webhookUrl: process.env.WEBHOOK_URL?.replace(/\/+$/, ''),
  webhookSecret: process.env.WEBHOOK_SECRET ?? '',
  adminIds: (process.env.ADMIN_IDS ?? '')
    .split(',').map((s) => s.trim()).filter(Boolean).map(Number),
  supportHandle: process.env.SUPPORT_HANDLE ?? '@admin',
  trialHours: num('TRIAL_HOURS', 5),
  /** A second network must stay silent this long before it can take over a token. */
  deviceSwitchSeconds: num('DEVICE_SWITCH_SECONDS', 120),
  upstreams: (process.env.UPSTREAMS ??
    'https://1.1.1.1/dns-query,https://1.0.0.1/dns-query,https://8.8.8.8/dns-query')
    .split(',').map((s) => s.trim()).filter(Boolean),
  upstreamTimeoutMs: num('UPSTREAM_TIMEOUT_MS', 2000),
  /** Privacy default: only log lookups of rerouted game domains. */
  logAllQueries: process.env.LOG_ALL_QUERIES === 'true',
  logRetentionDays: num('LOG_RETENTION_DAYS', 7),
  routingFile: process.env.ROUTING_FILE ?? 'routing.json',
  routingJson: process.env.ROUTING_JSON,
};
