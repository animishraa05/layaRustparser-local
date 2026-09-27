//! Dual-trigger batch accumulator for the ULPF Integrity Plane.
//!
//! Flushes blocks based on:
//! - Count trigger: `count >= 1,000` records
//! - Duration trigger: `duration >= 2,000` ms
//!
//! On flush:
//! 1. Computes the RFC 6962 Merkle Tree over all raw log strings in the block.
//! 2. Writes an append-only entry to the in-memory and on-disk `ledger.jsonl`.
//! 3. Writes the block to columnar Apache Parquet format.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;
use uuid::Uuid;

use crate::merkle::{Hash, MerkleTree};
use crate::storage::{write_records_to_parquet, ParquetCompression, StoredLogRecord};

/// Default batch threshold: 1,000 records
pub const DEFAULT_MAX_BATCH_SIZE: usize = 1_000;
/// Default timeout threshold: 2,000 milliseconds
pub const DEFAULT_MAX_BATCH_DURATION_MS: u64 = 2_000;

/// Configuration for the batch accumulator.
#[derive(Clone, Debug)]
pub struct BatcherConfig {
    pub max_batch_size: usize,
    pub max_batch_duration_ms: u64,
    pub storage_dir: PathBuf,
    pub ledger_path: PathBuf,
    pub compression: ParquetCompression,
}

impl Default for BatcherConfig {
    fn default() -> Self {
        Self {
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            max_batch_duration_ms: DEFAULT_MAX_BATCH_DURATION_MS,
            storage_dir: PathBuf::from("data/parquet"),
            ledger_path: PathBuf::from("data/ledger.jsonl"),
            compression: ParquetCompression::Snappy,
        }
    }
}

/// An incoming unbatched log entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingLog {
    pub vendor: String,
    pub raw_log: String,
    pub timestamp: Option<i64>,
    pub event_id: Option<String>,
    pub ocsf_json: Option<String>,
    /// Hex-encoded SHA-256 of `raw_log`, passed through from the parser so the
    /// batcher never re-hashes. `None` (e.g. older callers, tests) falls back
    /// to hashing at flush time — output is byte-identical either way.
    pub raw_hash: Option<String>,
}

impl IncomingLog {
    /// Builds a log entry without parser-supplied extras; hash falls back to
    /// a flush-time recompute, so this is always safe for ad-hoc callers.
    pub fn new(vendor: impl Into<String>, raw_log: impl Into<String>) -> Self {
        Self {
            vendor: vendor.into(),
            raw_log: raw_log.into(),
            timestamp: None,
            event_id: None,
            ocsf_json: None,
            raw_hash: None,
        }
    }

    /// Attaches the event timestamp (epoch millis).
    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    /// Attaches a pre-assigned event id (a UUIDv7 is minted at flush otherwise).
    pub fn with_event_id(mut self, id: impl Into<String>) -> Self {
        self.event_id = Some(id.into());
        self
    }

    /// Attaches the pre-serialized OCSF payload (`"{}"` is used when absent).
    pub fn with_ocsf(mut self, ocsf: impl Into<String>) -> Self {
        self.ocsf_json = Some(ocsf.into());
        self
    }

    /// Attaches the parser-computed SHA-256 of `raw_log` (unprefixed lowercase
    /// hex, 64 chars / 32 bytes) so the flush path can pass it straight into
    /// Parquet without hashing a second time.
    ///
    /// Trust contract: the value MUST be `SHA-256(raw_log)` for the exact
    /// bytes in `raw_log`. The only trusted supplier is the parser ingest
    /// path, which hashes the line once at classification time. A
    /// valid-length-but-wrong digest would persist a `raw_hash` disagreeing
    /// with `raw_log` (only caught later by `verify`), so debug builds
    /// re-check the digest at flush and panic on mismatch — release builds
    /// skip that rehash entirely. Malformed values (non-hex, wrong length)
    /// are rejected with an error before anything is buffered or drained.
    pub fn with_raw_hash(mut self, hash: impl Into<String>) -> Self {
        self.raw_hash = Some(hash.into());
        self
    }
}

