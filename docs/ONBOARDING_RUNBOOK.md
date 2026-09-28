# Onboarding Runbook — New Firewall in Production

**How-to guide.** Follow these steps, in order, to take a brand-new firewall
from raw sample lines to parsed output. No Rust knowledge required. No theory
below — for background, see the reference docs linked at the end.

**Time:** ~10 minutes. **Commands:** 3. **Network:** none required — every step
is fully air-gapped.

> Rule 0: `ulpf --help` is authoritative. If this runbook and `--help` ever
> disagree, trust `--help`.

---

## What you need before you start

- The `ulpf` binary built: `cargo build --release` (run from the repo root).
- Shell access to the machine that runs `ulpf` (ingest host or analyst box).
- 3–5 sample log lines from the new firewall (Step 1).

---

## Step 1 — Collect 3–5 representative sample lines

Save the samples as a plain-text file, one log line per line, e.g.
`samples/new_fw.log`.

**What "representative" means — check all four:**

1. **Every message type.** If the firewall emits CREATE, CLOSE, and DENY
   lines, include at least one of each. A parser trained only on CREATE lines
   may silently miss DENY lines in production.
2. **Both allow and deny outcomes.** Include at least one permitted session
   and one denied/dropped session so the allow-vs-deny mapping gets validated.
3. **Real addresses and ports.** Use production-format IPs, ports, and zone
   names. Do not redact fields into `XXX` — redacted samples teach the
   synthesizer the wrong shape.
4. **At least 3 lines.** The onboarder refuses fewer than 3. (Note: with
   fewer than 20 samples the validation gate is effectively 100% — every
   line must match. That is normal, not a bug. If you can, collect 20+
   lines to get the true 95% gate.)

**Minimum 3 lines; 5 covering all message types is the sweet spot.**

---

## Step 2 — Run `ulpf onboard`

From the repo root:

```bash
./target/release/ulpf onboard --sample samples/new_fw.log --vendor <name> --model <model> --out data/parsers
```

| Flag | Meaning | Example |
| :--- | :--- | :--- |
| `-s, --sample` | Path to your sample-lines file (required) | `--sample samples/new_fw.log` |
| `-v, --vendor` | Vendor / device family name (lowercase, no spaces) | `--vendor juniper` |
| `-m, --model` | Specific device model | `--model srx` |
| `-o, --out` | Directory for the generated parser spec (default `data/parsers`) | `--out data/parsers` |

(Verify: `cargo run --release -p ulpf-cli -- onboard --help`.)

A successful run prints a banner, the validation pass rate, the synthesized
regex, and a first-sample OCSF check. It writes **two files** — a JSON spec
(the loader reads it) and a YAML twin (for humans):

```text
data/parsers/<vendor>.json
data/parsers/<vendor>.yaml
```

---

## Step 3 — Read the validation %

The line that matters:

```text
Validation Pass Rate: 100.0% (5 of 5 samples passed)
```

