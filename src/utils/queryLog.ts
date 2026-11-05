import { lt } from 'drizzle-orm';
import { db } from '../db/client';
import { queryLogs } from '../db/schema';
import { config } from '../config';

type Row = typeof queryLogs.$inferInsert;
let queue: Row[] = [];

export function queueLog(row: Row) {
  if (queue.length < 5000) queue.push(row); // drop under pressure rather than grow unbounded
}
export async function flushLogs() {
  if (!queue.length) return;
  const batch = queue; queue = [];
  try { await db.insert(queryLogs).values(batch); } catch { /* logging must never break DNS */ }
}
export async function pruneLogs() {
  const cutoff = new Date(Date.now() - config.logRetentionDays * 86400_000);
  await db.delete(queryLogs).where(lt(queryLogs.timestamp, cutoff)).catch(() => undefined);
}
