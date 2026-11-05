import { pgTable, serial, bigserial, bigint, varchar, integer, timestamp } from 'drizzle-orm/pg-core';

export const users = pgTable('users', {
  id: serial('id').primaryKey(),
  telegramId: bigint('telegram_id', { mode: 'number' }).notNull().unique(),
  firstName: varchar('first_name', { length: 255 }),
  username: varchar('username', { length: 255 }),
  token: varchar('token', { length: 64 }).notNull().unique(),
  status: varchar('status', { length: 32 }).notNull().default('trial'), // trial | active | expired | paused
  trialStartedAt: timestamp('trial_started_at', { withTimezone: true }),
  trialExpiresAt: timestamp('trial_expires_at', { withTimezone: true }),
  subscriptionExpiresAt: timestamp('subscription_expires_at', { withTimezone: true }),
  lastActiveIp: varchar('last_active_ip', { length: 64 }),
  ipChangeCounter: integer('ip_change_counter').notNull().default(0),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
});

export const queryLogs = pgTable('query_logs', {
  id: bigserial('id', { mode: 'number' }).primaryKey(),
  userToken: varchar('user_token', { length: 64 }).references(() => users.token, { onDelete: 'cascade', onUpdate: 'cascade' }),
  domain: varchar('domain', { length: 255 }).notNull(),
  clientIp: varchar('client_ip', { length: 64 }),
  resolvedIp: varchar('resolved_ip', { length: 64 }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull().defaultNow(),
});

export type User = typeof users.$inferSelect;