/// An append-only ledger entry recorded on disk and in memory.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerEntry {
    pub block_id: u64,
    pub timestamp: i64,
    pub leaf_count: usize,
    pub merkle_root: String,
    pub parquet_file: String,
}

/// Result returned upon a successful block flush.
#[derive(Clone, Debug)]
pub struct BlockFlushResult {
    pub block_id: u64,
    pub merkle_root: Hash,
    pub leaf_count: usize,
    pub parquet_path: PathBuf,
    pub tree: MerkleTree,
    /// Wall time spent in the durable pair (parquet sync + ledger sync) in
    /// microseconds. Surfaced so the flush thread can log the fsync cost at
    /// INFO — the proof that crash safety is cheap on the ingest path.
    pub fsync_micros: u64,
}

/// Checks a parser-supplied hash is well-formed hex decoding to 32 bytes.
///
/// This is deliberately format-only — no rehash of `raw_log`, so it stays
/// cheap on the ingest path. Anything malformed is rejected here, at the
/// input boundary, before the entry is buffered or a flush drains the batch.
fn validate_supplied_raw_hash(hash: &str, vendor: &str) -> Result<()> {
    let bytes = hex::decode(hash)
        .with_context(|| format!("supplied raw_hash for vendor '{vendor}' is not valid hex"))?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "supplied raw_hash for vendor '{vendor}' decodes to {} bytes, expected 32 (SHA-256)",
            bytes.len()
        );
    }
    Ok(())
}

/// Dual-trigger batch accumulator.
pub struct BatchAccumulator {
    config: BatcherConfig,
    buffer: Vec<IncomingLog>,
    first_item_time: Option<Instant>,
    current_block_id: u64,
    in_memory_ledger: Vec<LedgerEntry>,
}

impl BatchAccumulator {
    /// Creates a new `BatchAccumulator`. If an existing ledger is found, initializes the
    /// `current_block_id` to continue from the last block.
    pub fn new(config: BatcherConfig) -> Result<Self> {
        let in_memory_ledger = if config.ledger_path.exists() {
            Self::load_ledger_entries(&config.ledger_path)?
        } else {
            Vec::new()
        };

        let next_block_id = in_memory_ledger
            .last()
            .map(|entry| entry.block_id + 1)
            .unwrap_or(0);

        Ok(Self {
            config,
            buffer: Vec::new(),
            first_item_time: None,
            current_block_id: next_block_id,
            in_memory_ledger,
        })
    }

    /// Loads all entries from an existing `ledger.jsonl` file.
    pub fn load_ledger_entries(ledger_path: impl AsRef<Path>) -> Result<Vec<LedgerEntry>> {
        let file = File::open(ledger_path.as_ref())
            .with_context(|| format!("Failed to open ledger at {:?}", ledger_path.as_ref()))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (idx, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Error reading line {} from ledger", idx))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry: LedgerEntry = serde_json::from_str(trimmed).with_context(|| {
                format!("Failed parsing ledger entry at line {}: {}", idx, trimmed)
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Returns the current pending buffer length.
    pub fn pending_count(&self) -> usize {
        self.buffer.len()
    }

    /// Returns the duration since the oldest item in the buffer was ingested.
    pub fn pending_duration(&self) -> Option<Duration> {
        self.first_item_time.map(|t| t.elapsed())
    }

    /// Returns the in-memory ledger history.
    pub fn in_memory_ledger(&self) -> &[LedgerEntry] {
        &self.in_memory_ledger
    }

    /// Ingests an incoming log entry. A supplied `raw_hash` is format-checked
    /// up front (hex + 32 bytes) so a bad value is rejected before it reaches
    /// the buffer. Flushes immediately if the count or duration trigger is
    /// satisfied.
    pub fn push(&mut self, log: IncomingLog) -> Result<Option<BlockFlushResult>> {
        if let Some(hash) = log.raw_hash.as_deref() {
            validate_supplied_raw_hash(hash, &log.vendor)?;
        }
        if self.buffer.is_empty() {
            self.first_item_time = Some(Instant::now());
        }
        self.buffer.push(log);

        if self.should_flush() {
            self.flush()
        } else {
            Ok(None)
        }
    }

    /// Checks whether the duration trigger has elapsed and flushes if necessary.
    pub fn check_timeout(&mut self) -> Result<Option<BlockFlushResult>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        if let Some(elapsed) = self.pending_duration() {
            if elapsed >= Duration::from_millis(self.config.max_batch_duration_ms) {
                return self.flush();
            }
        }

        Ok(None)
    }

    /// Returns true if either the count trigger or duration trigger is met.
    pub fn should_flush(&self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }

        let count_met = self.buffer.len() >= self.config.max_batch_size;
        let duration_met = self
            .pending_duration()
            .map(|d| d >= Duration::from_millis(self.config.max_batch_duration_ms))
            .unwrap_or(false);

        count_met || duration_met
    }

