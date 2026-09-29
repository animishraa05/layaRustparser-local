use anyhow::{Context, Result};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};

use ulpf_generator::{load_dataset, locate_data_dir, DatasetKind, Protocol};

#[derive(Parser, Debug)]
#[command(
    name = "ulpf-generator",
    about = "ULPF Multi-Threaded High-Speed Syslog Traffic Generator (10k - 500k+ EPS)"
)]
struct Cli {
    /// Target destination in IP:PORT format
    #[arg(short, long, default_value = "127.0.0.1:5140")]
    target: String,

    /// Transport protocol: udp or tcp
    #[arg(short, long, default_value = "udp")]
    proto: String,

    /// Target rate in Events Per Second (EPS). Set to 0 for unthrottled maximum speed.
    #[arg(short, long, default_value_t = 50000)]
    rate: u64,

    /// Duration to generate traffic in seconds (0 for infinite)
    #[arg(short, long, default_value_t = 10)]
    duration: u64,

    /// Dataset to stream: all, cisco, fortigate, paloalto, suricata, pfsense, kaggle
    #[arg(short = 'D', long, default_value = "all")]
    dataset: String,

    /// Number of concurrent worker tasks (defaults to number of logical CPU cores)
    #[arg(short, long)]
    workers: Option<usize>,

    /// Path to data/raw directory containing log files
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Batch size of packets dispatched per worker loop iteration
    #[arg(long, default_value_t = 64)]
    batch_size: usize,
}

struct Stats {
    packets_sent: AtomicU64,
    bytes_sent: AtomicU64,
    errors: AtomicU64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let proto: Protocol = cli.proto.parse()?;
    let dataset_kind: DatasetKind = cli.dataset.parse()?;
    let target_addr: SocketAddr = cli
        .target
        .parse()
        .with_context(|| format!("Invalid target socket address '{}'", cli.target))?;

    let num_workers = cli.workers.unwrap_or_else(|| {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        cpus.max(2)
    });

    println!("============================================================");
    println!(" ULPF High-Speed Syslog Traffic Generator");
    println!("============================================================");
    println!("  Target:      {} ({})", target_addr, proto);
    println!("  Dataset:     {}", dataset_kind);
    if cli.rate == 0 {
        println!("  Target Rate: UNTHROTTLED (MAX EPS)");
    } else {
        println!(
            "  Target Rate: {} EPS ({} pkts/sec across {} workers)",
            cli.rate, cli.rate, num_workers
        );
    }
    if cli.duration == 0 {
        println!("  Duration:    Infinite (Press Ctrl+C to stop)");
    } else {
        println!("  Duration:    {} seconds", cli.duration);
    }
    println!("  Workers:     {}", num_workers);
    println!("  Batch Size:  {}", cli.batch_size);

    // Locate data directory and load logs into memory
    let data_dir = locate_data_dir(cli.data_dir.as_deref())?;
    println!("  Data Path:   {:?}", data_dir);
    println!("Loading datasets into memory buffer...");
    let logs_str = load_dataset(dataset_kind, &data_dir)?;
    println!(
        "  [✓] Buffered {} unique log lines in RAM (Zero-Disk-IO during blast)",
        logs_str.len()
    );

    // Convert to Vec<Arc<[u8]>> or pre-encoded byte buffers
    let logs: Arc<Vec<Vec<u8>>> = Arc::new(
        logs_str
            .into_iter()
            .map(|s| {
                let mut b = s.into_bytes();
                if proto == Protocol::Tcp {
                    b.push(b'\n');
                }
                b
            })
            .collect(),
    );

    let stats = Arc::new(Stats {
        packets_sent: AtomicU64::new(0),
        bytes_sent: AtomicU64::new(0),
        errors: AtomicU64::new(0),
    });

    let running = Arc::new(AtomicBool::new(true));

