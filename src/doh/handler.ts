import type { FastifyInstance, FastifyReply, FastifyRequest } from 'fastify';
import { config } from '../config';
import { resolve } from './resolver';
import { checkDevice, isBlocked, lookupToken, recordFailure } from '../utils/tokenCache';
import { queueLog } from '../utils/queryLog';

const TOKEN_RE = /^[A-Za-z0-9_-]{16,64}$/;
type Req = FastifyRequest<{ Params: { token: string }; Querystring: { dns?: string } }>;

async function serve(req: Req, reply: FastifyReply, wire: Buffer | null) {
  const ip = req.ip;
  if (isBlocked(ip)) return reply.code(429).send();

  const { token } = req.params;
  if (!TOKEN_RE.test(token)) { recordFailure(ip); return reply.code(404).send(); }
  const entry = await lookupToken(token);
  if (!entry) { recordFailure(ip); return reply.code(404).send(); }
  if (!entry.valid) return reply.code(403).send();
  if (!checkDevice(token, ip)) return reply.code(429).header('retry-after', String(config.deviceSwitchSeconds)).send();

  if (!wire || wire.length < 12 || wire.length > 4096) return reply.code(400).send();
  const result = await resolve(wire);
  if (!result) return reply.code(400).send();

  if (result.rerouted || config.logAllQueries) {
    queueLog({ userToken: token, domain: (result.name ?? '').slice(0, 255), clientIp: ip, resolvedIp: result.rerouted?.slice(0, 64) ?? null });
  }
  return reply
    .code(200)
    .header('content-type', 'application/dns-message')
    .header('cache-control', `private, max-age=${result.ttl}`)
    .send(result.response);
}

export function registerDoh(app: FastifyInstance) {
  app.addContentTypeParser('application/dns-message', { parseAs: 'buffer', bodyLimit: 4096 },
    (_req, body, done) => done(null, body));

  // RFC 8484 GET: ?dns=<base64url>
  app.get<{ Params: { token: string }; Querystring: { dns?: string } }>('/b/:token', (req, reply) => {
    const dns = req.query.dns;
    return serve(req, reply, dns ? Buffer.from(dns, 'base64url') : null);
  });
  // RFC 8484 POST: binary body
  app.post<{ Params: { token: string }; Querystring: { dns?: string } }>('/b/:token', (req, reply) =>
    serve(req, reply, Buffer.isBuffer(req.body) ? req.body : null));
}
