use sha2::Digest;
use tempfile::TempDir;

use ulpf_integrity::batcher::{BatchAccumulator, BatcherConfig, IncomingLog};
use ulpf_integrity::merkle::{
    empty_tree_hash, hash_leaf, verify_consistency_proof, Hash, MerkleTree, Side,
};
use ulpf_integrity::storage::{
    read_parquet_file, write_records_to_parquet, ParquetCompression, StoredLogRecord,
};
use ulpf_integrity::tamper::{verify_block_file, verify_block_with_ledger, TamperReason};

/// Serializes every test that drives a flush.
///
/// The crash hooks in `batcher.rs` are env-gated (`ULPF_FAULT_*`), and the
/// environment is process-global: a fault var set by one test would abort
/// flushes in every other test sharing this binary. Every flush-driving
/// test — old and new — holds this lock, so a fault window can never leak
/// into an unrelated flush. Cheap (uncontended mutex, held for ms) and
/// load-bearing: remove it and the fault tests become flaky under
/// `cargo test` parallelism.
static FLUSH_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn exclusive_flush_env() -> std::sync::MutexGuard<'static, ()> {
    FLUSH_SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Sets an `ULPF_FAULT_*` var for exactly one flush and removes it on drop —
/// even on panic — so no fault survives into the next test.
struct FaultEnv(&'static str);

impl FaultEnv {
    fn set(name: &'static str) -> Self {
        // SAFETY: single-threaded w.r.t. other flushes by FLUSH_SERIAL; no
        // other test in this binary touches these var names.
        unsafe { std::env::set_var(name, "1") };
        FaultEnv(name)
    }
}

impl Drop for FaultEnv {
    fn drop(&mut self) {
        unsafe { std::env::remove_var(self.0) };
    }
}

/// Builds a batcher over a fresh scratch dir with caller-chosen thresholds.
fn scratch_batcher(
    max_batch_size: usize,
    max_batch_duration_ms: u64,
) -> (TempDir, BatchAccumulator) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let config = BatcherConfig {
        max_batch_size,
        max_batch_duration_ms,
        storage_dir: temp_dir.path().join("parquet"),
        ledger_path: temp_dir.path().join("ledger.jsonl"),
        compression: ParquetCompression::Snappy,
    };
    let batcher = BatchAccumulator::new(config).expect("init batcher");
    (temp_dir, batcher)
}

/// Every ledger entry must name a Parquet file that exists on disk — the
/// crash-safety invariant the durable pair exists to guarantee (a ledger
/// line without its block is a proof pointing at nothing).
fn assert_no_orphan_ledger_lines(ledger_path: &std::path::Path) {
    // A ledger that was never created holds zero entries — vacuously clean.
    if !ledger_path.exists() {
        return;
    }
    let entries =
        BatchAccumulator::load_ledger_entries(ledger_path).expect("ledger must still parse");
    let parent = ledger_path.parent().expect("ledger has a parent dir");
    for entry in &entries {
        let block_path = parent.join("parquet").join(&entry.parquet_file);
        assert!(
            block_path.exists(),
            "ledger entry for block #{} names {:?}, which is missing: orphan!",
            entry.block_id,
            block_path
        );
    }
}

/// Helper to generate synthetic log strings for testing
fn generate_logs(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            format!(
                "<134>Sep 21 12:00:{:02} firewall-01 %ASA-6-302013: Built outbound TCP connection {} for outside:192.168.1.{}/443 (192.168.1.{}/443) to inside:10.0.0.{}/54321",
                i % 60,
                100000 + i,
                (i % 250) + 1,
                (i % 250) + 1,
                (i % 200) + 1
            )
        })
        .collect()
}