    // Handle Ctrl+C gracefully
    let running_ctrlc = running.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        println!("\n[!] Received Ctrl+C, shutting down generator...");
        running_ctrlc.store(false, Ordering::SeqCst);
    });

    println!("Blasting traffic to {}...", target_addr);

    // Rate calculations per worker
    let worker_rate = if cli.rate > 0 {
        (cli.rate / num_workers as u64).max(1)
    } else {
        0
    };

    let start_time = Instant::now();
    let mut worker_handles = Vec::with_capacity(num_workers);

    for worker_id in 0..num_workers {
        let logs_clone = logs.clone();
        let stats_clone = stats.clone();
        let running_clone = running.clone();
        let batch_sz = cli.batch_size;

        let handle = tokio::spawn(async move {
            match proto {
                Protocol::Udp => {
                    run_udp_worker(
                        worker_id,
                        target_addr,
                        logs_clone,
                        stats_clone,
                        running_clone,
                        worker_rate,
                        batch_sz,
                    )
                    .await;
                }
                Protocol::Tcp => {
                    run_tcp_worker(
                        worker_id,
                        target_addr,
                        logs_clone,
                        stats_clone,
                        running_clone,
                        worker_rate,
                        batch_sz,
                    )
                    .await;
                }
            }
        });
        worker_handles.push(handle);
    }

    // Reporter task
    let stats_reporter = stats.clone();
    let running_reporter = running.clone();
    let duration_secs = cli.duration;

    let reporter_handle = tokio::spawn(async move {
        let mut prev_pkts = 0u64;
        let mut prev_bytes = 0u64;
        let mut second = 0u64;
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.tick().await; // first tick fires immediately

        while running_reporter.load(Ordering::Relaxed) {
            interval.tick().await;
            second += 1;

            let cur_pkts = stats_reporter.packets_sent.load(Ordering::Relaxed);
            let cur_bytes = stats_reporter.bytes_sent.load(Ordering::Relaxed);
            let cur_errors = stats_reporter.errors.load(Ordering::Relaxed);

            let delta_pkts = cur_pkts.saturating_sub(prev_pkts);
            let delta_bytes = cur_bytes.saturating_sub(prev_bytes);
            prev_pkts = cur_pkts;
            prev_bytes = cur_bytes;

            let mbytes_per_sec = (delta_bytes as f64) / (1024.0 * 1024.0);
            let total_mbytes = (cur_bytes as f64) / (1024.0 * 1024.0);

            println!(
                "[{:02}:{:02}] Rate: {:>7} pkts/s (EPS) | Bandwidth: {:>6.2} MB/s | Total: {:>9} pkts ({:>6.2} MB) | Errors: {}",
                second / 60,
                second % 60,
                delta_pkts,
                mbytes_per_sec,
                cur_pkts,
                total_mbytes,
                cur_errors
            );

            if duration_secs > 0 && second >= duration_secs {
                running_reporter.store(false, Ordering::SeqCst);
                break;
            }
        }
    });

    // Wait for reporter to finish (which signals running = false)
    let _ = reporter_handle.await;

    // Await all workers
    for h in worker_handles {
        let _ = h.await;
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let total_pkts = stats.packets_sent.load(Ordering::Relaxed);
    let total_bytes = stats.bytes_sent.load(Ordering::Relaxed);
    let total_errors = stats.errors.load(Ordering::Relaxed);
    let avg_eps = if elapsed > 0.0 {
        (total_pkts as f64 / elapsed) as u64
    } else {
        0
    };
    let avg_mb_sec = if elapsed > 0.0 {
        (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed
    } else {
        0.0
    };

    println!("\n============================================================");
    println!(" ULPF Generator Run Finished");
    println!("============================================================");
    println!("  Target:            {} ({})", target_addr, proto);
    println!("  Dataset:           {}", dataset_kind);
    println!("  Elapsed Time:      {:.2} seconds", elapsed);
    println!("  Total Packets:     {} pkts", total_pkts);
    println!(
        "  Total Data Sent:   {:.2} MB ({} bytes)",
        (total_bytes as f64) / (1024.0 * 1024.0),
        total_bytes
    );
    println!("  Average Rate:      {} pkts/sec (EPS)", avg_eps);
    println!("  Average Bandwidth: {:.2} MB/sec", avg_mb_sec);
    println!("  Total Errors:      {}", total_errors);
    println!("============================================================");

    Ok(())
}

/// Returns how many packets to send this batch, sleeping first if the worker
/// is ahead of its target rate.
///
/// Previously this slept a fixed 500us and re-polled, tying pacing resolution
/// to the send path: the worker burned loop trips just to re-check the clock.
/// Sleeping once until the exact deadline the deficit implies decouples the
/// two — one timer wait per batch instead of a poll loop.
async fn next_batch_size(
    sent_count: u64,
    worker_start: Instant,
    target_rate: u64,
    batch_size: usize,
) -> usize {
    if target_rate == 0 {
        return batch_size;
    }
    let expected = worker_start.elapsed().as_secs_f64() * target_rate as f64;
    if sent_count as f64 > expected + batch_size as f64 {
        let ahead_secs = (sent_count as f64 - expected) / target_rate as f64;
        let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(ahead_secs.max(0.0));
        tokio::time::sleep_until(deadline).await;
        return 0; // re-evaluate after the sleep; send nothing this trip
    }
    batch_size.min((expected as u64).saturating_sub(sent_count).max(1) as usize)
}

/// Collects the next `count` datagram payloads starting at `start_idx`,
/// wrapping around the corpus. Shared by the `sendmmsg` fast path and the
/// per-packet fallback so both emit identical wire bytes: 1 log = 1 datagram.
fn collect_batch(logs: &[Vec<u8>], start_idx: usize, count: usize) -> Vec<&[u8]> {
    let n = logs.len();
    (0..count)
        .map(|i| logs[(start_idx + i) % n].as_slice())
        .collect()
}

/// Sends one batch of datagrams, returning (packets_sent, bytes_sent, errors).
///
/// NOTE on `--batch-size`: before this change it only bounded how many loop
/// trips (individual `send()` syscalls) a worker did per iteration — it never
/// coalesced syscalls. The Linux path below finally makes it a real batch:
/// one `sendmmsg` syscall per batch. GSO (`UDP_SEGMENT`) was considered and
/// REJECTED: it splits one buffer into fixed-size segments, but corpus lines
/// vary in length by vendor, so GSO would need padding (changing wire bytes
/// and breaking benchmark comparability) or fail outright. `sendmmsg` keeps
/// 1 log = 1 datagram byte-for-byte.
async fn send_udp_batch(socket: &UdpSocket, batch: &[&[u8]]) -> (u64, u64, u64) {
    #[cfg(target_os = "linux")]
    {
        send_udp_batch_sendmmsg(socket, batch).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        send_udp_batch_one_by_one(socket, batch).await
    }
}

/// Per-packet fallback: one `send()` per datagram. Used on non-Linux and
/// when `sendmmsg` hits a per-datagram error mid-batch.
async fn send_udp_batch_one_by_one(socket: &UdpSocket, batch: &[&[u8]]) -> (u64, u64, u64) {
    let mut pkts = 0u64;
    let mut bytes = 0u64;
    let mut errors = 0u64;
    for payload in batch {
        match socket.send(payload).await {
            Ok(n) => {
                pkts += 1;
                bytes += n as u64;
            }
            Err(_) => errors += 1,
        }
    }
    (pkts, bytes, errors)
}

/// Outcome of one `sendmmsg` syscall. Owns no pointers, so the async retry
/// loop can match on it without holding raw `iovec` borrows across `.await`.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MmsgOutcome {
    Sent { count: usize, bytes: u64 },
    Empty,
    WaitWritable,
    Retry,
    FallbackRemainder,
}