    /// Forces a flush of all currently accumulated logs into a Parquet block and ledger entry.
    ///
    /// Every supplied hash is re-validated for format before the buffer is
    /// drained, so a malformed value fails here with the batch still intact
    /// instead of dying mid-write in the storage conversion.
    pub fn flush(&mut self) -> Result<Option<BlockFlushResult>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        for item in &self.buffer {
            if let Some(hash) = item.raw_hash.as_deref() {
                validate_supplied_raw_hash(hash, &item.vendor)?;
            }
        }

        let logs = std::mem::take(&mut self.buffer);
        self.first_item_time = None;

        let block_id = self.current_block_id;
        self.current_block_id += 1;

        let now_ms = Utc::now().timestamp_millis();
        let count = logs.len();

        // 1. Build StoredLogRecords
        let mut stored_records = Vec::with_capacity(count);
        for (i, item) in logs.into_iter().enumerate() {
            let event_id = item.event_id.unwrap_or_else(|| Uuid::now_v7().to_string());
            let timestamp = item.timestamp.unwrap_or(now_ms);
            // Hash #2 deleted: the parser already hashed these exact bytes
            // (unprefixed hex SHA-256), so pass it through. A missing hash —
            // older callers, ad-hoc tests — recomputes the identical digest.
            let raw_hash = match item.raw_hash {
                Some(supplied) => {
                    // Trust contract (see `with_raw_hash`): the parser guarantees
                    // this is SHA-256(raw_log). Recompute only under
                    // debug_assertions, so our own bugs trip in dev/CI/test at
                    // zero cost to release throughput.
                    debug_assert_eq!(
                        supplied,
                        hex::encode(Sha256::digest(item.raw_log.as_bytes())),
                        "with_raw_hash contract violated for vendor '{}': supplied digest is not SHA-256(raw_log)",
                        item.vendor
                    );
                    supplied
                }
                None => hex::encode(Sha256::digest(item.raw_log.as_bytes())),
            };
            let ocsf_json = item.ocsf_json.unwrap_or_else(|| "{}".to_string());

            stored_records.push(StoredLogRecord {
                event_id,
                block_id,
                leaf_index: i as u32,
                timestamp,
                vendor: item.vendor,
                raw_log: item.raw_log,
                raw_hash,
                ocsf_json,
            });
        }

        // 2. Compute RFC 6962 Merkle Tree over raw logs
        let tree = MerkleTree::from_raw_logs(stored_records.iter().map(|r| r.raw_log.as_bytes()));
        let merkle_root = tree.root();
        let root_hex = merkle_root.to_hex();

        // 3. Write Parquet block file, then make it durable BEFORE the
        // ledger line exists: sync the file AND its parent directory, so a
        // crash can never leave a ledger entry pointing at a missing or
        // half-written block (rename durability needs the dir sync).
        let parquet_filename = format!("block_{:05}.parquet", block_id);
        let parquet_path = self.config.storage_dir.join(&parquet_filename);

        write_records_to_parquet(&parquet_path, &stored_records, self.config.compression)
            .with_context(|| format!("Failed writing Parquet block to {:?}", parquet_path))?;

        let fsync_start = Instant::now();
        fault_abort("ULPF_FAULT_POST_PARQUET_PRE_SYNC")?;
        sync_file_and_parent(&parquet_path)?;