#[test]
fn test_rfc6962_empty_and_single_leaf() {
    // 1. Empty tree
    let empty_tree = MerkleTree::empty();
    assert_eq!(empty_tree.len(), 0);
    assert!(empty_tree.is_empty());
    assert_eq!(empty_tree.root(), empty_tree_hash());
    // SHA-256 of empty string is e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    assert_eq!(
        empty_tree.root_hex(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    // 2. Single leaf tree
    let raw_log = b"test single raw log line";
    let single_tree = MerkleTree::from_raw_logs(vec![raw_log]);
    assert_eq!(single_tree.len(), 1);
    let expected_leaf_hash = hash_leaf(raw_log);
    assert_eq!(single_tree.root(), expected_leaf_hash);

    // Inclusion proof for single leaf
    let proof = single_tree.inclusion_proof(0).expect("Proof for leaf 0");
    assert_eq!(proof.leaf_index, 0);
    assert_eq!(proof.tree_size, 1);
    assert!(proof.audit_path.is_empty());
    assert!(proof.verify(raw_log, &single_tree.root()));
}

#[test]
fn test_merkle_tree_scale_building() {
    // Requirements: Test tree building with 10, 100, 1,000, 5,000 leaves
    let test_scales = [10, 100, 1_000, 5_000];

    for &scale in &test_scales {
        let logs = generate_logs(scale);
        let tree = MerkleTree::from_raw_logs(logs.iter().map(|s| s.as_bytes()));

        assert_eq!(tree.len(), scale, "Tree size mismatch for scale {}", scale);
        assert!(!tree.root_hex().is_empty());

        // Verify inclusion proof for first, middle, last leaves
        let test_indices = [0, scale / 2, scale - 1];
        for &idx in &test_indices {
            let proof = tree.inclusion_proof(idx).unwrap_or_else(|e| {
                panic!(
                    "Failed to get proof for scale {} index {}: {:?}",
                    scale, idx, e
                )
            });

            assert_eq!(proof.leaf_index, idx);
            assert_eq!(proof.tree_size, scale);

            // Audit path length should be O(log2 N)
            let max_depth = (scale as f64).log2().ceil() as usize + 1;
            assert!(
                proof.audit_path.len() <= max_depth,
                "Path length {} exceeded max depth {} for scale {}",
                proof.audit_path.len(),
                max_depth,
                scale
            );

            // Proof must verify against raw log
            let raw_bytes = logs[idx].as_bytes();
            assert!(
                proof.verify(raw_bytes, &tree.root()),
                "Inclusion proof verification failed for scale {} at leaf index {}",
                scale,
                idx
            );
        }
    }
}

#[test]
fn test_inclusion_proof_arbitrary_indices_and_tree_sizes() {
    // Test diverse tree sizes: odd, even, powers of 2, non-powers of 2
    let sizes = [2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 127, 128, 250];

    for &n in &sizes {
        let logs = generate_logs(n);
        let tree = MerkleTree::from_raw_logs(logs.iter().map(|s| s.as_bytes()));

        // Check every single leaf in this tree
        for (i, log) in logs.iter().enumerate().take(n) {
            let proof = tree.inclusion_proof(i).expect("proof generation");
            assert!(
                proof.verify(log.as_bytes(), &tree.root()),
                "Failed verification for tree size {} leaf {}",
                n,
                i
            );
        }
    }
}

#[test]
fn test_inclusion_proof_tamper_detection() {
    let logs = generate_logs(100);
    let tree = MerkleTree::from_raw_logs(logs.iter().map(|s| s.as_bytes()));

    let target_idx = 42;
    let proof = tree.inclusion_proof(target_idx).expect("proof");

    // Valid verification
    assert!(proof.verify(logs[target_idx].as_bytes(), &tree.root()));

    // 1. Tamper raw log by flipping 1 byte
    let mut tampered_bytes = logs[target_idx].as_bytes().to_vec();
    tampered_bytes[0] ^= 0x01; // flip 1 bit
    assert!(
        !proof.verify(&tampered_bytes, &tree.root()),
        "Verification should fail when a byte is flipped!"
    );

    // 2. Tamper expected root
    let fake_root = Hash::from_bytes([0xaa; 32]);
    assert!(
        !proof.verify(logs[target_idx].as_bytes(), &fake_root),
        "Verification should fail with corrupted root!"
    );

    // 3. Tamper audit path: corrupt one sibling hash
    let mut corrupted_proof = proof.clone();
    if let Some(first) = corrupted_proof.audit_path.first_mut() {
        first.0 = Hash::from_bytes([0xff; 32]);
        assert!(
            !corrupted_proof.verify(logs[target_idx].as_bytes(), &tree.root()),
            "Verification should fail with corrupted audit path sibling!"
        );
    }

    // 4. Tamper audit path: flip side
    let mut flipped_side_proof = proof.clone();
    if let Some(first) = flipped_side_proof.audit_path.first_mut() {
        first.1 = match first.1 {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
        assert!(
            !flipped_side_proof.verify(logs[target_idx].as_bytes(), &tree.root()),
            "Verification should fail with corrupted audit path side!"
        );
    }
}

#[test]
fn test_parquet_write_and_read_cycle() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let parquet_path = temp_dir.path().join("test_cycle.parquet");

    let count = 50;
    let raw_logs = generate_logs(count);
    let mut original_records = Vec::with_capacity(count);

    for (i, raw_log) in raw_logs.into_iter().enumerate() {
        let raw_hash = hex::encode(sha2::Sha256::digest(raw_log.as_bytes()));
        original_records.push(StoredLogRecord {
            event_id: uuid::Uuid::now_v7().to_string(),
            block_id: 1,
            leaf_index: i as u32,
            timestamp: 1726910000000 + i as i64,
            vendor: "cisco_asa".to_string(),
            raw_log,
            raw_hash,
            ocsf_json: r#"{"class_uid": 4001, "activity_id": 1}"#.to_string(),
        });
    }

    // Test Snappy compression write & read
    write_records_to_parquet(&parquet_path, &original_records, ParquetCompression::Snappy)
        .expect("write parquet with snappy");
    let read_back = read_parquet_file(&parquet_path).expect("read parquet");

    assert_eq!(read_back.len(), count);
    assert_eq!(read_back, original_records);

    // Test ZSTD compression write & read
    let zstd_path = temp_dir.path().join("test_cycle_zstd.parquet");
    write_records_to_parquet(&zstd_path, &original_records, ParquetCompression::Zstd)
        .expect("write parquet with zstd");
    let read_back_zstd = read_parquet_file(&zstd_path).expect("read parquet zstd");

    assert_eq!(read_back_zstd.len(), count);
    assert_eq!(read_back_zstd, original_records);
}

#[test]
fn test_batcher_dual_trigger_count_and_ledger() {
    let _exclusive_io = exclusive_flush_env();
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    // Configure batcher with count threshold = 20
    let config = BatcherConfig {
        max_batch_size: 20,
        max_batch_duration_ms: 10_000,
        storage_dir: storage_dir.clone(),
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let mut batcher = BatchAccumulator::new(config).expect("init batcher");
    let logs = generate_logs(20);

    // Push 19 logs: should not flush yet
    for log in &logs[..19] {
        let res = batcher
            .push(IncomingLog::new("fortinet", log.clone()))
            .expect("push log");
        assert!(res.is_none(), "Should not flush before batch threshold");
    }
    assert_eq!(batcher.pending_count(), 19);

    // Push 20th log: triggers automatic flush!
    let flush_result = batcher
        .push(IncomingLog::new("fortinet", logs[19].clone()))
        .expect("push 20th log")
        .expect("Should flush on 20th log");

    assert_eq!(flush_result.block_id, 0);
    assert_eq!(flush_result.leaf_count, 20);
    assert_eq!(batcher.pending_count(), 0);
    assert!(flush_result.parquet_path.exists());

    // Check ledger file
    assert!(ledger_path.exists());
    let ledger_entries = BatchAccumulator::load_ledger_entries(&ledger_path).expect("load ledger");
    assert_eq!(ledger_entries.len(), 1);
    let entry = &ledger_entries[0];
    assert_eq!(entry.block_id, 0);
    assert_eq!(entry.leaf_count, 20);
    assert_eq!(entry.merkle_root, flush_result.merkle_root.to_hex());

    // Forensic verification on the newly flushed block
    let report =
        verify_block_with_ledger(&flush_result.parquet_path, &ledger_path).expect("verify block");
    assert!(report.is_valid, "Flushed block must pass verification!");
    assert_eq!(report.actual_records, 20);
    assert!(report.tampered_records.is_empty());
}

#[test]
fn test_batcher_dual_trigger_duration_timeout() {
    let _exclusive_io = exclusive_flush_env();
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    // Configure batcher with timeout threshold = 50ms, count = 1000
    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 50,
        storage_dir,
        ledger_path,
        compression: ParquetCompression::Snappy,
    };

    let mut batcher = BatchAccumulator::new(config).expect("init batcher");

    // Ingest 5 logs (count threshold 1000 not met)
    for log in generate_logs(5) {
        let res = batcher
            .push(IncomingLog::new("paloalto", log))
            .expect("push log");
        assert!(res.is_none());
    }
    assert_eq!(batcher.pending_count(), 5);

    // Immediate check before 50ms should not flush
    let early_check = batcher.check_timeout().expect("check timeout early");
    assert!(early_check.is_none());

    // Sleep 60ms to exceed 50ms timeout
    std::thread::sleep(std::time::Duration::from_millis(60));

    // check_timeout should now flush the batch!
    let timeout_flush = batcher
        .check_timeout()
        .expect("check timeout after delay")
        .expect("Batch should have flushed due to timeout trigger!");

    assert_eq!(timeout_flush.block_id, 0);
    assert_eq!(timeout_flush.leaf_count, 5);
    assert_eq!(batcher.pending_count(), 0);
}

#[test]
fn test_forensic_tamper_detection_byte_flip() {
    let _exclusive_io = exclusive_flush_env();
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir,
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let mut batcher = BatchAccumulator::new(config).expect("init");
    for log in generate_logs(50) {
        batcher
            .push(IncomingLog::new("suricata", log))
            .expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    // 1. Untampered check passes
    let clean_report = verify_block_file(&flush.parquet_path, &batcher.in_memory_ledger()[0])
        .expect("clean report");
    assert!(clean_report.is_valid);
    assert!(clean_report.tampered_records.is_empty());

    // 2. Tamper simulation: read records, flip 1 byte at leaf #27
    let mut records = read_parquet_file(&flush.parquet_path).expect("read");
    let tampered_leaf = 27;
    let original_log = records[tampered_leaf].raw_log.clone();
    // Corrupt one character in the raw log string
    records[tampered_leaf].raw_log = original_log.replacen("firewall-01", "firewall-02", 1);

    // Overwrite the parquet file with corrupted records
    write_records_to_parquet(&flush.parquet_path, &records, ParquetCompression::Snappy)
        .expect("write corrupted");

    // 3. Run forensic tamper detection
    let report =
        verify_block_file(&flush.parquet_path, &batcher.in_memory_ledger()[0]).expect("verify");

    assert!(report.is_tampered(), "Tamper must be detected!");
    assert_eq!(report.block_id, 0);
    assert_eq!(report.tampered_records.len(), 1);

    let corrupted = &report.tampered_records[0];
    assert_eq!(corrupted.leaf_index, tampered_leaf as u32);
    assert_eq!(corrupted.stored_raw_hash, records[tampered_leaf].raw_hash);
    assert_ne!(corrupted.calculated_raw_hash, corrupted.stored_raw_hash);

    match &corrupted.reason {
        TamperReason::DigestMismatch {
            stored_hash,
            calculated_hash,
        } => {
            assert_eq!(stored_hash, &records[tampered_leaf].raw_hash);
            assert_eq!(calculated_hash, &corrupted.calculated_raw_hash);
        }
        other => panic!("Expected DigestMismatch, got {:?}", other),
    }

    // Verify format_alert output
    let alert_str = report.format_alert();
    assert!(alert_str.contains("ALARM: FORENSIC TAMPER DETECTED"));
    assert!(alert_str.contains("Leaf #27"));
}

#[test]
fn test_forensic_tamper_detection_ip_alteration() {
    let _exclusive_io = exclusive_flush_env();
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir,
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let mut batcher = BatchAccumulator::new(config).expect("init");
    for log in generate_logs(20) {
        batcher
            .push(IncomingLog::new("cisco_asa", log))
            .expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    // Tamper leaf #12: alter IP address inside raw log
    let mut records = read_parquet_file(&flush.parquet_path).expect("read");
    let target_leaf = 12;
    records[target_leaf].raw_log = records[target_leaf]
        .raw_log
        .replace("192.168.1.", "10.99.99.");

    write_records_to_parquet(&flush.parquet_path, &records, ParquetCompression::Snappy)
        .expect("write");

    let report = verify_block_with_ledger(&flush.parquet_path, &ledger_path).expect("verify");
    assert!(report.is_tampered());
    assert_eq!(report.tampered_records[0].leaf_index, target_leaf as u32);
}

#[test]
fn test_forensic_tamper_detection_row_deletion() {
    let _exclusive_io = exclusive_flush_env();
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir,
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let mut batcher = BatchAccumulator::new(config).expect("init");
    for log in generate_logs(30) {
        batcher
            .push(IncomingLog::new("pfsense", log))
            .expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    // Tamper: adversary deletes leaf #10 entirely
    let mut records = read_parquet_file(&flush.parquet_path).expect("read");
    records.remove(10); // Now 29 records instead of 30

    write_records_to_parquet(&flush.parquet_path, &records, ParquetCompression::Snappy)
        .expect("write");

    let report = verify_block_with_ledger(&flush.parquet_path, &ledger_path).expect("verify");
    assert!(report.is_tampered());
    assert_eq!(report.expected_records, 30);
    assert_eq!(report.actual_records, 29);

    // Should detect CountDiscrepancy and IndexMismatch for subsequent rows
    let has_count_err = report.tampered_records.iter().any(|r| {
        matches!(
            r.reason,
            TamperReason::CountDiscrepancy {
                expected_count: 30,
                actual_count: 29
            }
        )
    });
    assert!(has_count_err, "Must report CountDiscrepancy");
}

#[test]
fn test_forensic_tamper_detection_hash_recalculation_attack() {
    let _exclusive_io = exclusive_flush_env();
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir,
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let mut batcher = BatchAccumulator::new(config).expect("init");
    for log in generate_logs(25) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    // Sophisticated adversary modifies raw_log AND re-computes raw_hash in Parquet
    let mut records = read_parquet_file(&flush.parquet_path).expect("read");
    records[5].raw_log = "INJECTED MALICIOUS EVENT".to_string();
    records[5].raw_hash = hex::encode(sha2::Sha256::digest(records[5].raw_log.as_bytes()));

    write_records_to_parquet(&flush.parquet_path, &records, ParquetCompression::Snappy)
        .expect("write");

    // Individual hash check matches, BUT Merkle root fails against anchored ledger!
    let report = verify_block_with_ledger(&flush.parquet_path, &ledger_path).expect("verify");
    assert!(report.is_tampered());
    assert_ne!(report.computed_merkle_root, report.ledger_merkle_root);

    let has_root_mismatch = report
        .tampered_records
        .iter()
        .any(|r| matches!(r.reason, TamperReason::MerkleRootMismatch { .. }));
    assert!(
        has_root_mismatch,
        "Must flag MerkleRootMismatch when attacker recalculates raw_hash"
    );
}

#[test]
fn test_rfc6962_consistency_proof() {
    let logs = generate_logs(30);

    // Initial snapshot of size 12
    let tree_12 = MerkleTree::from_raw_logs(logs[..12].iter().map(|s| s.as_bytes()));
    // Grown snapshot of size 30
    let tree_30 = MerkleTree::from_raw_logs(logs.iter().map(|s| s.as_bytes()));

    let proof = tree_30.consistency_proof(12).expect("consistency proof");
    assert!(!proof.is_empty());

    let is_consistent = verify_consistency_proof(12, 30, &tree_12.root(), &tree_30.root(), &proof);
    assert!(is_consistent, "RFC 6962 consistency proof must verify!");

    // Inconsistent snapshot check
    let fake_root = Hash::from_bytes([0x42; 32]);
    let invalid = verify_consistency_proof(12, 30, &fake_root, &tree_30.root(), &proof);
    assert!(
        !invalid,
        "Consistency check must fail with invalid prev_root"
    );
}

#[test]
fn test_batcher_passes_through_parser_raw_hash() {
    let _exclusive_io = exclusive_flush_env();
    // The whole point of issue #4: a parser-supplied digest must land in
    // Parquet untouched — no second hash, byte-identical output.
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir,
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let logs = generate_logs(5);
    let mut batcher = BatchAccumulator::new(config).expect("init");
    for log in &logs {
        // Same unprefixed hex SHA-256 the parser computes for this line.
        let parser_hash = hex::encode(sha2::Sha256::digest(log.as_bytes()));
        batcher
            .push(IncomingLog::new("cisco", log.clone()).with_raw_hash(parser_hash))
            .expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    let records = read_parquet_file(&flush.parquet_path).expect("read");
    assert_eq!(records.len(), 5);
    for (rec, log) in records.iter().zip(logs.iter()) {
        assert_eq!(
            rec.raw_hash,
            hex::encode(sha2::Sha256::digest(log.as_bytes())),
            "pass-through hash must survive flush byte-identical"
        );
    }

    // And the passed-through block still verifies against its ledger root.
    let report = verify_block_with_ledger(&flush.parquet_path, &ledger_path).expect("verify");
    assert!(report.is_valid, "pass-through block must verify clean");
}

#[test]
fn test_batcher_missing_raw_hash_falls_back_to_recompute() {
    let _exclusive_io = exclusive_flush_env();
    // Old callers construct IncomingLog without a hash (new() leaves it
    // None): flush must hash the bytes itself, exactly as before #4.
    let temp_dir = TempDir::new().expect("create temp dir");
    let storage_dir = temp_dir.path().join("parquet");
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir,
        ledger_path: ledger_path.clone(),
        compression: ParquetCompression::Snappy,
    };

    let logs = generate_logs(3);
    let mut batcher = BatchAccumulator::new(config).expect("init");
    for log in &logs {
        batcher
            .push(IncomingLog::new("cisco", log.clone()))
            .expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    let records = read_parquet_file(&flush.parquet_path).expect("read");
    assert_eq!(records.len(), 3);
    for (rec, log) in records.iter().zip(logs.iter()) {
        assert_eq!(
            rec.raw_hash,
            hex::encode(sha2::Sha256::digest(log.as_bytes())),
            "None fallback must recompute the parser-identical digest"
        );
    }

    let report = verify_block_with_ledger(&flush.parquet_path, &ledger_path).expect("verify");
    assert!(report.is_valid, "fallback block must verify clean");
}

#[test]
fn test_batcher_rejects_malformed_hash_with_buffer_intact() {
    let _exclusive_io = exclusive_flush_env();
    // A garbage supplied hash must fail at the door — before buffering or
    // draining — so the batch stays recoverable and a retry just works.
    let temp_dir = TempDir::new().expect("create temp dir");
    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir: temp_dir.path().join("parquet"),
        ledger_path: temp_dir.path().join("ledger.jsonl"),
        compression: ParquetCompression::Snappy,
    };

    let log = generate_logs(1).pop().expect("one log");
    let good_hash = hex::encode(sha2::Sha256::digest(log.as_bytes()));
    let mut batcher = BatchAccumulator::new(config).expect("init");

    batcher
        .push(IncomingLog::new("cisco", log.clone()).with_raw_hash(good_hash.clone()))
        .expect("good push");

    // Non-hex and short-but-hex are both malformed; neither may drain the batch.
    for bad in ["zz-top-not-hex!!", "deadbeef", "abc"] {
        let err = batcher
            .push(IncomingLog::new("cisco", log.clone()).with_raw_hash(bad))
            .expect_err("malformed hash must be rejected");
        assert!(
            err.to_string().contains("raw_hash"),
            "error should name the culprit, got: {err}"
        );
        assert_eq!(
            batcher.pending_count(),
            1,
            "rejected push must leave the buffer untouched"
        );
    }

    // Retry with the right digest lands fine, and the block verifies.
    batcher
        .push(IncomingLog::new("cisco", log.clone()).with_raw_hash(good_hash))
        .expect("retry push");
    assert_eq!(batcher.pending_count(), 2);
    let flush = batcher.flush().expect("flush").expect("flushed");
    assert_eq!(flush.leaf_count, 2);
}

#[test]
#[should_panic(expected = "contract violated")]
fn test_batcher_wrong_digest_trips_debug_contract() {
    let _exclusive_io = exclusive_flush_env();
    // 64 valid hex chars, but not the hash of this line: format checks pass,
    // so only the debug trust-contract assert catches it (test builds panic).
    let temp_dir = TempDir::new().expect("create temp dir");
    let config = BatcherConfig {
        max_batch_size: 1_000,
        max_batch_duration_ms: 5_000,
        storage_dir: temp_dir.path().join("parquet"),
        ledger_path: temp_dir.path().join("ledger.jsonl"),
        compression: ParquetCompression::Snappy,
    };

    let log = generate_logs(1).pop().expect("one log");
    let mut batcher = BatchAccumulator::new(config).expect("init");
    batcher
        .push(IncomingLog::new("cisco", log).with_raw_hash("0".repeat(64)))
        .expect("format-valid push");
    let _ = batcher.flush();
}

#[test]
fn test_fault_post_parquet_pre_sync_leaves_no_ledger_line() {
    // Crash between the Parquet write and its sync: the block file is on
    // disk but the ledger must stay silent — a line without a synced block
    // behind it would be a proof pointing at nothing.
    let _exclusive_io = exclusive_flush_env();
    let (temp_dir, mut batcher) = scratch_batcher(1_000, 5_000);
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    for log in generate_logs(3) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    let _fault = FaultEnv::set("ULPF_FAULT_POST_PARQUET_PRE_SYNC");
    batcher
        .flush()
        .expect_err("injected fault must abort the flush");
    drop(_fault);

    // Parquet landed; the ledger never heard about it.
    assert!(temp_dir.path().join("parquet/block_00000.parquet").exists());
    assert_no_orphan_ledger_lines(&ledger_path);
    assert!(
        BatchAccumulator::load_ledger_entries(&ledger_path)
            .unwrap_or_default()
            .is_empty(),
        "no ledger line may precede its synced block"
    );

    // Recovery is a plain retry: the next flush anchors cleanly.
    batcher
        .push(IncomingLog::new("cisco", generate_logs(1)[0].clone()))
        .expect("push");
    let flush = batcher.flush().expect("flush").expect("flushed");
    assert!(flush.parquet_path.exists());
    assert_no_orphan_ledger_lines(&ledger_path);
}

#[test]
fn test_fault_pre_ledger_sync_keeps_parquet_durable_without_entry() {
    // Crash after the Parquet sync but before the ledger append: durable
    // block, still no entry. Same invariant, later kill point.
    let _exclusive_io = exclusive_flush_env();
    let (temp_dir, mut batcher) = scratch_batcher(1_000, 5_000);
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    for log in generate_logs(4) {
        batcher
            .push(IncomingLog::new("paloalto", log))
            .expect("push");
    }
    let _fault = FaultEnv::set("ULPF_FAULT_PRE_LEDGER_SYNC");
    batcher
        .flush()
        .expect_err("injected fault must abort the flush");
    drop(_fault);

    assert!(temp_dir.path().join("parquet/block_00000.parquet").exists());
    assert_no_orphan_ledger_lines(&ledger_path);

    batcher
        .push(IncomingLog::new("paloalto", generate_logs(1)[0].clone()))
        .expect("push");
    let flush = batcher.flush().expect("flush").expect("flushed");
    let report =
        verify_block_with_ledger(&flush.parquet_path, &ledger_path).expect("verify retried block");
    assert!(report.is_valid, "post-crash retry must verify clean");
    assert_no_orphan_ledger_lines(&ledger_path);
}

#[test]
fn test_fsync_pair_cost_is_logged_and_tiny() {
    // The durable pair reports its own cost so the flush thread can log it:
    // crash safety must stay in the noise next to a millisecond ingest
    // budget. Generous bound (50 ms) — spinning disks and loaded CI exist —
    // but a regression into the hundreds of ms fails loudly here.
    let _exclusive_io = exclusive_flush_env();
    let (_temp_dir, mut batcher) = scratch_batcher(1_000, 5_000);

    for log in generate_logs(10) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");
    assert!(
        flush.fsync_micros < 50_000,
        "fsync pair took {} µs — durability must stay cheap",
        flush.fsync_micros
    );
}

#[test]
fn test_cumulative_chain_passes_on_clean_prefix() {
    // Three anchored blocks: the cumulative chain (S_0 ⊂ S_1 ⊂ S_2) must
    // verify pair by pair, with nothing skipped.
    let _exclusive_io = exclusive_flush_env();
    let (temp_dir, mut batcher) = scratch_batcher(5, 60_000);
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    for log in generate_logs(12) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    batcher.flush().expect("tail flush").expect("flushed");

    let report = ulpf_integrity::proof::check_ledger_consistency(&ledger_path)
        .expect("consistency audit runs");
    assert_eq!(report.blocks_checked, 3);
    assert_eq!(report.pairs_checked, 2);
    assert_eq!(report.pairs_passed, 2);
    assert_eq!(report.skipped_missing_files, 0);
    assert!(report.overall_valid);
    // Cumulative sizes grow by exactly one block each step.
    assert_eq!(report.pairs[0].prev_size, 5);
    assert_eq!(report.pairs[0].curr_size, 10);
    assert_eq!(report.pairs[1].prev_size, 10);
    assert_eq!(report.pairs[1].curr_size, 12);
}

#[test]
fn test_cumulative_chain_detects_rewritten_block() {
    // Rewrite the middle block's bytes without touching the ledger: its
    // pair verdicts must fail while the untouched pair still passes — the
    // chain localizes the rewrite instead of just saying "something broke".
    let _exclusive_io = exclusive_flush_env();
    let (temp_dir, mut batcher) = scratch_batcher(5, 60_000);
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    for log in generate_logs(12) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    batcher.flush().expect("tail flush").expect("flushed");

    let victim = temp_dir.path().join("parquet/block_00001.parquet");
    let mut records = read_parquet_file(&victim).expect("read victim");
    records[2].raw_log.push_str(" REWRITTEN BY ADVERSARY");
    write_records_to_parquet(&victim, &records, ParquetCompression::Snappy).expect("rewrite");

    let report = ulpf_integrity::proof::check_ledger_consistency(&ledger_path)
        .expect("consistency audit runs");
    assert_eq!(report.blocks_checked, 3);
    assert_eq!(report.pairs_checked, 2);
    assert!(
        !report.overall_valid,
        "rewritten block must break the chain"
    );
    assert!(
        report.pairs.iter().all(|p| !p.passed || p.blocks_valid),
        "no pair touching the rewritten block may pass"
    );
    assert_eq!(
        report.pairs.iter().filter(|p| p.passed).count(),
        0,
        "every pair touches block 1 here, so none may pass"
    );
}

#[test]
fn test_cumulative_chain_stops_at_first_missing_file() {
    // Delete the middle block: the walk ends at the gap (contiguous prefix
    // only) and everything from the gap on counts as skipped — never as
    // silently dropped.
    let _exclusive_io = exclusive_flush_env();
    let (temp_dir, mut batcher) = scratch_batcher(5, 60_000);
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    for log in generate_logs(12) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    batcher.flush().expect("tail flush").expect("flushed");
    std::fs::remove_file(temp_dir.path().join("parquet/block_00001.parquet"))
        .expect("delete middle block");

    let report = ulpf_integrity::proof::check_ledger_consistency(&ledger_path)
        .expect("consistency audit runs");
    assert_eq!(report.blocks_checked, 1);
    assert_eq!(report.pairs_checked, 0);
    assert_eq!(report.skipped_missing_files, 2);
    assert!(!report.overall_valid);
}

#[test]
fn test_prove_leaf_anchored_outcomes() {
    // prove_leaf against a scratch block: clean leaf verifies, tampered
    // leaf does not, OOB errors, and a block with no ledger entry still
    // returns printable JSON flagged for exit 2.
    let _exclusive_io = exclusive_flush_env();
    let (temp_dir, mut batcher) = scratch_batcher(1_000, 5_000);
    let ledger_path = temp_dir.path().join("ledger.jsonl");

    for log in generate_logs(8) {
        batcher.push(IncomingLog::new("cisco", log)).expect("push");
    }
    let flush = batcher.flush().expect("flush").expect("flushed");

    let ok = ulpf_integrity::proof::prove_leaf(&flush.parquet_path, 3, &ledger_path)
        .expect("clean prove runs");
    assert!(!ok.no_ledger_entry);
    assert!(ok.output.verified);
    assert_eq!(ok.output.block_id, 0);
    assert_eq!(ok.output.tree_size, 8);
    assert!(ok.output.ledger_merkle_root.is_some());
    assert_eq!(
        ok.output.calculated_merkle_root,
        ok.output.ledger_merkle_root.clone().unwrap()
    );

    // Tamper the bytes in place: the audit path no longer reaches the
    // anchored root, but the JSON still prints (exit 2, not exit 1).
    let mut records = read_parquet_file(&flush.parquet_path).expect("read");
    records[3].raw_log.push_str(" TAMPERED");
    write_records_to_parquet(&flush.parquet_path, &records, ParquetCompression::Snappy)
        .expect("rewrite");
    let bad = ulpf_integrity::proof::prove_leaf(&flush.parquet_path, 3, &ledger_path)
        .expect("tampered prove still returns JSON");
    assert!(!bad.output.verified);

    // Out-of-bounds leaf is a usage error.
    ulpf_integrity::proof::prove_leaf(&flush.parquet_path, 8, &ledger_path)
        .expect_err("leaf == tree_size must error");

    // A block the ledger never heard of: full JSON, flagged no-entry.
    let orphan = ulpf_integrity::proof::prove_leaf(
        &flush.parquet_path,
        0,
        std::path::Path::new("/nonexistent-ledger.jsonl"),
    );
    assert!(orphan.is_err(), "unreadable ledger is exit 1, not exit 2");
    let empty_ledger = temp_dir.path().join("empty.jsonl");
    std::fs::write(&empty_ledger, "").expect("empty ledger");
    let missing = ulpf_integrity::proof::prove_leaf(&flush.parquet_path, 0, &empty_ledger)
        .expect("missing entry still returns JSON");
    assert!(missing.no_ledger_entry);
    assert!(!missing.output.verified);
    assert!(missing.output.ledger_merkle_root.is_none());
}