/// Owned `sendmmsg` scratch buffers: one iovec + one header per datagram.
/// The raw pointers inside make the plain `Vec`s `!Send`, which would poison
/// the worker future spawned on the multi-thread runtime. Wrapped so the
/// single owner (this worker task) can hold them across `.await` points.
/// Sound: the pointers target this struct's own iovec allocation and log
/// bytes borrowed for the batch, and no other thread can ever observe them —
/// `Send` only moves ownership, and the pointers stay valid wherever the
/// owner runs the syscall.
#[cfg(target_os = "linux")]
struct MmsgBufs {
    iovecs: Vec<libc::iovec>,
    headers: Vec<libc::mmsghdr>,
}

#[cfg(target_os = "linux")]
unsafe impl Send for MmsgBufs {}

/// Builds the `sendmmsg` scratch buffers for a batch. Iovecs borrow the log
/// buffers so payloads are never copied; headers are filled after the iovec
/// allocation is complete so no reallocation can dangle their pointers.
#[cfg(target_os = "linux")]
fn mmsg_bufs(batch: &[&[u8]]) -> MmsgBufs {
    let mut bufs = MmsgBufs {
        iovecs: batch
            .iter()
            .map(|b| libc::iovec {
                iov_base: b.as_ptr() as *mut libc::c_void,
                iov_len: b.len(),
            })
            .collect(),
        headers: Vec::with_capacity(batch.len()),
    };
    for iov in bufs.iovecs.iter_mut() {
        let mut hdr: libc::mmsghdr = unsafe { std::mem::zeroed() };
        // Connected socket: no per-message address needed.
        hdr.msg_hdr.msg_iov = iov;
        hdr.msg_hdr.msg_iovlen = 1;
        bufs.headers.push(hdr);
    }
    bufs
}

