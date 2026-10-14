# Historical reference evidence: archived, not an active gate

The 23 payloads formerly stored here were archived on 2026-10-14 to the
local-only `.codex/archive/reference-evidence-2026-10-14/`. Every byte size and
SHA256 was verified after moving; `archive-manifest.json` also records original
Git blob identities. Git commit `69c50d9` preserves the complete original tree
for other clones or recovery. No solver, test, tool or pipeline depends on these
historical payloads. Do not revive the archived Python regeneration commands.

Archived evidence includes the Python source/corpus manifest, behavior and
ordering registry captures, gates, stage/stress/performance reports, the native
migration captures and their dated acceptance/provenance documents. Original
paths mentioned in `IMPLEMENTED.md` are historical paths at that commit, not
instructions to run a missing tool. Archival changes storage, not the evidence.

Current sources of truth:

- [Native correctness/quality protocol and datasets](../quality/README.md).
- `tests/quality/native-baseline.v1.json`: retained native **pilot**, not a
  naturalness gold set; its dirty source and partial-output limitations remain.
- `cargo xtask review`: generates `.codex/docs/TAJSBENCH.md` from actual stored
  phrases and failures, with separate human judgment CSVs.
- [Native tooling and corpus provenance boundaries](../../NATIVE_TOOLS.md).
- [Completed implementation history](../../IMPLEMENTED.md).

Rust contracts, independent exhaustive/constraint tests, human judgments and
paired native experiments are the current acceptance mechanisms. Historical
score/order equality is not one.
