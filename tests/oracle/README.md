# Active Python test oracle and compatibility tools

This is intentionally **not archive**: native parity checks and retained
benchmark/compatibility pipelines depend on these modules. The deployed Rust app
does not. All executable Python sources, their unit tests and original runners
were moved together from `archive/`, without deleting capabilities.

From this directory:

- `python -m unittest discover -s tests -v`
- `python anagram_solver.py --help` (reference/compatibility CLI, not production)
- `python anagram_generate.py --help`
- `python train_feature_ranker.py --help`
- `python build_wikimedia_phrase_index.py --help`
- Existing `ci_*.py` and `benchmark_user_runs.py` commands remain available here.

From repository root, `tools/reference_fixtures.py --check` verifies the frozen
source/data identity and captured behavior. Only the three data/registry-root
relocations are permitted by the source verifier. Historical manifests keep
their original paths and hashes; lookup resolves those paths to this directory
without depending on archive files. Corpus/cache data remains at root
`.anagram_data/`. Native/desktop preparation and execution are documented in
[`NATIVE_TOOLS.md`](../../NATIVE_TOOLS.md).

`python tools/run_oracle.py COMMAND [-- ARGUMENTS]` runs the retained pipelines
from repository root or any other current directory, with the correct oracle
working directory and UTF-8 output. Its `--help` lists supported commands;
not every historical runner implements its own `--help` flag.

The historical corpus manifest also captured mutable `normal_user_v2.json`
cache metadata. If reference integration refreshes it, validation still requires
all dictionary/count/WordNet bytes to match their frozen hashes, and independently
recomputes the exact short-word list, source identity and stamps for that one
derived policy file. It never rewrites historical hashes or exempts other inputs.

`tests/parity/compatibility.py` declares three reviewed native grammar corrections
and expected native presentation fields. It adapts comparisons **in memory**, not
the frozen source. Unknown fields, altered presentation, scores or order remain
failures. Native negative input and canonical SQLite validation are stricter than
this legacy engine; their regression expectations are explicit.

The retained Python cache overflow and cleft-heuristic limitations are historical
oracle behavior, not native acceptance claims. New research modules belong in
`tools/`, production algorithms in `crates/`, and test-only reference code here.
