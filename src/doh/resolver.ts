import dnsPacket from 'dns-packet';
import { config } from '../config';
import { matchRule } from './routingRules';

export interface ResolveResult {
  response: Buffer;
  ttl: number;
  name?: string;
  rerouted?: string; // comma-joined IPs when we answered locally
}

const RD_RA = dnsPacket.RECURSION_DESIRED | dnsPacket.RECURSION_AVAILABLE;

const cache = new Map<string, { buf: Buffer; exp: number; ttl: number }>();
const MAX_CACHE = 5000;

const RCODE = { NOERROR: 0, SERVFAIL: 2 } as const;

// dns-packet takes the RCODE from the low 4 bits of `flags`.
function synth(id: number, questions: any[], answers: any[], rcode: keyof typeof RCODE = 'NOERROR'): Buffer {
  return dnsPacket.encode({ type: 'response', id, flags: RD_RA | RCODE[rcode], questions, answers } as any);
}

async function forward(query: Buffer): Promise<Buffer> {
  for (const url of config.upstreams) {
    try {
      const r = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/dns-message', accept: 'application/dns-message' },
        body: new Uint8Array(query),
        signal: AbortSignal.timeout(config.upstreamTimeoutMs),
      });
      if (r.ok) return Buffer.from(await r.arrayBuffer());
    } catch { /* try next upstream */ }
  }
  throw new Error('all upstreams failed');
}

function responseTtl(buf: Buffer): number | null {
  try {
    const p: any = dnsPacket.decode(buf);
    if (p.rcode === 'SERVFAIL' || p.rcode === 'REFUSED') return null;
    const ttls = [...(p.answers ?? []), ...(p.authorities ?? [])].map((r: any) => r.ttl).filter((t: any) => typeof t === 'number');
    return Math.min(120, Math.max(5, ttls.length ? Math.min(...ttls) : 30));
  } catch { return null; }
}

export async function resolve(query: Buffer): Promise<ResolveResult | null> {
  let msg: any;
  try { msg = dnsPacket.decode(query); } catch { return null; }
  if (msg.type !== 'query' || !msg.questions?.length) return null;
  const q = msg.questions[0];

  // 1) Local interception for configured game domains.
  if (q.type === 'A' || q.type === 'AAAA') {
    const rule = matchRule(q.name);
    if (rule) {
      if (q.type === 'A') {
        const answers = rule.ips.map((ip) => ({ type: 'A', name: q.name, class: 'IN', ttl: rule.ttl, data: ip }));
        return { response: synth(msg.id, msg.questions, answers), ttl: rule.ttl, name: q.name, rerouted: rule.ips.join(',') };
      }
      if (rule.blockIpv6) return { response: synth(msg.id, msg.questions, []), ttl: rule.ttl, name: q.name, rerouted: 'AAAA-suppressed' };
    }
  }

  // 2) Upstream with small TTL cache (key = query bytes minus the 2-byte ID).
  const key = query.subarray(2).toString('base64');
  const now = Date.now();
  const hit = cache.get(key);
  if (hit && hit.exp > now) {
    const out = Buffer.from(hit.buf);
    out.writeUInt16BE(msg.id, 0);
    return { response: out, ttl: Math.max(1, Math.round((hit.exp - now) / 1000)), name: q.name };
  }
  try {
    const buf = await forward(query);
    const ttl = responseTtl(buf);
    if (ttl !== null) {
      if (cache.size >= MAX_CACHE) cache.delete(cache.keys().next().value!);
      cache.set(key, { buf, exp: now + ttl * 1000, ttl });
    }
    return { response: buf, ttl: ttl ?? 0, name: q.name };
  } catch {
    return { response: synth(msg.id, msg.questions, [], 'SERVFAIL'), ttl: 0, name: q.name };
  }
}
