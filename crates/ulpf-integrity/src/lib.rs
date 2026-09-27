//! ULPF Integrity Plane & Merkle Storage Crate
//!
//! Provides:
//! - **RFC 6962 Standard Merkle Tree** (Certificate Transparency compliant):
//!   - `SHA256(0x00 || raw_log)` for leaves
//!   - `SHA256(0x01 || left || right)` for internal nodes
//!   - $O(\log N)$ inclusion proof generation and verification
//!   - Append-only consistency proofs
//! - **Dual-trigger Batch Accumulator**:
//!   - Flushes on `count >= 1,000` or `duration >= 2,000ms`
//!   - In-memory & on-disk append-only ledger (`ledger.jsonl`)
//!   - Automatic Merkle Tree root calculation and Parquet persistence
//! - **Columnar Apache Parquet WORM Storage**:
//!   - Arrow schema integration with Snappy & ZSTD compression
//! - **Forensic Tamper Detection**:
//!   - Bit-flip, IP alteration, row deletion, and order tampering detection
//!   - Direct isolation of corrupted index and diffing of expected vs calculated hashes

pub mod batcher;
pub mod merkle;
pub mod proof;
pub mod storage;
pub mod tamper;

pub use batcher::{
    BatchAccumulator, BatcherConfig, BlockFlushResult, IncomingLog, LedgerEntry,
    DEFAULT_MAX_BATCH_DURATION_MS, DEFAULT_MAX_BATCH_SIZE,
};
pub use merkle::{
    empty_tree_hash, hash_children, hash_leaf, largest_power_of_two_less_than,
    verify_consistency_proof, verify_inclusion_proof, verify_inclusion_proof_by_hash, Hash,
    InclusionProof, MerkleError, MerkleTree, Side, LEAF_PREFIX, NODE_PREFIX,
};
pub use proof::{
    check_ledger_consistency, prove_leaf, ConsistencyPair, ConsistencyReport, InclusionProofOutput,
    ProofAuditStep, ProveOutcome,
};
pub use storage::{
    batch_to_records, log_schema, read_parquet_file, records_to_batch, write_parquet_file,
    write_records_to_parquet, ParquetCompression, StoredLogRecord,
};
pub use tamper::{
    block_id_for_parquet, find_ledger_entry, verify_block_file, verify_block_with_ledger,
    verify_records, TamperReason, TamperReport, TamperedRecord,
};