| What you see | What it means | What to do |
| :--- | :--- | :--- |
| **100% (or ≥ 95% with 20+ samples)** | Parser matches your samples. | Deploy (Step 4). |
| **Below 95% (20+ samples), or any failure with < 20 samples** | `ulpf onboard` exits with an error and writes **no spec files**. Your samples contain at least two different line shapes. | Split the samples by shape (e.g. session lines vs alarm lines) and onboard each shape as its own `--vendor` name. |
| **100% but the regex looks too loose** (e.g. it matches lines from a *different* vendor's log) | The synthesizer over-matched: your samples were too uniform, so optional fields became wildcards. | Add more varied samples (different ports, zones, policies, message types) and re-run. Confirm with the dry-run test in Step 4 — feed it a foreign-vendor line and expect `matched: false`. |
| **The format is JSON, CEF, or needs per-field logic** (nested objects, conditional mapping, checksums) | A regex is the wrong tool. | Do **not** force the onboarder. A hand-written extractor (a small native parser in the codebase) is the right answer — file an issue with your sample file attached and the parser team will add it. The onboarded regex path is for flat, positional/KV syslog-style lines. |

---

## Step 4 — Deploy the spec

You have two options. **Hot-load is preferred** — zero downtime.

### Option A — Hot-load into a running `ulpf serve` (preferred)

`POST /onboard` with `confirm: true`. The server writes the JSON+YAML pair
and registers the parser in active memory immediately (response `201`,
`"status": "hot_loaded"`). No restart.

```bash
# 1. Build the request (replace vendor/model/lines with yours)
python3 -c "import json; print(json.dumps({
  'vendor': 'juniper',
  'device_model': 'srx',
  'sample_lines': [l.rstrip('\n') for l in open('samples/new_fw.log')],
  'confirm': True
}))" > /tmp/onboard_req.json

# 2. Hot-load (default serve address; adjust host/port to yours)
curl -X POST http://127.0.0.1:8080/onboard \
  -H 'Content-Type: application/json' -d @/tmp/onboard_req.json
```

> **Dry run first (recommended):** send the same request with `"confirm": false`.
> You get `200` + `"status": "preview"` and **nothing is written to disk**.
> Use preview to check the regex and validation % before committing.

Request shape reference: `docs/openapi.yaml` (`POST /onboard`:
`vendor`, `device_model`, `sample_lines`, `confirm`).

### Option B — Restart (ingest hosts without `serve`)

1. Copy the two spec files into the parsers directory on the ingest host
   (see Step 5 for the path caveat).
2. Restart `ulpf ingest` (or `ulpf serve` — it scans the parsers directory
   on startup and loads every `*.json` / `*.yaml` spec automatically).

### Verify the parser is active

```bash
# Must list your vendor as dynamic_onboarded + active:
curl -s http://127.0.0.1:8080/parsers | python3 -m json.tool | grep -A3 juniper
```

Then dry-run one live line against it — expect `matched: true` and the right
disposition — without writing anything to disk (`POST /parsers/test` never
writes):

```bash
curl -s -X POST http://127.0.0.1:8080/parsers/test \
  -H 'Content-Type: application/json' \
  -d '{"raw_log": "<paste one real line>", "vendor": "juniper"}'
```

Confirm on live traffic: watch your new vendor appear in the dashboard vendor
mix (`GET /metrics`) or query its records (`GET /blocks/{id}/records?vendor=juniper`).

---

## Step 5 — Where the spec files live (read this before deploying)

`data/parsers/*.json` and `data/parsers/*.yaml` are **gitignored** — specs
produced on your machine are **not** tracked by git and do **not** travel
with a `git push` / fresh clone. This is deliberate (specs are deployment
artifacts, not source), but it bites during deployment:

- **Deploying to another host?** Copy the two files explicitly (`scp`,
  config management, baked into your container image, or mounted volume).
  Point the receiver at them with `--parsers-dir` (`ulpf serve`) or place
  them in `data/parsers/` before starting `ulpf ingest`.
- **Fresh clone missing your parser?** That is expected. Re-run Step 2 from
  your sample file, or restore the files from backup.
- **Keep your sample file.** It is the reproducible source of the spec —
  commit it (e.g. next to `sample_new_firewall.log` at the repo root) so
  anyone can regenerate the parser.

---

## Worked example end-to-end (real output)

Using the tracked sample file `sample_new_firewall.log` (5 Juniper-style
`RT_FLOW` lines: CREATE, CLOSE, DENY). Command:

```bash
./target/release/ulpf onboard --sample sample_new_firewall.log --vendor juniper --model srx --out /tmp/ulpf_runbook_demo
```

Real output (timing varies run to run):

```text
====================================================================
              ULPF Air-Gapped AI Device Onboarder
====================================================================
  Sample Log Path : sample_new_firewall.log
  Target Vendor   : juniper
  Device Model    : srx
  Output Directory: /tmp/ulpf_runbook_demo
  Environment     : 100% Air-Gapped (Zero External Cloud Dependencies)
--------------------------------------------------------------------
[*] Ingested 5 sample raw events for pattern analysis...
[+] Synthesis completed in 8.51ms
  Validation Pass Rate: 100.0% (5 of 5 samples passed)
  Synthesized Regex   : ^RT_FLOW:\s+(?P<event_type>\S+)\s+session\s+(?P<action_verb>\w+)(?:.*?)\s+(?P<src_ip>[0-9a-fA-F.:%]+)/(?P<src_port>\d{1,5})->(?P<dst_ip>[0-9a-fA-F.:%]+)/(?P<dst_port>\d{1,5})\s+\S+\s+\S+\s+(?P<protocol>\d+)\s+(?P<policy>\S+)\s+(?P<src_zone>\S+)\s+(?P<dst_zone>\S+)(?:.*)$
[+] Parser specification exported successfully:
    JSON : /tmp/ulpf_runbook_demo/juniper.json
    YAML : /tmp/ulpf_runbook_demo/juniper.yaml

[SUCCESS] First sample normalized to OCSF 1.3:
  Activity Name : Open
  Disposition   : Allowed
  Source EP     : 192.168.10.55:Some(49152)
  Dest EP       : 10.0.0.1:Some(443)
  Protocol      : Some("TCP")
```

Deploy via hot-load and verify (same samples, `POST /onboard` with
`confirm: true` → `201`, `"status": "hot_loaded"`; files `juniper.json` +
`juniper.yaml` written to the parsers dir):

```text
GET /parsers  →  juniper  dynamic_onboarded  active
```

Dry-run one DENY line (`POST /parsers/test`, writes nothing):

```text
matched: true, vendor: juniper, disposition: Blocked,
src 203.0.113.88:61234 (untrust) → dst 10.0.0.80:8080 (dmz), TCP
```

Done: samples → validated spec → live parser, three commands
(`onboard`, `POST /onboard`, `GET /parsers`).

---

## Reference (not steps)

- API contract: `docs/CONTRACTS.md` (§`POST /onboard`, `GET /parsers`,
  `POST /parsers/test`) and `docs/openapi.yaml`.
- Demo script: `docs/DEMO.md`; scripted end-to-end: `scripts/run_demo.sh`.
- Architecture background: `docs/ARCHITECTURE_FINAL.md` (device-onboarding row).
- Requirement traceability: `docs/archive/SIH_EVALUATION_DOSSIER.md` (item e).
