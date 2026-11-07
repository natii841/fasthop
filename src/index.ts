import Fastify from 'fastify';
import { webhookCallback } from 'grammy';
import { config } from './config';
import { ensureSchema, pool } from './db/client';
import { registerDoh } from './doh/handler';
import { loadRules } from './doh/routingRules';
import { createBot } from './bot/telegram';
import { expiredNotice } from './bot/messages';
import { sweepExpired } from './db/users';
import { flushIpUpdates } from './utils/tokenCache';
import { flushLogs, pruneLogs } from './utils/queryLog';

async function main() {
  await ensureSchema();
  console.log(`[routing] ${loadRules()} rule(s) loaded`);
  process.on('SIGHUP', () => console.log(`[routing] reloaded ${loadRules()} rule(s)`));

  const app = Fastify({ trustProxy: true, bodyLimit: 8192, logger: { level: 'warn' } });
  registerDoh(app);
  app.get('/healthz', async () => ({ ok: true }));

  const bot = createBot();

  // Prevents grammY from halting polling on errors and logs the failure
  bot.catch((err) => {
    console.error(`[bot error] [user:${err.ctx?.from?.id}]:`, err.error);
  });

  await bot.init();
  if (config.webhookUrl) {
    if (!config.webhookSecret) throw new Error('WEBHOOK_SECRET is required when WEBHOOK_URL is set');
    const path = `/tg/${config.webhookSecret}`;
    app.post(path, webhookCallback(bot, 'fastify', { secretToken: config.webhookSecret }));
    await bot.api.setWebhook(config.webhookUrl + path, { secret_token: config.webhookSecret });
  } else {
    await bot.api.deleteWebhook();
    void bot.start({ drop_pending_updates: true });
    console.log('[bot] polling started');
  }

  const timers = [
    setInterval(() => void flushLogs(), 5_000),
    setInterval(() => void flushIpUpdates(), 60_000),
    setInterval(() => void pruneLogs(), 3600_000),
    setInterval(async () => {
      try {
        for (const id of await sweepExpired()) await bot.api.sendMessage(id, expiredNotice()).catch(() => undefined);
      } catch (e) { console.error('[sweep]', e); }
    }, 60_000),
  ];

  await app.listen({ port: config.port, host: '0.0.0.0' });
  console.log(`[doh] listening on :${config.port}`);

  const shutdown = async () => {
    timers.forEach(clearInterval);
    await Promise.allSettled([flushLogs(), flushIpUpdates()]);
    await app.close();
    if (!config.webhookUrl) await bot.stop();
    await pool.end();
    process.exit(0);
  };
  process.on('SIGTERM', shutdown);
  process.on('SIGINT', shutdown);
}

main().catch((e) => { console.error(e); process.exit(1); });