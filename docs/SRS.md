# ULPF Software Requirements Specification (NTRO / SIH26156)

Traceability from each NTRO requirement to design, code, test, and measured result.
This document is the canonical requirements verdict. The short matrix in the root
`README.md` mirrors it; the older tables in `docs/archive/SIH_EVALUATION_DOSSIER.md`
(§4) and `docs/ARCHITECTURE_FINAL.md` (§4) are superseded or corrected to match.

- **Requirement text:** quoted verbatim from the NTRO problem statement via the
  dossier's §4 rows. Never restated.
- **Measured results:** every number cites a committed `benchmarks/eval_*_report.md`
  §1/§1b row. Core/adversarial/full numbers are the 2026-09-28 re-measure (#46);
  the holdout report is frozen at 2026-09-24 by design and never re-run.
- **Timing rows** follow the benchmark ritual (`AGENTS.md` Gotchas): same-run
  baseline-vs-tiered ratios only; absolute µs are not comparable across machines.
- **Weak results stay in.** Where a number is bad, the status says so and the
  Honest-limitations framing is cited, not hidden.

## Verdicts at a glance

| Req | One-line verdict | Status |
| :--- | :--- | :---: |
| (a) | Raw bytes preserved + SHA-256 on every line, all corpora | done |
| (b) | Per-vendor extractors, 100% fields on core + full | done |
| (c) | OCSF 1.3 `NetworkActivity` 4001, VCA 100% core + full | done |
| (d) | UUIDv7 event ID + raw SHA-256 on every event | done |
| (e) | 3–5 sample lines → validated parser, offline, no restart | done |
| (f) | 5 vendor families → one JSON + Parquet schema | done |
| (g) | Parquet WORM blocks, 186/186 verify PASS, exit-code contract | done |
| (h) | 32 templates from 224,657 lines, inviolability 100%; −1.59 pt GA edge on fuzz disclosed | done |
| (i) | Days of regex work → automated ms-scale synthesis + validation | done |
| (j) | Zero network dependencies, loopback-only serve default | done |
| (k) | Binary under target; container image over target | partial |

## §3 Requirement traceability

### §3.a — Preserve complete raw event data without information loss.

| | |
| :--- | :--- |
| **Our design** | Raw bytes ride along untouched in `metadata.raw_data`; `metadata.raw_hash` is SHA-256(raw), recomputed and checked per line. Unknown-vendor lines fall back to lossless parse, never drop. |
| **Implementing files** | `crates/ulpf-core/src/schema/ocsf.rs`, `crates/ulpf-core/src/parser/mod.rs`, `crates/ulpf-integrity/src/storage.rs` (`raw_log`, `raw_hash` columns) |
| **Proving tests** | `test_lossless_preservation_and_cryptographic_hashes`, `test_universal_parser_sha256_and_uuid`, `test_fallback_lossless_parsing_for_unknown_log` |
| **Measured result** | Lossless SHA-256 match: 1720/1720 core, 224657/224657 full, 757/757 adversarial, 200/200 holdout (`eval_*_report.md` §1b) |
| **Status** | done |

### §3.b — Extract and parse source-specific attributes.

| | |
| :--- | :--- |
| **Our design** | One zero-copy byte-slice extractor per vendor family (IPs, ports, protocols, zones, session IDs, action verbs). No heap allocation on the hot path. |
| **Implementing files** | `crates/ulpf-core/src/parser/extractors/` (`cisco_asa.rs`, `fortigate.rs`, `paloalto.rs`, `pfsense.rs`, `suricata.rs`, `cef.rs`, `mod.rs`) |
| **Proving tests** | `test_cisco_asa_parsing_suite`, `test_fortinet_parsing_suite`, `test_palo_alto_parsing_suite`, `test_pfsense_parsing_suite`, `test_suricata_parsing_suite`, `test_cef_parsing_suite` |
| **Measured result** | Mean field accuracy 100% core + full; 97.15% adversarial (fuzz-mutated markers, both engines identical — engine delta 0) (`eval_hardcore_report.md`, `eval_full_report.md`, `eval_adversarial_report.md` §1) |
| **Status** | done |

### §3.c — Normalize fields into a common event taxonomy.

| | |
| :--- | :--- |
| **Our design** | Every vendor maps to OCSF 1.3 `NetworkActivity` (class UID 4001) with unified dispositions (`Allowed`, `Blocked`, `Dropped`). The classifier routes vendors in one Aho-Corasick pass. |
| **Implementing files** | `crates/ulpf-core/src/schema/ocsf.rs`, `crates/ulpf-core/src/parser/classifier.rs` |
| **Proving tests** | `test_classify_all_vendors`, `test_gt_disposition_ocfs_alignment` |
| **Measured result** | VCA 100% core + full; 96.30% adversarial (mutated vendor prefixes, baseline parity); disposition 100% core + full, 93.53% adversarial (Sept-28 §1 rows). Holdout VCA 40% tiered vs 0% baseline — unseen vendors, disclosed in §3.j context below. |
| **Status** | done |

### §3.d — Maintain traceability between normalized and original events.

| | |
| :--- | :--- |
| **Our design** | Every event carries a time-ordered UUIDv7 `event_id` plus the SHA-256 of its exact raw bytes. `ulpf inspect` walks ID → raw → OCSF for any event. |
| **Implementing files** | `crates/ulpf-core/src/schema/ocsf.rs`, `crates/ulpf-core/src/parser/mod.rs` |
| **Proving tests** | `test_universal_parser_sha256_and_uuid` (asserts UUID version 7 + hash match) |
| **Measured result** | Lossless binding holds on all 227,334 graded lines across the four corpora (§1b rows); `inspect` demo in README quick start 6 |
| **Status** | done |

### §3.e — Plug-and-play onboarding of new log sources.

| | |
| :--- | :--- |
| **Our design** | Deterministic heuristic synthesizer: 3–5 sample lines → token boundaries → named-group regex → sandbox validation → hot-load via `DynamicParserRegistry`, no restart, no network. |
| **Implementing files** | `crates/ulpf-ai/src/onboarder.rs`, `docs/ONBOARDING_RUNBOOK.md` (3-command operator procedure with worked example) |
| **Proving tests** | `test_onboarder_synthesize_juniper_srx`, `test_dynamic_parser_registry`, `test_tier3_onboards_novel_cluster_with_three_exemplars`, `test_validation_threshold_95_percent_with_warnings` |
| **Measured result** | Synthesized Juniper SRX parser passes its validation gate in-test; the "3.88 ms" figure is a demo measurement quoted in the dossier and `ARCHITECTURE_FINAL.md` §10 — no gated timing test asserts it, so treat it as indicative, not guaranteed |
| **Status** | done (timing figure ungated — stated, not hidden) |

### §3.f — Unified visibility across enterprise environments.

| | |
| :--- | :--- |
| **Our design** | All five vendor families land in one OCSF JSON shape and one Parquet schema, ready for centralized SIEM correlation. The classifier pins parse order so native vendors win over registry parsers. |
| **Implementing files** | `crates/ulpf-core/src/parser/mod.rs`, `crates/ulpf-core/src/parser/classifier.rs`, `crates/ulpf-integrity/src/storage.rs` |
| **Proving tests** | `test_classify_all_vendors`, `test_pinned_order_native_wins_over_registry` |
| **Measured result** | Format recognised (not `unknown`): 1720/1720 core, 224657/224657 full, 729/757 adversarial (`SCORECARDS.md` robustness table, from §1b rows) |
| **Status** | done |

### §3.g — Efficient SIEM and Data Lake integration.

| | |
| :--- | :--- |
| **Our design** | Dual-trigger batcher (1,000 events / 2,000 ms) seals Snappy Parquet WORM blocks anchored to an append-only Merkle ledger. `ulpf verify` returns machine-readable exit codes: 0 valid, 1 IO error, 2 tamper. |
| **Implementing files** | `crates/ulpf-integrity/src/storage.rs`, `crates/ulpf-integrity/src/batcher.rs`, `crates/ulpf-integrity/src/merkle.rs`, `crates/ulpf-integrity/src/proof.rs`, `crates/ulpf-integrity/src/tamper.rs` |
| **Proving tests** | `test_parquet_write_and_read_cycle`, `test_batcher_dual_trigger_count_and_ledger`, `test_forensic_tamper_detection_byte_flip`, `test_forensic_tamper_detection_ip_alteration`, `test_forensic_tamper_detection_row_deletion`, `test_forensic_tamper_detection_hash_recalculation_attack`, `test_inclusion_proof_tamper_detection`, `test_rfc6962_consistency_proof`, `test_cumulative_chain_detects_rewritten_block` |
| **Measured result** | Live full-scale chain: 186 blocks, 186/186 `verify` PASS (`FULL_DATASET_RESULTS.md` §4); in-repo tamper demo fails shut with exit 2 on the deliberately-tampered `block_00000` (`SCORECARDS.md` chain-of-custody transcript) |
| **Status** | done |

### §3.h — AI/ML-ready security and operational analytics.

| | |
| :--- | :--- |
| **Our design** | Native Rust Drain3 miner clusters log templates on CPU; structural cluster IDs ship with every event as pre-clustered ML features. Anchor tokens keep allow/deny classes apart. Laya scores Tier-3 novelty out-of-band, never blocking ingest. |
| **Implementing files** | `crates/ulpf-ai/src/drain.rs`, `crates/ulpf-ai/src/laya.rs`, `crates/ulpf-ai/src/pipeline.rs` |
| **Proving tests** | `test_drain_unique_event_patterns_anchor_tokens`, `test_drain_template_clustering_cisco_asa`, `test_drain_template_clustering_fortinet`, `test_drain_template_clustering_palo_alto`, `test_drain_anomaly_detection_unknown_template`, `test_drain_microsecond_performance`, `test_differential_baseline_vs_tiered` |
| **Measured result** | 224,657 lines → 32 templates (4,312× compression), TA 100%, inviolability 100% (Sept-28 full §1). Duel: 3-tier wins GA 4/4 rounds (`eval_duel_report.md`). Disclosed edges: adversarial GA 98.41% (−1.59 pt, deny-class variants; inviolability still 100%); R2 keeps 10/81 mixed-action clusters (vanilla 17/53) — deliberately not tuned away on the exposing corpus (README Honest-limitations, duel Disclosures). |
| **Status** | done, with the two disclosed edges above |

### §3.i — Reduced parser development effort.

| | |
| :--- | :--- |
| **Our design** | Same synthesizer as §3.e replaces days of manual regex authoring: samples in, validated parser out, with a 95% validation gate and loud failure below 20 samples. |
| **Implementing files** | `crates/ulpf-ai/src/onboarder.rs`, `docs/ONBOARDING_RUNBOOK.md` |
| **Proving tests** | `test_onboarder_synthesize_juniper_srx`, `test_validation_threshold_95_percent_with_warnings`, `test_threshold_fails_with_zero_samples`, `test_threshold_strict_below_20_samples`, `test_parse_caches_compiled_regex` |
| **Measured result** | End-to-end sample → hot-loaded parser runs in milliseconds on a laptop CPU (dossier/ARCHITECTURE_FINAL demo figure 3.88 ms; ungated — see §3.e). Effort claim is structural (automated vs manual), not a benchmark. |
| **Status** | done |

### §3.j — Deployable in an air-gapped network.

| | |
| :--- | :--- |
| **Our design** | Pure Rust workspace with no network client dependencies anywhere in the manifests; no telemetry, no model downloads, no license checks. `ulpf serve` binds loopback (`127.0.0.1:8080`) with CORS scoped to local origins. |
| **Implementing files** | `Cargo.toml`, `crates/*/Cargo.toml` (dependency surface), `crates/ulpf-cli/src/serve/mod.rs` (bind + CORS), `Dockerfile`, `docker-compose.yml` (isolated bridge) |
| **Proving tests** | `test_onboarder_synthesize_juniper_srx` (synthesis completes with no network to call), `test_serve_cors_origin_enforcement` (loopback scoping). No test asserts absence of sockets — that property is structural (dependency surface), stated as such. |
| **Measured result** | Binary runs with no outbound calls in the runtime path (AGENTS.md invariant 1; README air-gap section). Holdout disposition 0/0 is the honest cost of air-gap honesty: unseen vendors grade `Unknown` rather than guessed (`eval_holdout_report.md` §1, frozen). |
| **Status** | done |

### §3.k — Packaged in a container for platform independence.

| | |
| :--- | :--- |
| **Our design** | Multi-stage Dockerfile (Rust slim-bookworm builder → debian bookworm-slim runtime, non-root `ulpf` user, healthcheck) plus compose for local bring-up. |
| **Implementing files** | `Dockerfile`, `.dockerignore`, `docker-compose.yml` |
| **Proving tests** | None — no test builds or sizes the image. Stated, not covered up. |
| **Measured result** | Release binary ≈ 22.8 MB on disk, inside the 35 MB target. The image itself is over 35 MB (bookworm-slim + python3 + shipped docs/data). Slim-down is planned, tracked in #45. |
| **Status** | partial — binary meets the target, image does not |

## Notes for judges

- **(k) is the only partial.** The old dossier tables that mark it covered are superseded (bannered, not rewritten).
- **Throughput honesty:** tiered-vs-baseline EPS is 0.97× core, 0.90× adversarial, 1.01× full (Sept-28 same-run ratios). Small-corpus ratios below 1× are designed — Drain bookkeeping the frozen baseline skips. Latency wins at every scale (−92% to −99% p50).
- **What we did not claim:** no distributed clustering, no host-log scope, no GDPR crypto-shredding (dossier §5 roadmap stands).
