# FastHop DoH & Telegram Bot

Token-gated DNS-over-HTTPS (RFC 8484) resolver + Telegram subscription bot.

- `GET|POST https://<domain>/b/<token>` — GET uses `?dns=<base64url>`, POST uses `application/dns-message`.
- Domains in your routing rules get answers you define; everything else is proxied to Cloudflare, falling back to Google.
- Access = `trial` (5h by default) or `active` (paid) and unexpired. Expired/paused tokens get 403.
- One device at a time: a second network (IPv4 /24 or IPv6 /64) is refused with 429 while the first is active. Roaming works: after 120s of silence a new network takes over.

## Run
    cp .env.example .env && cp routing.example.json routing.json   # edit both
    npm install && npm run dev
Docker: `docker build -t fasthop . && docker run --env-file .env -p 8080:8080 fasthop`

DoH **requires valid HTTPS** — put it behind Railway's domain, Caddy or nginx. `trustProxy` is on so the real client IP is read from `X-Forwarded-For`.

## Deploy on Railway

1. Create a new Railway project and deploy this repository. Railway will use
   [`railway.toml`](./railway.toml) and the included [`Dockerfile`](./Dockerfile).
2. Add a **PostgreSQL** service to the project. Set the application service's
   `DATABASE_URL` to the PostgreSQL service's `DATABASE_URL` reference
   (in Railway variables, use `${{Postgres.DATABASE_URL}}`; use your database
   service's actual name if it is different).
3. Add these required variables to the application service:

   - `BOT_TOKEN`: token from BotFather.
   - `DATABASE_URL`: Railway PostgreSQL connection string.
   - `BASE_URL`: the public HTTPS URL of this service, without a trailing slash.
   - `ADMIN_IDS`: comma-separated Telegram user IDs for bot administrators.
   - `ROUTING_JSON`: JSON array containing your production routing rules. Start
     from [`routing.example.json`](./routing.example.json), replace its example
     values, and paste the resulting one-line JSON into Railway.

   `PORT` is provided by Railway automatically. Leave `DATABASE_SSL=false` when
   the application and PostgreSQL services are in the same Railway project.
   Set it to `true` only when your PostgreSQL provider requires SSL.
4. Generate a Railway public domain in **Settings → Networking → Public
   Networking**, then set `BASE_URL` to that `https://` URL and redeploy.
   Confirm that `https://<domain>/healthz` returns `{"ok":true}`.
5. Keep `WEBHOOK_URL` empty for the default Telegram long-polling mode. If you
   use webhooks instead, set `WEBHOOK_URL` to the same public HTTPS origin and
   set `WEBHOOK_SECRET` to a long random value. The bot webhook will then be
   available at `/tg/<WEBHOOK_SECRET>`.

The service creates its tables automatically on first startup. Do not commit
`.env` or `routing.json`; both are ignored, and Railway should receive their
values through variables. Railway's generated domain provides HTTPS, which is
required by DoH clients.

## Admin (Telegram, IDs in `ADMIN_IDS`)
`/grant <id> <days>` · `/pause` · `/resume` · `/revoke` · `/reload`. `kill -HUP` also reloads routing.
Payments are manual; to automate (e.g. M-Pesa Daraja), call `grantDays(telegramId, days)` from your payment callback.

## Notes
- **You must supply the routing IPs/domains** (`routing.json`). Nothing here knows eFootball's real hostnames or good endpoints; measure them yourself. Rerouting only helps if the endpoint actually accepts those clients — test before selling.
- Single-instance by design (device-binding state is in memory). Scale out → move `sessions` to Redis.
- Logs: only rerouted domains by default, pruned after 7 days. Tell users (the bot does).
