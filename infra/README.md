# FastHop relay VPS deployment

The relay is deliberately separate from the Railway DoH/dashboard service.
Run it on a UDP-capable VPS close to the selected game region. `fasthop-relay`
accepts client datagrams on `RELAY_BIND`, gives each client address an ephemeral
upstream socket, and forwards replies only from `RELAY_UPSTREAM`. It does not
inspect, rewrite, compress, or buffer gameplay packets.

## Build and install

```bash
sudo useradd --system --home /opt/fasthop-relay --shell /usr/sbin/nologin fasthop
sudo install -d -o fasthop -g fasthop /opt/fasthop-relay /etc/fasthop
cargo build --release -p fasthop-relay
sudo install -o fasthop -g fasthop target/release/fasthop-relay /opt/fasthop-relay/
sudo install -o root -g root infra/systemd/fasthop-relay.service /etc/systemd/system/
sudo install -o root -g root infra/sysctl.d/99-fasthop-relay.conf /etc/sysctl.d/
sudo sysctl --system
sudo systemctl daemon-reload
sudo systemctl enable --now fasthop-relay
```

Copy `.env.relay.example` to `/etc/fasthop/relay.env`, set a real upstream
address, and restrict file permissions with `chmod 600`.

## WireGuard and firewall

Use the client template in `infra/wireguard/client.conf.example`. Do not add
`0.0.0.0/0` or `::/0`: only measured game endpoint CIDRs belong in
`AllowedIPs`. Google Cloud/AWS Global Accelerator addresses are not inherently
game endpoints, so include them only after verifying the game's resolved
addresses and provider terms.

On a VPS already configured with `wg0`, enable forwarding and apply the exact
minimal UDP rules below (set `PUBLIC_IF` first):

```bash
sudo install -m 0750 infra/iptables-relay.sh /usr/local/sbin/fasthop-iptables
sudo PUBLIC_IF=eth0 /usr/local/sbin/fasthop-iptables
```

Persist rules using the distribution's `iptables-persistent`/`netfilter-persistent`
mechanism. The relay's direct public listener needs UDP/5521; WireGuard needs
UDP/51820. No TCP forwarding is required.

## Measurement checklist

1. Compare each candidate VPS with `mtr -u -r -c 100 <game-endpoint>` and record
   median, worst latency, and loss.
2. Measure raw UDP capacity and loss with
   `iperf3 -c <vps> -u -b 10M -t 60 -J > iperf.json` (run `iperf3 -s` on the VPS).
3. Capture both sides with
   `sudo tcpdump -ni any 'udp port 5521 or udp port 10000' -w relay.pcap`;
   inspect sequence timing and retransmission/application behavior in Wireshark.
4. Run the game through the WireGuard profile and compare 100+ match samples,
   not a single ping. Record RTT median/p95, jitter, and loss separately.
5. Watch `journalctl -u fasthop-relay -f`; an expired session should disappear
   after `RELAY_SESSION_TIMEOUT_SECS` without affecting other clients.

The relay cannot fix a game server's matchmaking decision or a route that is
already optimal. Re-test endpoint CIDRs after game updates and provider changes.
