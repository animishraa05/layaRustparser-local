# ULPF Hardcore Architectural & Accuracy Telemetry Report

**Timestamp:** 2026-09-28T23:30:12.426806964+00:00  
**Environment:** Linux (x86_64, 16 Cores) | **Workers:** 16 parallel threads  
**Duration:** 3s per engine | **Corpus:** 224657 logs (62241.99 KB)  
**Corpus Kind:** `core` (core = fixed file set, adversarial/holdout = sidecar-GT corpora)  

---

## 1. Executive Telemetry & Accuracy Scorecard

| Benchmark Dimension | Baseline (`UniversalParser`) | 3-Tier (`LRU+Drain+Laya`) | Speedup / Delta |
| :--- | :---: | :---: | :---: |
| **Throughput (EPS)** | **995247 EPS** | **1003273 EPS** | **1.01x** |
| **Data Bandwidth (MB/s)** | **265.80 MB/s** | **268.42 MB/s** | **1.01x** |
| **Median Latency (p50)** | 106.67 µs | **6.15 µs** | **-94.2%** |
| **99th %ile Latency (p99)** | 165.42 µs | **9.86 µs** | **-94.0%** |
| **Vendor Classification (VCA)** | **100.00%** | **100.00%** | Ground-Truth Exact Match |
| **Grouping Accuracy (GA %)** | **100.00%** | **100.00%** | Loghub-2.0 Standard |
| **Template Accuracy (TA %)** | **100.00%** | **100.00%** | Template Validity (generalization-correct vs masked line) |
| **Oracle GA Ceiling (GT-hash, unfair)** | **100.00%** | **100.00%** | Labeled Ceiling — Not a Fair Baseline |
| **Unique Templates (compression)** | **137986** | **32** | Strictly Fewer vs Naive Baseline (§5.2) |
| **Field Extraction Mean Accuracy** | **100.00%** | **100.00%** | IP/Port/Proto Extraction |
| **Disposition Resolution Accuracy** | **100.00%** | **100.00%** | OCSF Action Mapping |
| **Action Inviolability** | N/A | **100% PRESERVED** | `ALLOW`/`DENY` isolated |

## 1b. Corpus Robustness Scorecard (P7)

| Robustness Dimension | Baseline | 3-Tier (`LRU+Drain+Laya`) | Gate Meaning |
| :--- | ---: | ---: | :--- |
| **Lines Audited** | 224657 | 224657 | Full corpus, every line scored |
| **Format Recognized** | 224657 | 224657 | Vendor routed to a known format (not `unknown`) |
| **No Panic** | 224657 | 224657 | `catch_unwind` parse, zero aborts |
| **Lossless (SHA-256 match)** | 224657 | 224657 | `raw_hash` == SHA-256(raw), byte-exact |
| **Sidecar GT fields graded** | 1000000 | 1000000 | Non-null expectations (correct + wrong) |
| **GT fields correct** | 1000000 | 1000000 | Engine matched the expectation |
| **GT fields wrong** | 0 | 0 | Contradicted expectation (strictly worse than null) |
| **GT fields null (honest)** | 123285 | 123285 | No expectation — excluded from wrong |

## 2. Microsecond Latency Spectrum

| Percentile Observation | Baseline (µs) | 3-Tier Engine (µs) | Latency Reduction |
| :--- | :---: | :---: | :---: |
| **p1 (Fastest 1%)** | 104.02 µs | 5.16 µs | -95.0% |
| **p50 (Median)** | 106.67 µs | 6.15 µs | -94.2% |
| **p90** | 139.39 µs | 6.71 µs | -95.2% |
| **p99** | 165.42 µs | 9.86 µs | -94.0% |
| **p99.9 (Three Nines)** | 208.81 µs | 27.54 µs | -86.8% |
| **Worst Case (Max)** | 371.45 µs | 60.66 µs | -83.7% |

## 3. Academic Accuracy & Quality Breakdown

| Metric Category | Baseline Score | 3-Tier Score | Evaluation Target |
| :--- | :---: | :---: | :---: |
| **Vendor Classification Accuracy** | 100.00% | 100.00% | ≥ Baseline (parity, §5.1) |
| **LogPai Grouping Accuracy (GA %)** | 100.00% | 100.00% | ≥ Naive Baseline (§5.2) |
| **Loghub Template Accuracy (TA %)** | 100.00% | 100.00% | > Naive Baseline (§5.2); ≥ at 100% ceiling (see §5.2 amendment) |
| **Source IP Accuracy** | 100.0% | 100.0% | Ground Truth Exact; null correct only when raw lacks a marker (P5) |
| **Destination IP Accuracy** | 100.0% | 100.0% | Ground Truth Exact; null correct only when raw lacks a marker (P5) |
| **Source Port Accuracy** | 100.0% | 100.0% | Valid Port Range; null correct only when raw lacks port evidence (P5) |
| **Destination Port Accuracy** | 100.0% | 100.0% | Valid Port Range; null correct only when raw lacks port evidence (P5) |
| **Protocol Disambiguation** | 100.0% | 100.0% | OCSF 4001; null correct without protocol evidence (P5) |
| **Disposition Resolution Accuracy** | 100.00% | 100.00% | Security Invariant |
| **Lossless Cryptographic SHA-256** | 100.00% | 100.00% | 100.0% Required |

