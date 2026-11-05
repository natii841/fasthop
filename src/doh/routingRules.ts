import { readFileSync, existsSync } from 'node:fs';
import { isIPv4 } from 'node:net';
import { config } from '../config';

export interface Rule {
  /** Exact name ("mm.example.com") or wildcard ("*.example.com", matches subdomains only). */
  domain: string;
  /** IPv4 endpoints returned for A queries. YOU supply these from your own latency measurements. */
  ips: string[];
  ttl?: number;
  /** Answer AAAA with empty NOERROR so clients stay on the IPv4 path (default true). */
  blockIpv6?: boolean;
}
export interface CompiledRule { ips: string[]; ttl: number; blockIpv6: boolean }

let exact = new Map<string, CompiledRule>();
let wild = new Map<string, CompiledRule>();

export function loadRules(): number {
  let raw: string | undefined = config.routingJson;
  if (!raw && existsSync(config.routingFile)) raw = readFileSync(config.routingFile, 'utf8');
  const e = new Map<string, CompiledRule>(), w = new Map<string, CompiledRule>();
  if (raw) {
    for (const r of JSON.parse(raw) as Rule[]) {
      const ips = (r.ips ?? []).filter(isIPv4);
      if (!r.domain || !ips.length) { console.warn(`[routing] skipping invalid rule: ${r.domain}`); continue; }
      const c = { ips, ttl: r.ttl ?? 30, blockIpv6: r.blockIpv6 ?? true };
      const d = r.domain.toLowerCase().replace(/\.$/, '');
      if (d.startsWith('*.')) w.set(d.slice(2), c); else e.set(d, c);
    }
  }
  exact = e; wild = w;
  return e.size + w.size;
}

export function matchRule(name: string): CompiledRule | undefined {
  const n = name.toLowerCase().replace(/\.$/, '');
  const hit = exact.get(n);
  if (hit) return hit;
  const parts = n.split('.');
  for (let i = 1; i < parts.length; i++) {
    const w = wild.get(parts.slice(i).join('.'));
    if (w) return w;
  }
  return undefined;
}
