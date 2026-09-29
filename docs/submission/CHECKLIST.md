# SIH Submission Checklist

Every line is either done (with the artifact linked) or has an owner plus a
tracking issue. No silent TODOs.

| # | Item | Status | Artifact / owner |
| :--- | :--- | :--- | :--- |
| 1 | Source repo | done | [guptchar/layaRustparser](https://github.com/guptchar/layaRustparser) |
| 2 | License | done | [`LICENSE`](../../LICENSE) (Apache-2.0), labelled on the image (`Dockerfile`) |
| 3 | Requirements traceability (SRS) | done | [`docs/SRS.md`](../SRS.md) — canonical verdicts for NTRO (a)–(k) |
| 4 | Architecture doc | done | [`docs/ARCHITECTURE_FINAL.md`](../ARCHITECTURE_FINAL.md) |
| 5 | Measured scorecards | done | [`docs/benchmarks/`](../benchmarks/) + [`docs/SCORECARDS.md`](../SCORECARDS.md) |
| 6 | Pitch script (5 slides) | done | [`docs/PRESENTATION.md`](../PRESENTATION.md), each slide cites an SRS clause |
| 7 | Pitch deck artifact (PPTX/PDF/HTML) | todo | Owner: TBD — comment on [#51](https://github.com/guptchar/layaRustparser/issues/51) to claim. deliberate: no binary decks in git without a regen path. Printable PDFs live in [`docs/releases/`](../releases/) meanwhile. |
| 8 | Demo video | todo | Owner: TBD — comment on [#51](https://github.com/guptchar/layaRustparser/issues/51) to claim. Script is done: [`docs/DEMO.md`](../DEMO.md), one-command run via `scripts/run_demo.sh`. |
| 9 | Container image < 35 MB (req k) | todo | Owner: TBD — tracked in [#45](https://github.com/guptchar/layaRustparser/issues/45). Binary meets the target (≈ 22.8 MB); image slim-down is planned. |
| 10 | Team + contact | done | 6-person SIH team, workflow in [`CONTRIBUTING.md`](../../CONTRIBUTING.md). Contact: the [issue tracker](https://github.com/guptchar/layaRustparser/issues) (reviewers `@animishraa05`, `@sudobhavik`). |

Note on items 7–8: the issue text asks for a deck artifact. Binaries do not
belong in git without a regeneration recipe, so the checklist records the deck
(and the unrecorded video) as owned work items instead of committing opaque files.
