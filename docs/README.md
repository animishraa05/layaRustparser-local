# ULPF Docs — Start Here

| Doc | What it is | Who reads it |
| :--- | :--- | :--- |
| [`ARCHITECTURE_FINAL.md`](ARCHITECTURE_FINAL.md) | **Canonical architecture.** Subsystem-by-subsystem walkthrough: original proposal → why it fails in practice → what we built. **Read this first.** | Everyone, judges |
| [`DEMO.md`](DEMO.md) | 2-minute video script + terminal timeline. Run via `scripts/run_demo.sh`. | Demo / video owner |
| [`ONBOARDING_RUNBOOK.md`](ONBOARDING_RUNBOOK.md) | **Operator how-to: new firewall → parsed output in 3 commands.** Collect samples, run `ulpf onboard`, read the validation %, hot-load via `POST /onboard`, verify. Worked example included. | SOC operator, demo owner |
| [`PRESENTATION.md`](PRESENTATION.md) | 5-slide technical pitch + deliverables checklist. | Pitch owner |
| [`reference/Ulpf-proposal.pdf`](reference/Ulpf-proposal.pdf) | Original proposal document. Historical context only — implementation overruled it where noted in `ARCHITECTURE_FINAL.md`. | Curious |
| [`archive/`](archive/) | Superseded docs (stale 2-tier spec, overlapping dossiers). **Do not cite.** Kept for history. | Nobody |
| `ULPF_*.pdf` (this dir) | Generated frozen PDF exports for submission. Regenerate from `*_template.html` if content changes. | Submission |

Related, at repo root:

| File | What |
| :--- | :--- |
| [`../README.md`](../README.md) | Operational guide: quickstart, usage steps, crate map |
| [`../AGENTS.md`](../AGENTS.md) | Agent handbook: invariants, verification gate, gotchas, extension recipes |
| [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | Team workflow: branches, gate, PR checklist |
| [GitHub issues](https://github.com/guptchar/layaRustparser/issues) | Open work tracker (replaces the retired `remainingStuff.md` roadmap) |
| [`../futurescope.md`](../futurescope.md) | Measured future gaps, in priority order |

Conventions:

- Real CLI flags come from `ulpf --help`. If README and `--help` disagree, `--help` wins.
- Run binaries from the repo root (defaults assume `data/raw`, `data/parquet`, `data/ledger.jsonl`).
- `evaluate` / `benchmark` need **release** builds for meaningful numbers. Debug builds work for all subcommands since P10.0 (old duplicate `-d` clap clash fixed).
