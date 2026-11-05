import type { User } from '../db/schema';
import { config } from '../config';
import { effectiveExpiry } from '../db/users';

export const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
export const dohUrl = (u: User) => `${config.baseUrl}/b/${u.token}`;

function remaining(d: Date | null): string {
  if (!d) return 'none';
  const ms = d.getTime() - Date.now();
  if (ms <= 0) return 'expired';
  const h = Math.floor(ms / 3600_000);
  return h >= 48 ? `${Math.floor(h / 24)} days ${h % 24}h` : `${h}h ${Math.floor((ms % 3600_000) / 60_000)}m`;
}

export const welcome = (u: User, isNew: boolean) =>
  `👋 <b>${isNew ? 'Welcome' : 'Welcome back'}, ${esc(u.firstName ?? 'player')}!</b>\n\n` +
  (isNew ? `Your <b>${config.trialHours}-hour free trial</b> has started.\n\n` : '') +
  `Your personal DNS link (keep it private — one device at a time):\n<code>${dohUrl(u)}</code>\n\n` +
  `${statusLine(u)}\n\nPick your phone for setup steps 👇\n\n` +
  `<i>Privacy: we only log lookups of rerouted game domains, kept ${config.logRetentionDays} days.</i>`;

export const statusLine = (u: User) => {
  const exp = effectiveExpiry(u);
  const icon = { trial: '🟡', active: '🟢', expired: '🔴', paused: '⏸️' }[u.status] ?? '⚪';
  return `${icon} Status: <b>${u.status}</b>` + (exp ? ` — ${remaining(exp)} left` : '');
};

export const status = (u: User) =>
  `${statusLine(u)}\nNetwork changes seen: ${u.ipChangeCounter}\n` +
  (u.status === 'expired' ? `\nTo renew, message ${esc(config.supportHandle)}.` : '');

export const androidGuide = (u: User) =>
`📱 <b>Android — Intra</b>
1. Install <b>Intra</b> (by Jigsaw) from Google Play.
2. Open Intra → ⚙️ Settings → <b>Select DNS over HTTPS server</b> → <b>Custom server URL</b>.
3. Paste:
<code>${dohUrl(u)}</code>
4. Go back and switch Intra <b>ON</b>. Accept the VPN prompt (it's local — Intra only handles DNS).
5. Restart the game.

Android's built-in "Private DNS" only supports DoT, so use Intra.`;

export const iosGuide = (u: User) =>
`🍎 <b>iPhone — DNSecure</b>
1. Install <b>DNSecure</b> from the App Store.
2. Add a <b>Custom Server</b> → type <b>HTTPS</b> and paste:
<code>${dohUrl(u)}</code>
3. Save, then <b>Enable</b> it and approve the profile in Settings → General → VPN &amp; Device Management.
4. Restart the game.`;

export const help = (admin: boolean) =>
`<b>Commands</b>
/start — your link &amp; trial
/status — time remaining
/setup — Android &amp; iOS guides
/link — show your DNS link
/newtoken — rotate your link (if shared by mistake)
` + (admin ? `\n<b>Admin</b>
/grant &lt;telegram_id&gt; &lt;days&gt;
/pause &lt;telegram_id&gt; · /resume &lt;telegram_id&gt; · /revoke &lt;telegram_id&gt;
/reload — reload routing rules` : '');

export const expiredNotice = () =>
  `⏰ Your access has ended. Message ${esc(config.supportHandle)} to renew — your link stays the same once reactivated.`;
