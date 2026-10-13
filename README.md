# TajsAnagrams

Desktop anagram solver powered by the native Rust engine, with a shared Svelte UI.
The installed application does not require Python or a separate CLI executable.

- [Desktop app and local builds](apps/desktop/README.md)
- [Native CLI and data/model tools](NATIVE_TOOLS.md)
- [Implementation plan and progress](PLAN.md)
- [Native quality evaluation and experiment datasets](tests/quality/README.md)

## Repository layout

- `crates/`: native engine, CLI and shared data/model tooling.
- `apps/`: desktop application and shared UI.
- `contracts/`: generated request/result schemas and types.
- `xtask/`: Rust contract generation, independent evaluation, benchmarking and research.
- `tools/`: desktop build/install scripts and frontend regressions.
- `tests/quality/`: versioned targets, actual judgments and native workload fixtures.
- `tests/reference/`: historical migration evidence, not ordinary development gates.

Rust is the authoritative implementation of TajsAnagrams. Active development and
CI do not require Python. Correctness is checked by native contracts, independent
letter/constraint verification and a simple exhaustive small-input enumerator;
quality and performance are measured against explicit targets and native revisions,
not historical Python score/order equivalence. Node and PowerShell remain useful
for the frontend and desktop build/install flow.

```text
npm --prefix contracts ci --ignore-scripts
cargo xtask check
cargo xtask contracts
cargo xtask benchmark --timeout-ms 10000
cargo xtask quality --runs .codex/temp/native-benchmark/runs.jsonl
cargo xtask research ngram --help
```

Retired Python code/tools/workflows were moved to the local-only ignored
`.codex/archive/python-retired-2026-10-13/`; Git history also preserves them.
They are historical source, not maintained or executed dependencies. Historical
JSON evidence remains tracked. Existing corpora stay in `.anagram_data/`, and
installed data/settings are unaffected. See `NATIVE_TOOLS.md` for current commands.