#[cfg(target_os = "linux")]
fn sendmmsg_once(fd: libc::c_int, headers: &mut [libc::mmsghdr]) -> MmsgOutcome {
    // SAFETY: caller guarantees `headers` points at live log buffers and
    // `fd` is the worker's connected UDP socket. The borrow ends on return.
    let ret = unsafe { libc::sendmmsg(fd, headers.as_mut_ptr(), headers.len() as libc::c_uint, 0) };
    if ret < 0 {
        return match std::io::Error::last_os_error().kind() {
            std::io::ErrorKind::Interrupted => MmsgOutcome::Retry,
            std::io::ErrorKind::WouldBlock => MmsgOutcome::WaitWritable,
            _ => MmsgOutcome::FallbackRemainder,
        };
    }
    if ret == 0 {
        return MmsgOutcome::Empty;
    }
    let n = ret as usize;
    let mut bytes = 0u64;
    for hdr in headers.iter().take(n) {
        bytes += hdr.msg_len as u64;
    }
    MmsgOutcome::Sent { count: n, bytes }
}

/// Batched send via `sendmmsg` on the tokio socket's fd. The socket stays
/// nonblocking: EINTR retries inline, EAGAIN waits for writability, and a
/// partial return resumes at the first unsent datagram.
#[cfg(target_os = "linux")]
async fn send_udp_batch_sendmmsg(socket: &UdpSocket, batch: &[&[u8]]) -> (u64, u64, u64) {
    use std::os::unix::io::AsRawFd;

    if batch.is_empty() {
        return (0, 0, 0);
    }
    let mut transport = SocketTransport {
        fd: socket.as_raw_fd(),
        socket,
        batch,
        // One build per batch; partial sends resume at `sent` without rebuilding.
        bufs: mmsg_bufs(batch),
    };
    mmsg_drive_loop(batch.len(), &mut transport).await
}

