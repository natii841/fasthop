import { randomBytes } from 'node:crypto';
import { and, eq, lt, or } from 'drizzle-orm';
import { db } from './client';
import { users, User } from './schema';
import { config } from '../config';
import { invalidateToken } from '../utils/tokenCache';

export const newToken = () => randomBytes(24).toString('base64url'); // 32 chars

export function effectiveExpiry(u: User): Date | null {
  if (u.status === 'trial') return u.trialExpiresAt;
  if (u.status === 'active') return u.subscriptionExpiresAt;
  return null;
}

export async function getByTelegramId(telegramId: number): Promise<User | undefined> {
  const [row] = await db.select().from(users).where(eq(users.telegramId, telegramId)).limit(1);
  return row;
}

/** One Telegram account = one trial (telegram_id is unique). */
export async function getOrCreateUser(from: { id: number; first_name?: string; username?: string }): Promise<User> {
  const existing = await getByTelegramId(from.id);
  if (existing) return existing;
  const now = new Date();
  await db.insert(users).values({
    telegramId: from.id,
    firstName: from.first_name ?? null,
    username: from.username ?? null,
    token: newToken(),
    status: 'trial',
    trialStartedAt: now,
    trialExpiresAt: new Date(now.getTime() + config.trialHours * 3600_000),
  }).onConflictDoNothing();
  return (await getByTelegramId(from.id))!;
}

export async function rotateToken(telegramId: number): Promise<User | undefined> {
  const u = await getByTelegramId(telegramId);
  if (!u) return;
  await db.update(users).set({ token: newToken(), updatedAt: new Date() }).where(eq(users.telegramId, telegramId));
  invalidateToken(u.token);
  return getByTelegramId(telegramId);
}

export async function grantDays(telegramId: number, days: number): Promise<User | undefined> {
  const u = await getByTelegramId(telegramId);
  if (!u) return;
  const base = Math.max(Date.now(), u.subscriptionExpiresAt?.getTime() ?? 0); // stacking renewals
  await db.update(users).set({
    status: 'active',
    subscriptionExpiresAt: new Date(base + days * 86400_000),
    updatedAt: new Date(),
  }).where(eq(users.telegramId, telegramId));
  invalidateToken(u.token);
  return getByTelegramId(telegramId);
}

export async function setStatus(telegramId: number, status: 'paused' | 'expired'): Promise<User | undefined> {
  const u = await getByTelegramId(telegramId);
  if (!u) return;
  await db.update(users).set({ status, updatedAt: new Date() }).where(eq(users.telegramId, telegramId));
  invalidateToken(u.token);
  return getByTelegramId(telegramId);
}

export async function resume(telegramId: number): Promise<User | undefined> {
  const u = await getByTelegramId(telegramId);
  if (!u) return;
  const now = Date.now();
  const status = (u.subscriptionExpiresAt?.getTime() ?? 0) > now ? 'active'
    : (u.trialExpiresAt?.getTime() ?? 0) > now ? 'trial' : 'expired';
  await db.update(users).set({ status, updatedAt: new Date() }).where(eq(users.telegramId, telegramId));
  invalidateToken(u.token);
  return getByTelegramId(telegramId);
}

/** Flip lapsed trial/active rows to 'expired'; returns Telegram ids to notify. */
export async function sweepExpired(): Promise<number[]> {
  const now = new Date();
  const rows = await db.update(users).set({ status: 'expired', updatedAt: now }).where(or(
    and(eq(users.status, 'trial'), lt(users.trialExpiresAt, now)),
    and(eq(users.status, 'active'), lt(users.subscriptionExpiresAt, now)),
  )).returning({ telegramId: users.telegramId, token: users.token });
  rows.forEach((r) => invalidateToken(r.token));
  return rows.map((r) => r.telegramId);
}
