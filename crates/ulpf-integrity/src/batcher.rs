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

    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    pub fn with_event_id(mut self, id: impl Into<String>) -> Self {
        self.event_id = Some(id.into());
        self
    }

    pub fn with_ocsf(mut self, ocsf: impl Into<String>) -> Self {
        self.ocsf_json = Some(ocsf.into());
        self
    }

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

    /// Ingests an incoming log entry. Flushes immediately if the count or duration
    /// trigger is satisfied.
    pub fn push(&mut self, log: IncomingLog) -> Result<Option<BlockFlushResult>> {
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
    pub fn flush(&mut self) -> Result<Option<BlockFlushResult>> {
        if self.buffer.is_empty() {
            return Ok(None);
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
            let raw_hash = hex::encode(Sha256::digest(item.raw_log.as_bytes()));
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

        // 3. Write Parquet block file
        let parquet_filename = format!("block_{:05}.parquet", block_id);
        let parquet_path = self.config.storage_dir.join(&parquet_filename);

        write_records_to_parquet(&parquet_path, &stored_records, self.config.compression)
            .with_context(|| format!("Failed writing Parquet block to {:?}", parquet_path))?;

        // 4. Append to on-disk & in-memory ledger
        let ledger_entry = LedgerEntry {
            block_id,
            timestamp: now_ms,
            leaf_count: count,
            merkle_root: root_hex,
            parquet_file: parquet_filename,
        };

        self.append_ledger_entry(&ledger_entry)?;
        self.in_memory_ledger.push(ledger_entry);

        Ok(Some(BlockFlushResult {
            block_id,
            merkle_root,
            leaf_count: count,
            parquet_path,
            tree,
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
        file.flush().context("Failed syncing ledger file")?;

        Ok(())
    }
}
