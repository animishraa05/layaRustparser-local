# ULPF Hardcore Architectural & Accuracy Telemetry Report

**Timestamp:** 2026-09-28T23:27:41.955983433+00:00  
**Environment:** Linux (x86_64, 16 Cores) | **Workers:** 16 parallel threads  
**Duration:** 3s per engine | **Corpus:** 1720 logs (565.78 KB)  
**Corpus Kind:** `core` (core = fixed file set, adversarial/holdout = sidecar-GT corpora)  

---

## 1. Executive Telemetry & Accuracy Scorecard

| Benchmark Dimension | Baseline (`UniversalParser`) | 3-Tier (`LRU+Drain+Laya`) | Speedup / Delta |
| :--- | :---: | :---: | :---: |
| **Throughput (EPS)** | **790662 EPS** | **763813 EPS** | **0.97x** |
| **Data Bandwidth (MB/s)** | **253.89 MB/s** | **245.30 MB/s** | **0.97x** |
| **Median Latency (p50)** | 106.18 µs | **3.84 µs** | **-96.4%** |
| **99th %ile Latency (p99)** | 190.96 µs | **10.26 µs** | **-94.6%** |
| **Vendor Classification (VCA)** | **100.00%** | **100.00%** | Ground-Truth Exact Match |
| **Grouping Accuracy (GA %)** | **100.00%** | **100.00%** | Loghub-2.0 Standard |
| **Template Accuracy (TA %)** | **100.00%** | **100.00%** | Template Validity (generalization-correct vs masked line) |
| **Oracle GA Ceiling (GT-hash, unfair)** | **100.00%** | **100.00%** | Labeled Ceiling — Not a Fair Baseline |
| **Unique Templates (compression)** | **1407** | **73** | Strictly Fewer vs Naive Baseline (§5.2) |
| **Field Extraction Mean Accuracy** | **100.00%** | **100.00%** | IP/Port/Proto Extraction |
| **Disposition Resolution Accuracy** | **100.00%** | **100.00%** | OCSF Action Mapping |
| **Action Inviolability** | N/A | **100% PRESERVED** | `ALLOW`/`DENY` isolated |

## 1b. Corpus Robustness Scorecard (P7)

| Robustness Dimension | Baseline | 3-Tier (`LRU+Drain+Laya`) | Gate Meaning |
| :--- | ---: | ---: | :--- |
| **Lines Audited** | 1720 | 1720 | Full corpus, every line scored |
| **Format Recognized** | 1720 | 1720 | Vendor routed to a known format (not `unknown`) |
| **No Panic** | 1720 | 1720 | `catch_unwind` parse, zero aborts |
| **Lossless (SHA-256 match)** | 1720 | 1720 | `raw_hash` == SHA-256(raw), byte-exact |
| **Sidecar GT fields graded** | 0 | 0 | Non-null expectations (correct + wrong) |
| **GT fields correct** | 0 | 0 | Engine matched the expectation |
| **GT fields wrong** | 0 | 0 | Contradicted expectation (strictly worse than null) |
| **GT fields null (honest)** | 0 | 0 | No expectation — excluded from wrong |

## 2. Microsecond Latency Spectrum

| Percentile Observation | Baseline (µs) | 3-Tier Engine (µs) | Latency Reduction |
| :--- | :---: | :---: | :---: |
| **p1 (Fastest 1%)** | 101.07 µs | 2.05 µs | -98.0% |
| **p50 (Median)** | 106.18 µs | 3.84 µs | -96.4% |
| **p90** | 148.92 µs | 6.92 µs | -95.4% |
| **p99** | 190.96 µs | 10.26 µs | -94.6% |
| **p99.9 (Three Nines)** | 276.41 µs | 26.93 µs | -90.3% |
| **Worst Case (Max)** | 440.73 µs | 73.86 µs | -83.2% |

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