/// Seam between the `sendmmsg` retry loop and the socket. The production
/// impl below drives the real fd; tests inject a scripted fake so
/// partial-send and persistent-EAGAIN transitions are covered without
/// touching the network.
#[cfg(target_os = "linux")]
trait MmsgTransport {
    fn send_step(&mut self, sent: usize) -> MmsgOutcome;
    fn wait_writable(&mut self) -> impl std::future::Future<Output = bool> + Send;
    fn fallback_from(
        &mut self,
        sent: usize,
    ) -> impl std::future::Future<Output = (u64, u64, u64)> + Send;
}

/// Shared retry loop over `MmsgOutcome`s. Counts every datagram exactly
/// once: `sent` successes stay in the total even when the remainder falls
/// back to per-packet sends after a hard error, and every EAGAIN is gated
/// on one transport wait so the loop can never busy-spin the syscall.
#[cfg(target_os = "linux")]
async fn mmsg_drive_loop(batch_len: usize, transport: &mut impl MmsgTransport) -> (u64, u64, u64) {
    let mut sent = 0usize;
    let mut bytes = 0u64;
    let mut errors = 0u64;
    while sent < batch_len {
        match transport.send_step(sent) {
            MmsgOutcome::Retry => continue,
            MmsgOutcome::Empty => {
                // Should not happen for datagrams; yield, don't hot-spin.
                tokio::task::yield_now().await;
            }
            MmsgOutcome::WaitWritable => {
                if !transport.wait_writable().await {
                    errors += (batch_len - sent) as u64;
                    break;
                }
            }
            // e.g. EMSGSIZE for one oversize datagram aborts the whole call,
            // so drain the rest one by one like before.
            MmsgOutcome::FallbackRemainder => {
                let (p, b, e) = transport.fallback_from(sent).await;
                return (sent as u64 + p, bytes + b, errors + e);
            }
            MmsgOutcome::Sent { count, bytes: n } => {
                bytes += n;
                sent += count;
            }
        }
    }
    (sent as u64, bytes, errors)
}

/// Production transport: raw `sendmmsg` on the tokio socket's fd plus the
/// per-packet fallback. Owns the scratch `MmsgBufs` (whose raw pointers
/// point at its own iovec allocation and the batch's log bytes) so nothing
/// dangles across `.await` points; `Send` holds via the `MmsgBufs` impl
/// below because no other thread can ever observe the pointers.
#[cfg(target_os = "linux")]
struct SocketTransport<'s, 'b> {
    fd: libc::c_int,
    socket: &'s UdpSocket,
    batch: &'b [&'b [u8]],
    bufs: MmsgBufs,
}

#[cfg(target_os = "linux")]
impl MmsgTransport for SocketTransport<'_, '_> {
    fn send_step(&mut self, sent: usize) -> MmsgOutcome {
        sendmmsg_once(self.fd, &mut self.bufs.headers[sent..])
    }

    async fn wait_writable(&mut self) -> bool {
        // Clear Tokio's cached readiness before awaiting: the raw syscall
        // above runs outside `try_io`, so without this the readiness bit
        // from before the EAGAIN can stay set and `writable().await` would
        // return immediately, spinning the syscall while the socket is
        // still blocked. A guaranteed-`WouldBlock` `try_io` resets the bit
        // so the await genuinely sleeps until the socket is writable.
        let _ = self.socket.try_io(tokio::io::Interest::WRITABLE, || {
            Err::<(), std::io::Error>(std::io::Error::from(std::io::ErrorKind::WouldBlock))
        });
        self.socket.writable().await.is_ok()
    }

    async fn fallback_from(&mut self, sent: usize) -> (u64, u64, u64) {
        send_udp_batch_one_by_one(self.socket, &self.batch[sent..]).await
    }
}

