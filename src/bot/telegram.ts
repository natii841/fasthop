import { Bot, InlineKeyboard } from 'grammy';
import { config } from '../config';
import * as M from './messages';
import { getByTelegramId, getOrCreateUser, grantDays, resume, rotateToken, setStatus } from '../db/users';
import { loadRules } from '../doh/routingRules';

const HTML = { parse_mode: 'HTML' as const, link_preview_options: { is_disabled: true } };
const guideKb = new InlineKeyboard().text('📱 Android (Intra)', 'guide:android').text('🍎 iOS (DNSecure)', 'guide:ios');

export function createBot() {
  const bot = new Bot(config.botToken);
  const isAdmin = (id?: number) => !!id && config.adminIds.includes(id);

  bot.command('start', async (ctx) => {
    if (!ctx.from) return;
    const before = await getByTelegramId(ctx.from.id);
    const u = await getOrCreateUser(ctx.from);
    await ctx.reply(M.welcome(u, !before), { ...HTML, reply_markup: guideKb });
  });

  const withUser = async (ctx: any, fn: (u: NonNullable<Awaited<ReturnType<typeof getByTelegramId>>>) => Promise<unknown>) => {
    const u = ctx.from && await getByTelegramId(ctx.from.id);
    if (!u) return ctx.reply('Send /start first.');
    return fn(u);
  };

  bot.command('status', (ctx) => withUser(ctx, (u) => ctx.reply(M.status(u), HTML)));
  bot.command('link', (ctx) => withUser(ctx, (u) => ctx.reply(`<code>${M.dohUrl(u)}</code>\n\n${M.statusLine(u)}`, HTML)));
  bot.command('setup', (ctx) => withUser(ctx, () => ctx.reply('Which phone do you use?', { reply_markup: guideKb })));
  bot.command('help', (ctx) => ctx.reply(M.help(isAdmin(ctx.from?.id)), HTML));
  bot.command('newtoken', async (ctx) => {
    if (!ctx.from) return;
    const u = await rotateToken(ctx.from.id);
    if (!u) return ctx.reply('Send /start first.');
    await ctx.reply(`🔄 New link — update it in your DNS app:\n<code>${M.dohUrl(u)}</code>`, HTML);
  });

  bot.callbackQuery(/^guide:(android|ios)$/, async (ctx) => {
    await ctx.answerCallbackQuery();
    const u = await getByTelegramId(ctx.from.id);
    if (!u) return;
    await ctx.reply(ctx.match[1] === 'android' ? M.androidGuide(u) : M.iosGuide(u), HTML);
  });

  /* ---- admin ---- */
  const admin = (name: string, fn: (ctx: any, args: string[]) => Promise<unknown>) =>
    bot.command(name, async (ctx) => {
      if (!isAdmin(ctx.from?.id)) return;
      return fn(ctx, ctx.match.trim().split(/\s+/).filter(Boolean));
    });

  admin('grant', async (ctx, [id, days]) => {
    const d = Number(days), tid = Number(id);
    if (!tid || !(d > 0)) return ctx.reply('Usage: /grant <telegram_id> <days>');
    const u = await grantDays(tid, d);
    if (!u) return ctx.reply('User not found (they must /start first).');
    await ctx.reply(`✅ ${tid}: +${d} days. ${u.subscriptionExpiresAt?.toISOString()}`);
    await bot.api.sendMessage(tid, `✅ Payment received — ${d} days added!\n\n${M.statusLine(u)}`, HTML).catch(() => undefined);
  });
  for (const [cmd, fn] of [
    ['pause', (id: number) => setStatus(id, 'paused')],
    ['revoke', (id: number) => setStatus(id, 'expired')],
    ['resume', (id: number) => resume(id)],
  ] as const) {
    admin(cmd, async (ctx, [id]) => {
      const u = await fn(Number(id));
      await ctx.reply(u ? `${cmd}: ${id} → ${u.status}` : 'User not found.');
    });
  }
  admin('reload', async (ctx) => ctx.reply(`Loaded ${loadRules()} routing rules.`));

  bot.catch((e) => console.error('[bot]', e.error));
  return bot;
}
