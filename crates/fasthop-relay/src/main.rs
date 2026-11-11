use anyhow::{Context, Result};
use bytes::Bytes;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    collections::HashMap,
    env,
    net::{SocketAddr, UdpSocket as StdUdpSocket},
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    sync::mpsc,
    time::{self, Instant},
};

const MAX_DATAGRAM: usize = 65_535;
const DEFAULT_RELAY_BIND: &str = "0.0.0.0:5521";
const DEFAULT_UPSTREAM: &str = "127.0.0.1:10000";
const DEFAULT_SESSION_TIMEOUT: u64 = 60;
const DEFAULT_SOCKET_BUFFER: usize = 4 * 1024 * 1024;
const CHANNEL_CAPACITY: usize = 256;

struct Session {
    packets: mpsc::Sender<Bytes>,
    last_seen: Instant,
}

#[derive(Clone, Copy)]
struct RelayConfig {
    bind: SocketAddr,
    upstream: SocketAddr,
    session_timeout: Duration,
    socket_buffer: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let config = RelayConfig::from_env()?;
    let relay = bind_udp(config.bind, config.socket_buffer)
        .with_context(|| format!("binding relay socket on {}", config.bind))?;
    let relay = std::sync::Arc::new(relay);
    let mut sessions = HashMap::<SocketAddr, Session>::new();
    let mut receive_buffer = [0_u8; MAX_DATAGRAM];
    let mut cleanup = time::interval(Duration::from_secs(5));

    tracing::info!(
        bind = %config.bind,
        upstream = %config.upstream,
        timeout_secs = config.session_timeout.as_secs(),
        "fasthop UDP relay listening"
    );

    loop {
        tokio::select! {
            result = relay.recv_from(&mut receive_buffer) => {
                let (length, client) = result.context("receiving client datagram")?;
                let packet = Bytes::copy_from_slice(&receive_buffer[..length]);

                if let Some(session) = sessions.get_mut(&client) {
                    session.last_seen = Instant::now();
                    if session.packets.send(packet).await.is_err() {
                        sessions.remove(&client);
                    }
                    continue;
                }

                let (packets, receiver) = mpsc::channel(CHANNEL_CAPACITY);
                let upstream = UdpSocket::bind("0.0.0.0:0")
                    .await
                    .context("binding per-client upstream socket")?;
                let local = upstream.local_addr().context("reading upstream socket address")?;
                let relay_socket = relay.clone();
                tokio::spawn(async move {
                    if let Err(error) =
                        run_session(client, upstream, relay_socket, receiver, config.upstream).await
                    {
                        tracing::debug!(%client, %error, %local, "UDP session stopped");
                    }
                });

                let session = Session {
                    packets,
                    last_seen: Instant::now(),
                };
                if session.packets.send(packet).await.is_ok() {
                    sessions.insert(client, session);
                }
            }
            _ = cleanup.tick() => {
                let now = Instant::now();
                sessions.retain(|client, session| {
                    let active = now.duration_since(session.last_seen) <= config.session_timeout;
                    if !active {
                        tracing::debug!(%client, "expired UDP session");
                    }
                    active
                });
            }
        }
    }
}

async fn run_session(
    client: SocketAddr,
    upstream: UdpSocket,
    relay: std::sync::Arc<UdpSocket>,
    mut packets: mpsc::Receiver<Bytes>,
    upstream_addr: SocketAddr,
) -> Result<()> {
    let mut receive_buffer = [0_u8; MAX_DATAGRAM];
    loop {
        tokio::select! {
            packet = packets.recv() => {
                let Some(packet) = packet else { return Ok(()); };
                upstream.send_to(&packet, upstream_addr).await
                    .with_context(|| format!("sending packet to upstream {upstream_addr}"))?;
            }
            result = upstream.recv_from(&mut receive_buffer) => {
                let (length, source) = result.context("receiving upstream datagram")?;
                if source != upstream_addr {
                    continue;
                }
                relay.send_to(&receive_buffer[..length], client).await
                    .with_context(|| format!("sending packet to client {client}"))?;
            }
        }
    }
}

fn bind_udp(address: SocketAddr, buffer_size: usize) -> Result<UdpSocket> {
    let domain = if address.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket =
        Socket::new(domain, Type::DGRAM, Some(Protocol::UDP)).context("creating UDP socket")?;
    socket
        .set_reuse_address(true)
        .context("setting SO_REUSEADDR")?;
    #[cfg(unix)]
    socket
        .set_reuse_port(true)
        .context("setting SO_REUSEPORT")?;
    socket
        .set_recv_buffer_size(buffer_size)
        .context("setting receive buffer")?;
    socket
        .set_send_buffer_size(buffer_size)
        .context("setting send buffer")?;
    socket.bind(&address.into()).context("binding UDP socket")?;
    let socket: StdUdpSocket = socket.into();
    socket
        .set_nonblocking(true)
        .context("setting nonblocking mode")?;
    UdpSocket::from_std(socket).context("converting UDP socket to Tokio")
}

impl RelayConfig {
    fn from_env() -> Result<Self> {
        let bind = parse_addr("RELAY_BIND", DEFAULT_RELAY_BIND)?;
        let upstream = parse_addr("RELAY_UPSTREAM", DEFAULT_UPSTREAM)?;
        let session_timeout = env::var("RELAY_SESSION_TIMEOUT_SECS")
            .unwrap_or_else(|_| DEFAULT_SESSION_TIMEOUT.to_string())
            .parse::<u64>()
            .context("RELAY_SESSION_TIMEOUT_SECS must be an integer")?;
        let socket_buffer = env::var("RELAY_SOCKET_BUFFER")
            .unwrap_or_else(|_| DEFAULT_SOCKET_BUFFER.to_string())
            .parse::<usize>()
            .context("RELAY_SOCKET_BUFFER must be an integer")?;
        anyhow::ensure!(
            session_timeout > 0,
            "RELAY_SESSION_TIMEOUT_SECS must be greater than zero"
        );
        anyhow::ensure!(
            socket_buffer > 0,
            "RELAY_SOCKET_BUFFER must be greater than zero"
        );
        Ok(Self {
            bind,
            upstream,
            session_timeout: Duration::from_secs(session_timeout),
            socket_buffer,
        })
    }
}

fn parse_addr(name: &str, default: &str) -> Result<SocketAddr> {
    env::var(name)
        .unwrap_or_else(|_| default.to_owned())
        .parse()
        .with_context(|| format!("{name} must be an IP:port socket address"))
}