async fn run_udp_worker(
    worker_id: usize,
    target: SocketAddr,
    logs: Arc<Vec<Vec<u8>>>,
    stats: Arc<Stats>,
    running: Arc<AtomicBool>,
    target_rate: u64,
    batch_size: usize,
) {
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Worker {}] Failed to bind UDP socket: {}", worker_id, e);
            stats.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    if let Err(e) = socket.connect(target).await {
        eprintln!(
            "[Worker {}] Failed to connect UDP socket to {}: {}",
            worker_id, target, e
        );
        stats.errors.fetch_add(1, Ordering::Relaxed);
        return;
    }

    let num_logs = logs.len();
    let mut log_idx = (worker_id * 17) % num_logs;
    let worker_start = Instant::now();
    let mut sent_count = 0u64;

    while running.load(Ordering::Relaxed) {
        let to_send = next_batch_size(sent_count, worker_start, target_rate, batch_size).await;
        if to_send == 0 {
            continue;
        }

        // One batch = one sendmmsg syscall on Linux, still 1 log = 1 datagram.
        let batch = collect_batch(&logs, log_idx, to_send);
        log_idx = (log_idx + to_send) % num_logs;
        let (batch_pkts, batch_bytes, batch_errors) = send_udp_batch(&socket, &batch).await;

        sent_count += batch_pkts;
        stats.packets_sent.fetch_add(batch_pkts, Ordering::Relaxed);
        stats.bytes_sent.fetch_add(batch_bytes, Ordering::Relaxed);
        stats.errors.fetch_add(batch_errors, Ordering::Relaxed);

        if target_rate == 0 {
            // Unthrottled yield to allow cooperative task scheduling
            tokio::task::yield_now().await;
        }
    }
}

/// Connects a TCP stream with Nagle's algorithm disabled.
///
/// Benchmark payloads are small per-log `write_all` calls; leaving Nagle on
/// would delay segments up to ~200ms waiting to coalesce, adding jitter to
/// the measured ingest path that has nothing to do with parser throughput.
/// Single call site for both the initial connect and the reconnect path so
/// a reconnected stream can never silently lose the flag.
async fn connect_tcp(target: SocketAddr) -> Result<TcpStream> {
    let stream = TcpStream::connect(target)
        .await
        .with_context(|| format!("Failed to connect TCP to {}", target))?;
    stream
        .set_nodelay(true)
        .context("Failed to set TCP_NODELAY")?;
    Ok(stream)
}

