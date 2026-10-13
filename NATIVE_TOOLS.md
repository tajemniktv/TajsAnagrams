# Native tools and remaining compatibility boundaries

Build `cargo build -p anagram-cli --release` to obtain `build/release/anagram-cli.exe`.

On Windows, `pwsh -File tools/build-release.ps1` builds in the persistent
`build/cli` cache, smoke-checks the candidate, then publishes to
`build/release/anagram-cli.exe` with one rotating backup in `build/backup`.
It does not delete other build artifacts. Local dev/test and release builds
use incremental codegen; unchanged builds reuse Cargo's fingerprints and artifacts.
Use `cargo clean --target-dir build/cli` only when deliberately
reclaiming space or diagnosing a build-cache problem.
`.cargo/config.toml` also directs ordinary Cargo builds to root `build` rather
than the old `target` folder. Only ephemeral staging/recovery files use `.codex/temp`.
These are optional maintenance/experiment tools; the desktop solver never launches
the CLI and does not need Python. Tool output is JSON. `--timeout-ms N` also applies
to these commands. Existing model/dictionary/database outputs are never overwritten:
choose a new path and select it in desktop Settings after successful preparation.

## Learned order ranking

1. Prepare a JSON array of cases, for example:
   `[ {"answer":"we are home","acceptable_orders":["we are home"]}, ... ]`.
   Use multiple distinct bags. All orders of a bag stay in the same validation fold.
2. `anagram-cli ranker-build WORDNET_DICT [PHRASE_DB]` reads that array from stdin
   and writes `{groups, skipped}`. It enumerates exact orders for 2–6 words and
   retains up to 56. Missing acceptable orders are reported as skipped, never
   inserted into the pool as an artificial positive. Combine multiple acceptable
   orders into one case rather than repeating the same bag.
3. `anagram-cli ranker-train NEW_MODEL.json` reads that output from stdin. Optional
   top-level `options` contains `folds`, `epochs`, `learning_rate`, `l2` (defaults
   5, 80, 0.08, 0.002). It writes the model file and reports baseline and held-out
   recall@1/MRR. At least two occupied hash folds are required. The final saved
   all-group fit is **not** used to calculate held-out metrics.
4. Select the model in desktop Settings → Experimental, or pass it to the native solver:
   `anagram-cli solve DICTIONARY UNIGRAMS BIGRAMS WORDNET - MODEL.json`.
   Replace `-` with a phrase database if desired. Solve requests accept optional
   `workers` (0 for automatic, 1–32 explicit, default 1) and `refine` (default false).
5. For offline experiments, `anagram-cli ranker-rank MODEL.json` reads an array
   of `{key, features, baseline_score}` from stdin and returns ranked indices and
   loaded-model identity. The 18-feature schema is in `learned.rs`.

Alternatively, save the `ranker-build` output as JSON and use Settings → Experimental
to train locally. The desktop limits input to 4 MiB and 1,000 groups, with one worker
and a 60-second deadline. Epochs (1–200), folds (2–10), learning rate and L2 are
explicit controls. The resulting quality report records those options.
Cancel and app shutdown stop the worker. Successful models get unique files in
`user-data/models` beside the installed app; they are not automatically enabled.
Select the model and save settings to apply it. No source downloads or dataset
preparation run automatically. The Experimental tab can also build datasets from
the labelled case JSON described in step 1 and reopen saved `.report.json` quality
reports. Authoring the correct answers remains a user-owned step.
The native builder additionally accepts `{ "cases": [...], "options": {
"retained_orders": 56, "phrase_bonus_max": 10.0 } }`; the desktop exposes both
options. Plain case arrays retain the historical defaults. Standalone `ranker-rank`
is available from Experimental without training or model activation.

Settings → Data exposes native downloads, offline preparation, non-destructive
updates and validation. Settings → Diagnostics exposes last-solve provenance,
timings and cache information. The desktop links the same maintenance library as
the CLI; it does not invoke a CLI executable or Python process.

Desktop learned selection operates on the retained-order corpus shortlist. It does
not imply a quality improvement; evaluate a model on representative held-out cases.
Standard final scores are retained so their existing interpretation does not change.

## Corpus preparation and updates

