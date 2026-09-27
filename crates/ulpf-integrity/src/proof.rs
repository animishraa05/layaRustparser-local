//! Ledger-anchored Merkle proving for the ULPF Integrity Plane.
//!
//! Two read-only auditors live here, both shared by the `prove` /
//! `verify --consistency` CLIs and (through re-export) the serve plane:
//!
//! - [`prove_leaf`]: rebuilds a block's RFC 6962 tree, emits the audit path
//!   for one leaf, and verifies it against the **ledger** root — never the
//!   just-recomputed tree root, which would make every proof trivially true.
//! - [`check_ledger_consistency`]: walks the contiguous prefix of ledger
//!   entries whose Parquet files still exist, chaining each block's leaves
//!   into a cumulative tree and checking the RFC 6962 consistency proof of
//!   every adjacent pair. Per-block trees are independent, so a naive
//!   N→N+1 proof is impossible; the cumulative chain is the construction
//!   that makes "nothing was rewritten between blocks" checkable.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::batcher::{BatchAccumulator, LedgerEntry};
use crate::merkle::{hash_leaf, verify_consistency_proof, Hash, MerkleTree, Side};
use crate::storage::read_parquet_file;
use crate::tamper::{block_id_for_parquet, find_ledger_entry, verify_records};

// -----------------------------------------------------------------------------
// Inclusion proving (`ulpf prove`)
// -----------------------------------------------------------------------------

/// One audit-path step, serialized exactly like serve's `AuditStep`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofAuditStep {
    pub hash: String,
    pub side: String,
}

/// Ledger-anchored inclusion proof, serialized with serve's
/// `InclusionProofResponse` field names so the CLI's JSON and the HTTP API
/// stay byte-compatible for the same leaf.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InclusionProofOutput {
    pub block_id: u64,
    pub leaf_index: usize,
    pub tree_size: usize,
    pub leaf_hash: String,
    pub calculated_merkle_root: String,
    pub ledger_merkle_root: Option<String>,
    pub verified: bool,
    pub audit_path: Vec<ProofAuditStep>,
    pub standard: String,
}

/// The printable proof plus whether its block has any ledger entry at all.
/// A missing entry is not a usage error — the JSON is still complete
/// (`ledger_merkle_root: null`, `verified: false`) and the caller exits 2.
pub struct ProveOutcome {
    pub output: InclusionProofOutput,
    pub no_ledger_entry: bool,
}

/// Proves leaf `leaf_index` of the block in `parquet_path` against
/// `ledger_path`. Errors only on missing/unreadable inputs or an
/// out-of-bounds leaf (the CLI maps those to exit 1); a block with no
/// ledger entry or a proof that does not verify is a successful return
/// the caller maps to exit 2.
pub fn prove_leaf(
    parquet_path: &Path,
    leaf_index: usize,
    ledger_path: &Path,
) -> Result<ProveOutcome> {
    let records = read_parquet_file(parquet_path)
        .with_context(|| format!("Failed reading Parquet block at {:?}", parquet_path))?;
    let block_id = block_id_for_parquet(parquet_path, records.first().map(|r| r.block_id));

    if leaf_index >= records.len() {
        anyhow::bail!(
            "Leaf index {} out of bounds (block #{} has {} records)",
            leaf_index,
            block_id,
            records.len()
        );
    }

    let tree = MerkleTree::from_raw_logs(records.iter().map(|r| r.raw_log.as_bytes()));
    let calculated_root = tree.root();
    let proof = tree
        .inclusion_proof(leaf_index)
        .with_context(|| format!("Failed generating inclusion proof for leaf {}", leaf_index))?;

    let ledger_entries = BatchAccumulator::load_ledger_entries(ledger_path)
        .with_context(|| format!("Failed reading ledger at {:?}", ledger_path))?;

    // Anchor on the LEDGER root, never the recomputed one: verifying
    // against tree.root() succeeds by construction and proves nothing.
    let (ledger_root_hex, verified, no_ledger_entry) =
        match find_ledger_entry(&ledger_entries, block_id, ledger_path) {
            Ok(entry) => {
                let expected = Hash::from_hex(&entry.merkle_root).with_context(|| {
                    format!(
                        "Ledger entry for block #{} holds an invalid root {:?}",
                        block_id, entry.merkle_root
                    )
                })?;
                let ok = proof.verify(records[leaf_index].raw_log.as_bytes(), &expected);
                (Some(entry.merkle_root), ok, false)
            }
            Err(_) => (None, false, true),
        };

    let audit_path = proof
        .audit_path
        .iter()
        .map(|(h, s)| ProofAuditStep {
            hash: h.to_hex(),
            side: match s {
                Side::Left => "Left".to_string(),
                Side::Right => "Right".to_string(),
            },
        })
        .collect();

    Ok(ProveOutcome {
        output: InclusionProofOutput {
            block_id,
            leaf_index,
            tree_size: records.len(),
            leaf_hash: hash_leaf(records[leaf_index].raw_log.as_bytes()).to_hex(),
            calculated_merkle_root: calculated_root.to_hex(),
            ledger_merkle_root: ledger_root_hex,
            verified,
            audit_path,
            standard: "RFC 6962 Certificate Transparency Standard".to_string(),
        },
        no_ledger_entry,
    })
}

