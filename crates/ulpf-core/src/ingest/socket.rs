use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::parser::UniversalParser;
use crate::schema::ocsf::NetworkActivity;

/// Configuration for Syslog Ingestion Sockets
#[derive(Debug, Clone)]
pub struct IngestConfig {
    pub bind_addr: SocketAddr,
    pub reuse_port: bool,
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub buffer_capacity: usize,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:514".parse().unwrap(),
            reuse_port: true,
            batch_size: 1000,
            batch_timeout: Duration::from_millis(50),
            buffer_capacity: 65536,
        }
    }
}

/// SO_RCVBUF requested on every ingest socket (UDP + TCP listener).
///
/// The kernel default on this machine class is ~208 KiB (212,992 bytes as
/// reported by getsockopt, which doubles the real allocation for
/// bookkeeping). At burst rates the socket task cannot drain a 208 KiB
/// buffer fast enough and the kernel drops datagrams before userspace
/// ever sees them — docs/INGEST_LIMITS.md has the measured knee.
/// 4 MiB fits under the usual rmem_max (4,194,304) so an unprivileged
/// set succeeds; where a machine caps lower, Linux clamps silently and
/// the ingest banner reports the effective value, so the operator never
/// has to guess which buffer they got.
pub const INGEST_RCVBUF_BYTES: usize = 4 * 1024 * 1024;

/// Create a non-blocking UDP socket with SO_REUSEPORT and SO_REUSEADDR enabled
pub fn create_udp_socket(addr: SocketAddr, reuse_port: bool) -> io::Result<UdpSocket> {
    let domain = match addr {
        SocketAddr::V4(_) => Domain::IPV4,
        SocketAddr::V6(_) => Domain::IPV6,
    };
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;

    #[cfg(all(unix, not(target_os = "solaris")))]
    if reuse_port {
        socket.set_reuse_port(true)?;
    }

    // Best-effort: a clamped or default buffer still ingests, just with a
    // lower burst ceiling. The ingest banner prints the effective value
    // (see udp_socket_rcvbuf), so a silent clamp stays visible.
    let _ = socket.set_recv_buffer_size(INGEST_RCVBUF_BYTES);

    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    let std_socket: std::net::UdpSocket = socket.into();
    UdpSocket::from_std(std_socket)
}

/// Effective SO_RCVBUF of a bound UDP socket, as reported by the kernel.
/// Linux doubles the requested value for bookkeeping — compare
/// getsockopt-to-getsockopt, not to /proc/sys/net/core/rmem_default.
pub fn udp_socket_rcvbuf(socket: &UdpSocket) -> io::Result<usize> {
    socket2::SockRef::from(socket).recv_buffer_size()
}

/// Create a non-blocking TCP listener with SO_REUSEPORT and SO_REUSEADDR enabled
pub fn create_tcp_listener(
    addr: SocketAddr,
    reuse_port: bool,
    backlog: i32,
) -> io::Result<TcpListener> {
    let domain = match addr {
        SocketAddr::V4(_) => Domain::IPV4,
        SocketAddr::V6(_) => Domain::IPV6,
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;

    #[cfg(all(unix, not(target_os = "solaris")))]
    if reuse_port {
        socket.set_reuse_port(true)?;
    }

    // Same best-effort tuning as UDP. On Linux an accepted stream
    // inherits the listener's receive buffer, so this one call covers
    // every connection the listener hands out.
    let _ = socket.set_recv_buffer_size(INGEST_RCVBUF_BYTES);

    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    socket.listen(backlog)?;
    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener)
}

/// Effective SO_RCVBUF of a bound TCP listener, as reported by the kernel.
/// Accepted streams inherit this buffer on Linux.
pub fn tcp_listener_rcvbuf(listener: &TcpListener) -> io::Result<usize> {
    socket2::SockRef::from(listener).recv_buffer_size()
}

/// High-throughput UDP Syslog listener
pub struct UdpSyslogListener {
    socket: UdpSocket,
    config: IngestConfig,
}