Phrase building automatically uses up to eight workers for normalization and
phrase extraction. Bounded batches combine duplicate phrases before sorted SQLite
writes; a 64-MiB page cache and `WITHOUT ROWID` table reduce storage and random I/O.
Decompression and database writes remain serial. Output includes `workers` and
`database_updates`; phrase/count semantics and no-overwrite publication are unchanged.

- `anagram-cli prepare-dictionary BASE UNIGRAMS NEW_DICTIONARY` normalizes and
  deduplicates the base vocabulary, adds the established contraction spellings,
  and reports corpus-common two-letter words (Zipf >= 5) for the extra-short list.
- `anagram-cli build-phrases NEW_DB TITLE_FILE[.gz] ...` ingests one or more local
  Wikimedia-style title dumps. It accepts 2–8-word titles, stores whole titles
  and 2–5-word subphrases, and aggregates duplicate evidence into unique SQLite
  text keys. Inputs may be plain text or gzip. Outputs are published only after
  successful completion; cancelled/failed builds never publish a partial database.
- `pwsh -File tools/prepare-native-data.ps1 -NativeCli PATH_TO_CLI -OutputDirectory NEW_DIRECTORY`
  explicitly downloads the dictionary, Norvig counts and WordNet 3.1, prepares the
  dictionary, and records file hashes. Add `-IncludeWiktionary` and/or
  `-IncludeWikipedia` to download title dumps and build a combined phrase database.
  It never changes the active desktop settings or overwrites an existing corpus
  directory. Source downloads stay in project `.codex/temp` for inspection/retry.
  `-SourceDirectory DIR` uses already-downloaded `base.txt`, `count_1w.txt`,
  `count_2w.txt`, `wordnet.tar.gz`, and optional `enwiktionary.gz`/`enwiki.gz`
  instead of network access. Dataset licensing still governs redistribution.

Use a new versioned folder beneath the app's `corpora` directory for an update,
then choose its files in Settings. The previous corpus is preserved for rollback.
Automatic background updates/downloads are deliberately not enabled.

Corpus intake is bounded by default to 2 GiB per role and 3 GiB aggregate.
`--limits POLICY.json` applies to generation, solving, and native maintenance
commands. `max_corpus_role_bytes` and `max_total_corpus_bytes` accept positive
integers; an omitted field uses its finite default and explicit `null` disables
that limit for trusted local tooling. Limits are enforced on actual reads, so
file growth cannot bypass them. Phrase builds count decompressed title bytes
with a distinct role per source. WordNet files share the other solve inputs'
aggregate budget. Models retain their additional 64 KiB format cap.

Phrase-index admission counts the canonical schema/row JSON stream, including
escaping overhead, in an unsorted pass before aggregation or sorting. Identity
hashing re-reads that admitted SQLite snapshot without charging its input twice.
Ranker dataset building checks duplicate keys without calculating a discarded
identity hash. SQLite pages, WAL, temporary sorting storage and
parser heap allocations are additional. These limits bound input, not process
memory. Desktop validation performs separately bounded sanity and solve phases;
native desktop preparation also bounds archive decompression. The PowerShell
`tools/prepare-native-data.ps1` archive extraction does not have this decompressed
byte limit; use it only with trusted archives and sufficient disk space.
Repeated parser passes over a
captured snapshot do not charge its source bytes again. Default sizes preserve
the preparation tools' existing 2 GiB source ceiling; select smaller values when
the available memory requires them. Compatibility with private corpora was not
measured.

## Deliberately retained differences

### Native input and output conveniences

Generation/solve JSON accepts optional `generation.subtract` (or `subtract` on
the generation endpoint), an array of letter chunks. They are removed without
inserting output words. Unavailable letters, empty chunks and a fully empty
residual target are errors. Required words still consume the remaining inventory.
The desktop Advanced panel exposes the same option. Normalized result input is
the residual target, including on cache hits; subtraction participates in cache identity.

CLI generation/solve accepts repeated `--subtract TEXT` and `--exclude-file FILE`.
Exclusion files are whitespace-separated words, bounded to 1 MiB aggregate,
appended to JSON exclusions rather than replacing them. Corpus paths remain explicit.

`generate --stream-export NEW.jsonl` emits a header with `complete:false`,
one exact accepted bag per line during enumeration, then a terminal count/stop
record on successful completion. The extra cap probe is never exported. An
interrupted or failed stream can retain a partial transcript without a successful
footer; it must not be treated as a completed generation. Stream export currently
applies to generation, not ranked solve, and does not make generation constant-memory.

