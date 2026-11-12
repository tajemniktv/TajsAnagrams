# TajsAnagrams

Desktop anagram solver and Wordle assistant powered by the native Rust engine,
with a shared Svelte UI.
The installed application does not require Python or a separate CLI executable.

The Wordle tab offers Play Along with editable feedback, an Instant Solver with
position/count constraints, and a local Practice game with hints on demand.
Choose 4–8 English letters and 1–12 attempts. Recommendations distinguish
strategic probes from possible answers and identify heuristic versus completed
profile-optimal search. Broad/Common English profiles reuse configured local
data; custom answer and allowed-guess lists are supported. These profiles do not
claim to reproduce the official Wordle vocabulary. Play Along and Instant Solver
resume on this PC, with vocabulary identity checked before analysis. Undo is
available during the current run. Practice games are session-only. See [Wordle validation](tests/wordle/README.md) for tests and
reproducible policy/latency benchmarks.

- [Desktop app and local builds](apps/desktop/README.md)
- [Native CLI and data/model tools](NATIVE_TOOLS.md)
- [Implementation plan and progress](PLAN.md)
- [Native ownership and contracts](docs/ARCHITECTURE.md)
- [Native quality evaluation and experiment datasets](tests/quality/README.md)

## Repository layout

- `crates/`: native engine, CLI and shared data/model tooling.
- `apps/`: desktop application and shared UI.
- `contracts/`: generated request/result schemas and types.
- `xtask/`: Rust contract generation, independent evaluation, benchmarking and research.
- `tools/`: desktop build/install scripts and frontend regressions.
- `tests/quality/`: versioned targets, actual judgments and native workload fixtures.
- `tests/reference/`: recovery index for locally archived migration evidence, not a gate.

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
cargo xtask review
```

Retired Python code/tools/workflows were moved to the local-only ignored
`.codex/archive/python-retired-2026-10-13/`; Git history also preserves them.
They are historical source, not maintained or executed dependencies. Historical
reference payloads are local-only in `.codex/archive/reference-evidence-2026-10-14/`
and recoverable from Git commit `69c50d9`; current native quality evidence remains
in `tests/quality/`. Existing corpora stay in `.anagram_data/`, and
installed data/settings are unaffected. See `NATIVE_TOOLS.md` for current commands.
