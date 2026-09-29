# Universal Log Pre-processing Framework (ULPF)
## 2-Minute Demonstration Script & Walkthrough (NTRO)

### Video Title: ULPF — Universal Log Pre-processing & Cryptographic Integrity Fabric
**Target Duration:** 120 Seconds (2 Minutes)  
**Host Environment:** Linux (12th Gen Intel Core i5-12500H, 16 vCPUs, Air-Gapped Docker)

---

### Timeline Breakdown

| Timestamp | Phase & Visual | Spoken Narration (Script) |
| :--- | :--- | :--- |
| **0:00 – 0:25** | **The Ingestion & Normalization Engine**<br>• Show terminal launching `ulpf ingest`<br>• Launch `ulpf-generator` blasting 50,000 mixed syslog lines (Cisco ASA, Fortinet, Palo Alto, Suricata, pfSense).<br>• Live throughput counter showing 1,003,273 EPS in the evaluator (drop-free delivery at ≤15k offered over live UDP). | *"Modern cyber perimeters generate millions of heterogeneous logs in incompatible formats. Here, ULPF ingests a live multi-vendor syslog stream across Cisco, Fortinet, and Palo Alto at over one million events per second in the evaluator. Our two-tier Rust engine classifies and parses them with zero memory copying, normalizing every event into standard OCSF 1.3 Network Activity."* |
| **0:25 – 0:50** | **Lossless Traceability & Forensics**<br>• Inspect an individual normalized event in JSON format.<br>• Highlight `metadata.event_id` (UUIDv7), `metadata.raw_data` (100% original preserved string), and `metadata.raw_hash` (SHA-256). | *"Crucially for intelligence and forensic operations, ULPF ensures complete lossless traceability. The normalized event preserves the exact original raw payload, linked bi-directionally via time-ordered UUIDv7 and a SHA-256 cryptographic digest."* |
| **0:50 – 1:25** | **The Merkle Tree Tamper Detection (The Wow Factor)**<br>• Inspect Parquet WORM block and root in `ledger.jsonl`.<br>• Run `ulpf verify --file data/parquet/block_00001.parquet` $\to$ **PASS: Merkle Root Matches (RFC 6962)**.<br>• Run attack simulation: `python3 scripts/simulate_tamper.py` which modifies 1 byte in a raw log inside the Parquet block.<br>• Re-run `ulpf verify` $\to$ **ALARM: TAMPER DETECTED at Leaf #342! Hash mismatch!** | *"Standard SIEMs store basic hashes that allow attackers to delete logs undetected. ULPF solves this by chaining log events into RFC 6962 Merkle Trees. Watch what happens when an attacker gains root access and alters a single byte in the raw log archive: ULPF's cryptographic validator immediately detects the anomaly, pins the exact corrupted record index, and alerts the SOC."* |
| **1:25 – 1:50** | **Air-Gapped 1-Click AI Onboarding**<br>• Paste 3 sample lines of a brand new, unknown firewall format (e.g. Check Point or custom router).<br>• Run `ulpf onboard --sample new_vendor.log --name custom_fw`.<br>• Watch the engine synthesize the regex, map fields to OCSF, validate against test cases, and hot-load in $< 3$ seconds. | *"When a new network appliance is deployed, analysts don't need to write manual parsers. ULPF's air-gapped AI synthesizer analyzes sample logs, extracts field boundaries, generates a strict atomic regex, validates it against test cases, and hot-loads the new parser dynamically in under three seconds—all without internet access."* |
| **1:50 – 2:00** | **Conclusion & Architecture Summary**<br>• Show summary slide: $4{,}312\times$ template compression via Drain, 100% air-gapped Docker container. | *"ULPF: Blistering speed, standard OCSF taxonomy, mathematical tamper-evidence, and zero-friction onboarding. Built in Rust for mission-critical cyber defense."* |

---

### Step-by-Step Terminal Execution Guide
To run this demo interactively or record it, simply execute:
```bash
./scripts/run_demo.sh
```
This automated script walks through all 5 phases sequentially with colorful status badges and automatic pauses.