// -----------------------------------------------------------------------------
// Cumulative consistency (`ulpf verify --consistency`)
// -----------------------------------------------------------------------------

/// Per-block audit inside the walked prefix: the raw row data plus the two
/// checks the pair verdicts build on.
struct PrefixBlock {
    entry: LedgerEntry,
    leaf_hashes: Vec<Hash>,
    row_counts_match: bool,
    block_valid: bool,
}

/// Verdict for one adjacent pair in the cumulative chain.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsistencyPair {
    pub prev_block: u64,
    pub curr_block: u64,
    pub prev_size: usize,
    pub curr_size: usize,
    pub row_counts_match: bool,
    pub blocks_valid: bool,
    pub proof_valid: bool,
    pub passed: bool,
}

/// Whole-chain verdict. `blocks_checked` is the contiguous prefix length
/// (ledger order, stopping at the first missing Parquet file);
/// `skipped_missing_files` counts every ledger entry at or past that gap.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsistencyReport {
    pub pairs: Vec<ConsistencyPair>,
    pub pairs_checked: usize,
    pub pairs_passed: usize,
    pub blocks_checked: usize,
    pub skipped_missing_files: usize,
    pub overall_valid: bool,
}

/// Resolves the Parquet file for a ledger entry: beside the ledger's
/// `parquet/` sibling directory in the default layout (and in scratch
/// datasets, where the ledger sits next to its `parquet/` dir), falling
/// back to next to the ledger itself.
fn parquet_for_entry(ledger_path: &Path, entry: &LedgerEntry) -> Option<std::path::PathBuf> {
    let parent = ledger_path.parent()?;
    [
        parent.join("parquet").join(&entry.parquet_file),
        parent.join(&entry.parquet_file),
    ]
    .into_iter()
    .find(|candidate| candidate.exists())
}

/// Checks the cumulative prefix chain described in the module docs.
///
/// Only the contiguous prefix of entries with existing Parquet files is
/// walked — a gap ends the chain (its entries count as skipped, never as
/// silently dropped). Callers exit 1 when fewer than 2 blocks were walked,
/// 0 when every pair passed, 2 otherwise.
pub fn check_ledger_consistency(ledger_path: &Path) -> Result<ConsistencyReport> {
    let entries = BatchAccumulator::load_ledger_entries(ledger_path)
        .with_context(|| format!("Failed reading ledger at {:?}", ledger_path))?;

    // Contiguous prefix only: the first missing file ends the walk, so a
    // hole can never hide a rewrite behind it.
    let mut prefix: Vec<PrefixBlock> = Vec::new();
    let mut skipped_missing_files = 0;
    for entry in &entries {
        match parquet_for_entry(ledger_path, entry) {
            Some(path) => {
                let records = read_parquet_file(&path)
                    .with_context(|| format!("Failed reading Parquet block at {:?}", path))?;
                let row_counts_match = records.len() == entry.leaf_count;
                let block_valid = verify_records(&records, entry).is_valid;
                prefix.push(PrefixBlock {
                    entry: entry.clone(),
                    leaf_hashes: records
                        .iter()
                        .map(|r| hash_leaf(r.raw_log.as_bytes()))
                        .collect(),
                    row_counts_match,
                    block_valid,
                });
            }
            None => {
                // Gap: this entry and everything after it is uncheckable.
                skipped_missing_files = entries.len() - prefix.len();
                break;
            }
        }
    }

    // Cumulative roots S_N over the concatenated prefix leaves: S_0 is the
    // first block's own root, each later S_N extends the previous prefix.
    let mut cum_leaves: Vec<Hash> = Vec::new();
    let mut cum_roots: Vec<(Hash, usize)> = Vec::new();
    for block in &prefix {
        cum_leaves.extend_from_slice(&block.leaf_hashes);
        let root = MerkleTree::from_leaf_hashes(cum_leaves.clone()).root();
        cum_roots.push((root, cum_leaves.len()));
    }

    let mut pairs = Vec::new();
    for i in 1..prefix.len() {
        let (prev_root, prev_size) = cum_roots[i - 1];
        let (curr_root, curr_size) = cum_roots[i];
        let curr_tree = MerkleTree::from_leaf_hashes(cum_leaves[..curr_size].to_vec());
        let proof_valid = curr_tree
            .consistency_proof(prev_size)
            .ok()
            .map(|proof| {
                verify_consistency_proof(prev_size, curr_size, &prev_root, &curr_root, &proof)
            })
            .unwrap_or(false);
        let row_counts_match = prefix[i - 1].row_counts_match && prefix[i].row_counts_match;
        let blocks_valid = prefix[i - 1].block_valid && prefix[i].block_valid;
        pairs.push(ConsistencyPair {
            prev_block: prefix[i - 1].entry.block_id,
            curr_block: prefix[i].entry.block_id,
            prev_size,
            curr_size,
            row_counts_match,
            blocks_valid,
            proof_valid,
            passed: row_counts_match && blocks_valid && proof_valid,
        });
    }

    let pairs_checked = pairs.len();
    let pairs_passed = pairs.iter().filter(|p| p.passed).count();
    Ok(ConsistencyReport {
        pairs,
        pairs_checked,
        pairs_passed,
        blocks_checked: prefix.len(),
        skipped_missing_files,
        overall_valid: prefix.len() >= 2 && pairs_passed == pairs_checked,
    })
}