impl UdpSyslogListener {
    pub fn bind(config: IngestConfig) -> io::Result<Self> {
        let socket = create_udp_socket(config.bind_addr, config.reuse_port)?;
        Ok(Self { socket, config })
    }

    pub fn from_socket(socket: UdpSocket, config: IngestConfig) -> Self {
        Self { socket, config }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Run the UDP packet receiver and forward batches of raw string lines to a channel
    pub async fn run_raw(self, tx: mpsc::Sender<Vec<String>>) -> anyhow::Result<()> {
        let mut buf = vec![0u8; self.config.buffer_capacity];
        let mut current_batch = Vec::with_capacity(self.config.batch_size);
        let mut last_flush = tokio::time::Instant::now();

        loop {
            tokio::select! {
                res = self.socket.recv_from(&mut buf) => {
                    match res {
                        Ok((size, _peer)) => {
                            let payload = &buf[..size];
                            if let Ok(text) = std::str::from_utf8(payload) {
                                for line in text.lines() {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        current_batch.push(trimmed.to_string());
                                        if current_batch.len() >= self.config.batch_size {
                                            let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(self.config.batch_size));
                                            if tx.send(batch).await.is_err() {
                                                return Ok(());
                                            }
                                            last_flush = tokio::time::Instant::now();
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("UDP recv_from error: {:?}", e);
                        }
                    }
                }
                _ = sleep(self.config.batch_timeout) => {
                    if !current_batch.is_empty() && last_flush.elapsed() >= self.config.batch_timeout {
                        let batch = std::mem::replace(&mut current_batch, Vec::with_capacity(self.config.batch_size));
                        if tx.send(batch).await.is_err() {
                            return Ok(());
                        }
                        last_flush = tokio::time::Instant::now();
                    }
                }
            }
        }
    }

    /// Run the UDP packet receiver, parsing logs inline with UniversalParser
    /// and forwarding normalized OCSF events to the processing channel
    pub async fn run_parsed(
        self,
        parser: Arc<UniversalParser>,
        tx: mpsc::Sender<Vec<NetworkActivity>>,
    ) -> anyhow::Result<()> {
        let (raw_tx, mut raw_rx) = mpsc::channel::<Vec<String>>(100);

        tokio::spawn(async move {
            let _ = self.run_raw(raw_tx).await;
        });

        while let Some(raw_batch) = raw_rx.recv().await {
            let mut parsed_batch = Vec::with_capacity(raw_batch.len());
            for raw_line in raw_batch {
                let activity = parser.parse_lossless(&raw_line);
                parsed_batch.push(activity);
            }
            if tx.send(parsed_batch).await.is_err() {
                break;
            }
        }

        Ok(())
    }
}

/// High-throughput TCP Syslog listener
pub struct TcpSyslogListener {
    listener: TcpListener,
    config: IngestConfig,
}

impl TcpSyslogListener {
    pub fn bind(config: IngestConfig) -> io::Result<Self> {
        let listener = create_tcp_listener(config.bind_addr, config.reuse_port, 1024)?;
        Ok(Self { listener, config })
    }

    pub fn from_listener(listener: TcpListener, config: IngestConfig) -> Self {
        Self { listener, config }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Run TCP connection acceptor, spawning connection reader tasks
    pub async fn run_raw(self, tx: mpsc::Sender<Vec<String>>) -> anyhow::Result<()> {
        let batch_size = self.config.batch_size;
        let batch_timeout = self.config.batch_timeout;

        loop {
            match self.listener.accept().await {
                Ok((stream, _peer)) => {
                    let tx_conn = tx.clone();
                    tokio::spawn(async move {
                        let reader = BufReader::new(stream);
                        let mut lines = reader.lines();
                        let mut batch = Vec::with_capacity(batch_size);
                        let mut last_flush = tokio::time::Instant::now();

                        while let Ok(Some(line)) = lines.next_line().await {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                batch.push(trimmed.to_string());
                                if batch.len() >= batch_size
                                    || last_flush.elapsed() >= batch_timeout
                                {
                                    let to_send = std::mem::replace(
                                        &mut batch,
                                        Vec::with_capacity(batch_size),
                                    );
                                    if tx_conn.send(to_send).await.is_err() {
                                        break;
                                    }
                                    last_flush = tokio::time::Instant::now();
                                }
                            }
                        }

                        if !batch.is_empty() {
                            let _ = tx_conn.send(batch).await;
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("TCP accept error: {:?}", e);
                }
            }
        }
    }
}

/// Spawns a multi-worker UDP listener pool using SO_REUSEPORT across worker threads
pub fn spawn_udp_worker_pool(
    config: IngestConfig,
    num_workers: usize,
    parser: Arc<UniversalParser>,
    tx: mpsc::Sender<Vec<NetworkActivity>>,
) -> io::Result<Vec<tokio::task::JoinHandle<()>>> {
    let mut handles = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let listener = UdpSyslogListener::bind(config.clone())?;
        let parser_clone = Arc::clone(&parser);
        let tx_clone = tx.clone();

        let handle = tokio::spawn(async move {
            let _ = listener.run_parsed(parser_clone, tx_clone).await;
        });

        handles.push(handle);
    }

    Ok(handles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_so_reuseport_udp_creation() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let socket1 = create_udp_socket(addr, true).unwrap();
        let local_port = socket1.local_addr().unwrap().port();

        // Bind another socket to the same local port using SO_REUSEPORT
        let target_addr: SocketAddr = format!("127.0.0.1:{}", local_port).parse().unwrap();
        let socket2 = create_udp_socket(target_addr, true);
        assert!(socket2.is_ok(), "SO_REUSEPORT binding should succeed");
    }

    #[tokio::test]
    async fn test_ingest_sockets_request_tuned_rcvbuf() {
        // create_*_socket must apply INGEST_RCVBUF_BYTES. The kernel
        // reports double the allocation for bookkeeping and may clamp to
        // rmem_max, so instead of asserting an absolute number, compare
        // against a probe socket given the identical request on the same
        // machine: same request, same clamp, same readback.
        let probe = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
        let stock = probe.recv_buffer_size().unwrap();
        probe.set_recv_buffer_size(INGEST_RCVBUF_BYTES).unwrap();
        let expected = probe.recv_buffer_size().unwrap();
        assert!(
            expected >= stock,
            "probe sanity: requesting {INGEST_RCVBUF_BYTES} must not shrink the buffer"
        );

        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let tuned = create_udp_socket(addr, false).unwrap();
        let got = udp_socket_rcvbuf(&tuned).unwrap();
        assert_eq!(
            got, expected,
            "create_udp_socket must request INGEST_RCVBUF_BYTES ({INGEST_RCVBUF_BYTES})"
        );

        let listener = create_tcp_listener(addr, false, 16).unwrap();
        let tcp_got = tcp_listener_rcvbuf(&listener).unwrap();
        assert!(
            tcp_got >= stock,
            "tuned TCP rcvbuf ({tcp_got}) must be >= stock default ({stock})"
        );
    }

    #[tokio::test]
    async fn test_udp_ingest_pipeline() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut config = IngestConfig::default();
        config.bind_addr = addr;
        config.batch_size = 2;
        config.batch_timeout = Duration::from_millis(10);

        let listener = UdpSyslogListener::bind(config).unwrap();
        let actual_addr = listener.local_addr().unwrap();

        let (tx, mut rx) = mpsc::channel(10);
        let parser = Arc::new(UniversalParser::new());

        let server_handle = tokio::spawn(async move {
            let _ = listener.run_parsed(parser, tx).await;
        });

        // Send test UDP packet
        let sender = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let log_line = "%ASA-6-302013: Built inbound UDP connection 123 for outside:1.1.1.1/53 to inside:2.2.2.2/53\n";
        sender
            .send_to(log_line.as_bytes(), actual_addr)
            .await
            .unwrap();

        let received = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
        assert!(received.is_ok(), "Should receive batch within timeout");
        let batch = received.unwrap().expect("Batch was None");
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].src_endpoint.ip.as_deref(), Some("1.1.1.1"));

        server_handle.abort();
    }
}