        // 4. Append to on-disk & in-memory ledger, then sync the ledger file
        // so the entry is durable before flush() reports success.
        let ledger_entry = LedgerEntry {
            block_id,
            timestamp: now_ms,
            leaf_count: count,
            merkle_root: root_hex,
            parquet_file: parquet_filename,
        };

        fault_abort("ULPF_FAULT_PRE_LEDGER_SYNC")?;
        self.append_ledger_entry(&ledger_entry)?;
        fault_abort("ULPF_FAULT_POST_LEDGER_SYNC")?;
        let fsync_micros = fsync_start.elapsed().as_micros() as u64;
        // The fsync pair lives off the parse hot path (flush thread only),
        // but its cost is still worth stating on every flush: if this number
        // ever stops being small, the ingest budget needs re-measuring.
        info!(
            "[DURABLE FLUSH] Block #{} fsync pair took {} µs (parquet file+dir, ledger file)",
            block_id, fsync_micros
        );
        self.in_memory_ledger.push(ledger_entry);

        Ok(Some(BlockFlushResult {
            block_id,
            merkle_root,
            leaf_count: count,
            parquet_path,
            tree,
            fsync_micros,
        }))
    }

    fn append_ledger_entry(&self, entry: &LedgerEntry) -> Result<()> {
        if let Some(parent) = self.config.ledger_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.ledger_path)
            .with_context(|| format!("Failed opening ledger at {:?}", self.config.ledger_path))?;

        let serialized =
            serde_json::to_string(entry).context("Failed serializing ledger entry to JSON")?;
        writeln!(file, "{}", serialized).context("Failed appending entry to ledger file")?;
        file.flush().context("Failed flushing ledger file")?;
        // Durability, not just visibility: without sync_all a kill -9 between
        // the write and the kernel flush loses the entry while the Parquet
        // block (already synced above) survives — an orphaned block the
        // ledger never names.
        file.sync_all()
            .with_context(|| format!("Failed syncing ledger at {:?}", self.config.ledger_path))?;

        Ok(())
    }
}

/// Syncs a freshly written file and its parent directory.
///
/// The file sync pushes content through the page cache; the directory sync
/// makes the directory entry itself durable, so a crash right after flush
/// cannot lose the name-to-inode mapping. Both are plain `sync_all` — no
/// platform-specific durability APIs, nothing a reviewer needs a man page for.
fn sync_file_and_parent(path: &Path) -> Result<()> {
    let file = File::open(path)
        .with_context(|| format!("Failed reopening {:?} for durability sync", path))?;
    file.sync_all()
        .with_context(|| format!("Failed syncing Parquet block at {:?}", path))?;
    // The crate only ever writes under real directories (storage_dir is
    // created by the caller), so a missing parent is a bug, not a skip.
    if let Some(parent) = path.parent() {
        let dir = File::open(parent)
            .with_context(|| format!("Failed opening parent dir {:?} for sync", parent))?;
        dir.sync_all()
            .with_context(|| format!("Failed syncing parent dir {:?}", parent))?;
    }
    Ok(())
}

/// Deterministic crash-consistency hook for tests.
///
/// When the named env var is present (any value), returns an `Err` that
/// aborts the flush at that exact point — simulating a `kill -9` between
/// durability steps without ever sending a signal. Gated purely on the
/// environment: unset in production, the check is one `getenv` per flush
/// (off the hot path) and changes nothing. Known points:
/// - `ULPF_FAULT_POST_PARQUET_PRE_SYNC`: parquet written, nothing synced,
///   no ledger line (orphaned block, ledger silent).
/// - `ULPF_FAULT_PRE_LEDGER_SYNC`: parquet durable, no ledger line.
/// - `ULPF_FAULT_POST_LEDGER_SYNC`: entry appended but unsynced (tests the
///   sync itself, not the append).
fn fault_abort(env_var: &str) -> Result<()> {
    if std::env::var_os(env_var).is_some() {
        anyhow::bail!("fault injected at {} (env-gated crash simulation)", env_var);
    }
    Ok(())
}
