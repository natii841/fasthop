import { eq, sql } from 'drizzle-orm';
import { isIPv4, isIPv6 } from 'node:net';
import { db } from '../db/client';
import { users } from '../db/schema';
import { config } from '../config';

export interface TokenEntry { userId: number; valid: boolean }

interface CacheRec { userId: number | null; until: number; at: number }
const POSITIVE_TTL = 30_000;
const NEGATIVE_TTL = 10_000;
const cache = new Map<string, CacheRec>();
const sessions = new Map<string, { key: string; ip: string; lastSeen: number }>();
const pendingIp = new Map<string, { ip: string; changes: number }>();

export function invalidateToken(token: string) { cache.delete(token); sessions.delete(token); }

/**
 * Cached token lookup. null = unknown token; valid=false = known but expired/paused.
 * The expiry instant is cached too, so a stale entry can never extend access past its deadline.
 */
export async function lookupToken(token: string): Promise<TokenEntry | null> {
  const now = Date.now();
  let rec = cache.get(token);
  if (!rec || now - rec.at >= (rec.userId !== null ? POSITIVE_TTL : NEGATIVE_TTL)) {
    const [u] = await db.select().from(users).where(eq(users.token, token)).limit(1);
    const exp = u ? (u.status === 'trial' ? u.trialExpiresAt : u.status === 'active' ? u.subscriptionExpiresAt : null) : null;
    rec = { userId: u?.id ?? null, until: exp?.getTime() ?? 0, at: now };
    cache.set(token, rec);
  }
  return rec.userId === null ? null : { userId: rec.userId, valid: rec.until > now };
}

/* ---------- single-device enforcement ---------- */

function expandV6(ip: string): string[] {
  if (!ip.includes('::')) return ip.split(':');
  const [h, t] = ip.split('::');
  const head = h ? h.split(':') : [];
  const tail = t ? t.split(':') : [];
  return [...head, ...Array(Math.max(0, 8 - head.length - tail.length)).fill('0'), ...tail];
}

/** Collapse an address to a carrier-NAT-tolerant "network": IPv4 /24, IPv6 /64. */
export function networkKey(raw: string): string {
  const ip = raw.replace(/^::ffff:/i, '');
  if (isIPv4(ip)) return ip.split('.').slice(0, 3).join('.');
  if (isIPv6(ip)) return expandV6(ip).slice(0, 4).map((x) => x.padStart(4, '0')).join(':').toLowerCase();
  return ip;
}

/**
 * Returns false when a *different* network uses the token while the original is still active.
 * Roaming users (wifi<->LTE, CGNAT rotation) are fine: the old network goes quiet and, after
 * DEVICE_SWITCH_SECONDS, the new one takes over. In-memory => single instance; use Redis to scale out.
 */
export function checkDevice(token: string, clientIp: string): boolean {
  const now = Date.now();
  const key = networkKey(clientIp);
  const s = sessions.get(token);
  if (!s) {
    sessions.set(token, { key, ip: clientIp, lastSeen: now });
    pendingIp.set(token, { ip: clientIp, changes: 0 });
    return true;
  }
  if (s.key === key) { s.lastSeen = now; s.ip = clientIp; return true; }
  if (now - s.lastSeen > config.deviceSwitchSeconds * 1000) {
    sessions.set(token, { key, ip: clientIp, lastSeen: now });
    const p = pendingIp.get(token);
    pendingIp.set(token, { ip: clientIp, changes: (p?.changes ?? 0) + 1 });
    return true;
  }
  return false;
}

export async function flushIpUpdates(): Promise<void> {
  const batch = [...pendingIp.entries()];
  pendingIp.clear();
  for (const [token, { ip, changes }] of batch) {
    const s = sessions.get(token);
    await db.update(users).set({
      lastActiveIp: s?.ip ?? ip,
      ipChangeCounter: sql`${users.ipChangeCounter} + ${changes}`,
      updatedAt: new Date(),
    }).where(eq(users.token, token)).catch(() => undefined);
    if (s && Date.now() - s.lastSeen < 120_000) pendingIp.set(token, { ip: s.ip, changes: 0 });
  }
}

/* ---------- brute-force guard for bad tokens ---------- */

const failures = new Map<string, { count: number; resetAt: number }>();
const MAX_FAILS = 20, FAIL_WINDOW = 5 * 60_000;

export function isBlocked(ip: string): boolean {
  const f = failures.get(ip);
  return !!f && Date.now() < f.resetAt && f.count >= MAX_FAILS;
}
export function recordFailure(ip: string) {
  const now = Date.now();
  const f = failures.get(ip);
  if (!f || now > f.resetAt) failures.set(ip, { count: 1, resetAt: now + FAIL_WINDOW });
  else f.count++;
}

setInterval(() => {
  const now = Date.now();
  for (const [k, v] of failures) if (now > v.resetAt) failures.delete(k);
  for (const [k, v] of sessions) if (now - v.lastSeen > 3600_000) sessions.delete(k);
  for (const [k, v] of cache) if (now - v.at > 300_000) cache.delete(k);
}, 300_000).unref();