`--export NEW.json` writes the complete **returned** native JSON result.
`solve --deep-export NEW.json` filters returned rows to deep rows, retaining
original solve counters and an explicit export scope. These are native JSON,
not Python's legacy pre/deep text formats or an implicit widening of result limits.
All output paths must be new: existing files are never overwritten. A filesystem
write failure may leave a partial new file; no successful result is claimed.

`solve --hint-stats` reports hint occurrence among returned rows;
`solve --benchmark-answer PHRASE` reports exact displayed-answer ranks among
returned rows only, after scoring. Neither report changes search or ranking.
An absent answer is not proof that it was absent from unreturned candidates.
On generation these flags report hint counts and exact **word-bag** matches,
explicitly scoped to `returned_bags`, not a chosen sentence order.
The obsolete Python text/report workflows are retired historical source, not
maintained compatibility requirements. Native exports deliberately retain their
documented returned-result scope rather than reproducing those formats.

### Current benchmark/data workflows

The current phrase/performance CI jobs build and execute the native CLI. Data
preparation reuses `tools/prepare-native-data.ps1`; phrase indexing, grouped
dataset construction, training and solve are Rust operations. `cargo xtask
benchmark` is the native child-process measurement harness; `cargo xtask quality`
is its independent evaluator. Optional `--phrase-db` and `--model` are hashed
and distinguish variants. The evaluator preserves
failures and cache strata rather than equating exact-target recovery with usefulness.
Model training is informational: a completed fit is not a quality promotion.
Current phrase snapshots default to Wiktionary, with expensive combined Wikipedia
preparation opt-in on manual workflow runs; monthly cache refreshes are not
benchmark evidence. Corpora/models are not published in the report artifacts.

### Rust development interface

`cargo xtask check` runs formatting, core/CLI/tool tests, strict Clippy, generated
contract drift checks and independent AJV/TypeScript fixtures. Install contract
dependencies first with `npm --prefix contracts ci --ignore-scripts`.
`cargo xtask contracts [--check]` derives JSON schemas and TypeScript directly
from Rust, without a Python intermediary.

`cargo xtask benchmark --before BASELINE --mode generate` compares independently
built native revisions in randomized pairs. Equality is diagnostic, not a
historical score/order gate. `--stages` replaces the active stage-isolation
experiment; `--deep-all` and `--ordering-probe PATH` expose deeper interventions.
`--workloads tests/quality/workloads.v1.json` retains five explicit native
stress requests at their original 100,000 candidate caps, independently verifies
letters/constraints and successful cache roundtrips, and reports no quality labels.
Benchmark files/manifests record failures, trial/cache strata and exact hashes.
RSS is sampled process-tree resident memory, not a guaranteed peak. The sampler
sleeps 20 ms between refreshes; wall time includes capture/sampling overhead.
The subprocess guard defaults to the engine timeout plus 30 seconds.

`cargo xtask research lexicon --db ESDB.sqlite` reads a supplied ESDB database
read-only; database creation is upstream input preparation, not a required Python
step. `research ngram --arpa FILE --groups FILE` scores retained order pools with
ARPA backoff. `research tubelex --tubelex FILE.tsv.xz` reports frequency/diversity
features; plain TSV is also accepted. These are offline reports, not corpus/model
promotion. Use each command's `--help` for paths and budgets.
Experimental dependencies live only in `xtask`, not the installed application.

The Python solver, parity tests, obsolete runners and workflow are retired into
local-only `.codex/archive/python-retired-2026-10-13/`, with a content-hash manifest.
Tracked JSON/reference records remain historical evidence. Rust is authoritative;
no active check imports or executes the retired source. Future disposable Python
research is possible if justified, but is not a required project runtime.

- Rust regex excludes backreferences and look-around; invalid patterns are errors.
- Phrase databases must have a real `ngrams` table with unique text keys.
- Python pickle caches and historical command-line flag syntax are not imported.
- Extended desktop mode still respects engine correctness limits (10-word ordering),
  but no longer caps result counts. Pagination is not truncation. Standard bounded
  mode remains the safe default; cache and execution limits can be customized.
- The website, multi-platform packaging and full combined-corpus benchmark matrix
  remain deferred. Native features need not reproduce every historical
  experimental script's command-line interface.