async fn run_tcp_worker(
    worker_id: usize,
    target: SocketAddr,
    logs: Arc<Vec<Vec<u8>>>,
    stats: Arc<Stats>,
    running: Arc<AtomicBool>,
    target_rate: u64,
    batch_size: usize,
) {
    let mut stream = match connect_tcp(target).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Worker {}] {:?}", worker_id, e);
            stats.errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    let num_logs = logs.len();
    let mut log_idx = (worker_id * 17) % num_logs;
    let worker_start = Instant::now();
    let mut sent_count = 0u64;

    let mut send_buf = Vec::with_capacity(batch_size * 256);

    while running.load(Ordering::Relaxed) {
        let to_send = next_batch_size(sent_count, worker_start, target_rate, batch_size).await;
        if to_send == 0 {
            continue;
        }

        send_buf.clear();
        for _ in 0..to_send {
            let log_bytes = &logs[log_idx];
            log_idx = (log_idx + 1) % num_logs;
            send_buf.extend_from_slice(log_bytes);
        }

        match stream.write_all(&send_buf).await {
            Ok(_) => {
                sent_count += to_send as u64;
                stats
                    .packets_sent
                    .fetch_add(to_send as u64, Ordering::Relaxed);
                stats
                    .bytes_sent
                    .fetch_add(send_buf.len() as u64, Ordering::Relaxed);
            }
            Err(e) => {
                stats.errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "[Worker {}] TCP write error: {}. Reconnecting...",
                    worker_id, e
                );
                tokio::time::sleep(Duration::from_millis(500)).await;
                // Reconnect through the same helper so TCP_NODELAY survives.
                if let Ok(new_stream) = connect_tcp(target).await {
                    stream = new_stream;
                }
            }
        }

        if target_rate == 0 {
            tokio::task::yield_now().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal SHA-256 for the wire-equality test below. Test-only, kept here
    /// so the benchmark binary gains no new dependency for a test assertion.
    fn sha256_hex(data: &[u8]) -> String {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
        for chunk in msg.as_chunks::<64>().0 {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[4 * i],
                    chunk[4 * i + 1],
                    chunk[4 * i + 2],
                    chunk[4 * i + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let t1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        h.iter().map(|x| format!("{:08x}", x)).collect()
    }

    #[test]
    fn sha256_helper_known_answers() {
        // Guards against a degenerate helper making equality tests vacuous.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// Variable-length corpus, like the real multi-vendor dataset (the
    /// reason GSO with fixed-size segments was rejected for this path).
    fn sample_corpus() -> Vec<Vec<u8>> {
        vec![
            b"<134>fw-01 %ASA-4-106023: Deny udp src wan:1.2.3.4/53 dst lan:10.0.0.5/5353".to_vec(),
            b"<134>short".to_vec(),
            b"<134>fg-01 date=2024-01-01 time=00:00:01 devname=fg action=allow src=10.0.0.1 dst=8.8.8.8 msg=\"ok\"".to_vec(),
            b"<134>panw-01 TRAFFIC start src=192.168.1.2 dst=4.4.4.4 rule=allow-all bytes=177".to_vec(),
            b"<134>suricata-01 {\"event_type\":\"alert\",\"src_ip\":\"1.1.1.1\",\"signature\":\"ET MALWARE Possible Trojan\"}".to_vec(),
        ]
    }

    #[tokio::test]
    async fn tcp_connect_sets_nodelay_initial_and_reconnect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Initial connect path.
        let first = connect_tcp(addr).await.unwrap();
        assert!(
            first.nodelay().unwrap(),
            "TCP_NODELAY must be set after the initial connect"
        );
        drop(first);

        // Forced reconnect through the same helper the worker's reconnect
        // path uses — the flag must survive a reconnect.
        let second = connect_tcp(addr).await.unwrap();
        assert!(
            second.nodelay().unwrap(),
            "TCP_NODELAY must be set after a reconnect"
        );
    }

    #[test]
    fn udp_batch_builder_matches_per_packet_order() {
        // Single worker, deterministic start (worker 0 begins at index 0),
        // batch size that does not divide the corpus to force wrap-around.
        let logs = sample_corpus();
        let batch_size = 4;
        let mut idx = 0;
        let mut batched_stream = Vec::new();
        let mut direct_stream = Vec::new();
        for _ in 0..5 {
            for payload in collect_batch(&logs, idx, batch_size) {
                batched_stream.extend_from_slice(payload);
            }
            for i in 0..batch_size {
                direct_stream.extend_from_slice(&logs[(idx + i) % logs.len()]);
            }
            idx = (idx + batch_size) % logs.len();
        }
        // Same bytes in the same order: batching changes syscalls, not wire content.
        assert_eq!(batched_stream, direct_stream);
        assert_eq!(sha256_hex(&batched_stream), sha256_hex(&direct_stream));
    }

    #[tokio::test]
    async fn udp_batched_send_preserves_datagram_framing() {
        // End-to-end over loopback: every datagram received must equal one
        // log line byte-for-byte (1 log = 1 datagram, no coalescing).
        let rx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let rx_addr = rx.local_addr().unwrap();
        let tx = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        tx.connect(rx_addr).await.unwrap();

        let logs = sample_corpus();
        let batch = collect_batch(&logs, 0, logs.len());
        let (pkts, bytes, errors) = send_udp_batch(&tx, &batch).await;
        assert_eq!(errors, 0);
        assert_eq!(pkts, batch.len() as u64);
        assert_eq!(bytes, batch.iter().map(|b| b.len() as u64).sum::<u64>());

        let mut got = Vec::new();
        for _ in 0..logs.len() {
            let mut buf = vec![0u8; 65535];
            let n = tokio::time::timeout(Duration::from_secs(2), rx.recv(&mut buf))
                .await
                .expect("datagram receive timed out")
                .unwrap();
            got.push(buf[..n].to_vec());
        }
        // Sort both sides: datagram boundaries are what this proves, and
        // sorting removes any loopback reorder flake from the assertion.
        got.sort();
        let mut want = logs.clone();
        want.sort();
        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn pacing_unthrottled_sends_full_batch() {
        assert_eq!(next_batch_size(0, Instant::now(), 0, 64).await, 64);
    }

    /// Fault-injection seam: scripted `MmsgTransport` so sendmmsg
    /// transitions are covered without touching the network.
    #[cfg(target_os = "linux")]
    struct ScriptTransport {
        script: std::collections::VecDeque<MmsgOutcome>,
        send_calls: usize,
        wait_calls: usize,
        fallback_offsets: Vec<usize>,
        fallback_result: (u64, u64, u64),
        wait_ok: bool,
    }

    #[cfg(target_os = "linux")]
    impl ScriptTransport {
        fn new(script: Vec<MmsgOutcome>, fallback_result: (u64, u64, u64), wait_ok: bool) -> Self {
            Self {
                script: script.into(),
                send_calls: 0,
                wait_calls: 0,
                fallback_offsets: Vec::new(),
                fallback_result,
                wait_ok,
            }
        }
    }

    #[cfg(target_os = "linux")]
    impl MmsgTransport for ScriptTransport {
        fn send_step(&mut self, _sent: usize) -> MmsgOutcome {
            self.send_calls += 1;
            self.script.pop_front().expect("script exhausted")
        }

        async fn wait_writable(&mut self) -> bool {
            self.wait_calls += 1;
            self.wait_ok
        }

        async fn fallback_from(&mut self, sent: usize) -> (u64, u64, u64) {
            self.fallback_offsets.push(sent);
            self.fallback_result
        }
    }

    /// Finding 1: a partial `sendmmsg` batch followed by a hard error must
    /// keep the pre-error successes in the returned packet count.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn mmsg_partial_send_to_error_keeps_pre_error_count() {
        let mut transport = ScriptTransport::new(
            vec![
                MmsgOutcome::Sent {
                    count: 3,
                    bytes: 300,
                },
                MmsgOutcome::FallbackRemainder,
            ],
            (2, 200, 1),
            true,
        );
        let (pkts, bytes, errors) = mmsg_drive_loop(7, &mut transport).await;
        assert_eq!(pkts, 5, "pre-error successes must be in the total");
        assert_eq!(bytes, 500);
        assert_eq!(errors, 1);
        assert_eq!(transport.fallback_offsets, vec![3]);
    }

    /// Finding 2: persistent EAGAIN must gate every retry on one readiness
    /// wait — one send attempt per EAGAIN, never a busy spin.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn mmsg_persistent_eagain_waits_instead_of_spinning() {
        let mut script = vec![MmsgOutcome::WaitWritable; 50];
        script.push(MmsgOutcome::Sent {
            count: 4,
            bytes: 400,
        });
        let mut transport = ScriptTransport::new(script, (0, 0, 0), true);
        let (pkts, bytes, errors) = mmsg_drive_loop(4, &mut transport).await;
        assert_eq!((pkts, bytes, errors), (4, 400, 0));
        assert_eq!(transport.send_calls, 51);
        assert_eq!(transport.wait_calls, 50);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn mmsg_writable_failure_counts_remainder_as_errors() {
        let mut transport = ScriptTransport::new(vec![MmsgOutcome::WaitWritable], (0, 0, 0), false);
        let (pkts, bytes, errors) = mmsg_drive_loop(4, &mut transport).await;
        assert_eq!((pkts, bytes, errors), (0, 0, 4));
    }
}
