#!/usr/bin/env bash
set -euo pipefail

WG_IF="${WG_IF:-wg0}"
WG_NET="${WG_NET:-10.77.0.0/24}"
PUBLIC_IF="${PUBLIC_IF:?set PUBLIC_IF to the VPS egress interface}"
RELAY_PORT="${RELAY_PORT:-5521}"
WG_PORT="${WG_PORT:-51820}"

iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
iptables -A INPUT -i lo -j ACCEPT
iptables -A INPUT -p udp --dport "$WG_PORT" -j ACCEPT
iptables -A INPUT -p udp --dport "$RELAY_PORT" -j ACCEPT
iptables -A FORWARD -i "$WG_IF" -o "$PUBLIC_IF" -s "$WG_NET" -p udp -j ACCEPT
iptables -A FORWARD -i "$PUBLIC_IF" -o "$WG_IF" -d "$WG_NET" \
  -p udp -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
iptables -t nat -A POSTROUTING -s "$WG_NET" -o "$PUBLIC_IF" -j MASQUERADE
