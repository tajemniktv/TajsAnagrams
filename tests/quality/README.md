# Native TajsBench datasets and evaluation report v2

`cases.v1.json` is the curated/gold set, grouped by exact normalized letter bag.
Splits are bag-disjoint. The first cases deliberately start with the product
sanity phrase and existing project registry phrases. The registry is migration
evidence, not itself a human-quality label. New external phrases require a
source/version/license note in each case's `provenance`.
Its negative contrasts concern intended-target recovery; they are not actual
human judgments that those alternatives are useless.

`cases.v2.json` preserves those seven cases and adds twelve bag-disjoint authored
title/clause targets. New sources have sorted letters, so they cannot leak the
target word order. Their grades are curator relevance, **not actual user labels**
or an untouched held-out sample. Nineteen cases are still not a comprehensive
quality gate. Specify `--cases tests/quality/cases.v2.json` for the expanded set;
the default v1 keeps historical reports directly re-evaluable.

Run `cargo xtask check` to validate native correctness, datasets and metric
semantics. Evaluate recorded solver snapshots with:

```text
cargo xtask quality --runs .codex/temp/quality-runs.jsonl --output .codex/temp/quality-report.json
```

One JSONL record is `{case_id, variant, cache_state, trial, wall_ms, peak_rss_bytes,
result}` where `result` is the native `solve` JSON. An explicit `rows` array
can be used for scorer ablations or small independent experiments. Each row
needs `phrase` or `display_phrase`, or a `best_order` array. The evaluator rejects
non-anagrams. Native shallow rows with blank displays and exact `words` bags
receive zero relevance and retain their visible positions; their word order is
never invented. Optional `generated`, `expanded`, `deep_analyzed`,
`orders_evaluated` and `persistent_bytes` are reported, never estimated. Supply
separate records per cache state, trial and observed latency snapshot. A result
observed at 5 seconds is **not** retroactively counted at 50 ms.

The default `desktop` ranking preserves each native bucket's order and displays
ascending word-count buckets, exactly like `Results.svelte`. `--ranking
final_score` reproduces the former global score-sort as a separately labelled
diagnostic; `--ranking both` reports both. Explicit `rows` are already ordered
by the caller and are not re-sorted in either mode.

The report includes Recall@1/5/10, MRR, NDCG@5/10, cache-stratified p50/p95,
available state/work/size counters, word-count and puzzle-length breakdowns,
and quality at 50/200/1000/5000 ms. Metrics with no observations are null or
absent, not zero. Compare variants over the **same case IDs and budgets**; a
bare higher average over different coverage is not a win. Primary grouped
quality includes every recorded attempt, with terminal failures scoring zero;
`successful_metrics` explicitly reports the conditional successful-call average.
Completed-call latency and all-attempt latency have distinct field names.
`unattempted_cases` identifies dataset cases missing from each variant/cache
stratum; a missing record is not silently treated as an attempted solve.

Budget reports count `covered_cases` as unique puzzle IDs and `covered_attempts`
as delivered trials. Their flat quality fields describe actually observed
successful snapshots (null when none exist). `delivery_metrics` divides observed
quality by **all recorded attempts**: no delivery by the budget contributes zero
as an operational outcome, not as a fabricated intermediate ranking. A delivered
snapshot can coexist with a later failure of that attempt. Repeated snapshots
with the same `(variant, cache_state, case_id, trial)` are not independent trials.
Repeated timing trials do not add independent puzzles or human judgments.

`cargo xtask benchmark` defaults to the **desktop request
profile**, including prefix/10,000/no-repeats/min-Zipf-1.8/ten results per group,
and the desktop's 120-second default deadline. A regression test checks this
request against `Settings::default`. It still uses the CLI adapter and selected
CLI corpora/limits, not an installed desktop session. There is no legacy Python
profile or fallback; `--strategy`,
`--candidate-budget` and `--timeout-ms` are explicit overrides recorded in the
variant name and manifest. Optional `--phrase-db`/`--model` paths are explicitly
hashed and recorded; without them neither artifact is used.

The runner builds the native CLI if `--cli` is omitted; supply that flag for an
independently built revision. It creates a fresh disk cache per case/trial under
project-local `.codex/temp/xtask/benchmark` and removes only its own scratch cache,
then attempts disk-cache reuse. Reuse does not imply a successful cache hit;
the native `result.status.cache.hit` is the evidence for that. OS filesystem
caching is uncontrolled. `--repeats 3 --seed 13` randomizes case/trial order while
always running an empty cache before its reuse. Hash manifests freeze binary,
corpus, case and request identities. Process failures, invalid JSON and timeouts
are retained rather than aborting the remaining cases. Successful cold/warm
roundtrips must preserve result buckets and report a hit. Independent validation
checks exact residual letters, required multiplicity, hints, exclusions, repeats,
word counts, unique bags and chosen-order word identity without core validators.
RSS is sampled for the process tree; null means unobserved, not zero. The sampler
sleeps 20 ms between refreshes. Wall time includes capture/sampling overhead and
must not be directly compared with an older differently instrumented harness.

All seven initial source strings preserve the gold phrase's letter order.
`--shuffle-input --seed 13` scrambles only the input order, preserving exact
inventory, relevance and case identity; its variant and manifest identify the
control. Compare output buckets, not `normalized_input`, against the unshuffled
run. This checks source-order dependence but does not turn familiar targets into
a new held-out dataset or demonstrate naturalness.

`usefulness.v1.json` stores actual standalone-usefulness judgments separately
from exact target recovery and pairwise preference. Use `--usefulness
tests/quality/usefulness.v1.json` when evaluating them. The current two positive
labels came directly from the user; they are unblinded and express no relative
preference. `standalone_usefulness` reports useful/not-useful/unreviewed rows;
unreviewed phrases remain unknown, not fabricated negatives. Two phrase labels
on one bag are not a general naturalness benchmark. The frozen gold set and empty
pairwise preference dataset are deliberately unchanged.

Cache states are `disk_cache_empty`, `disk_cache_reuse`, `in_process_warm`,
`uncached`, or `unspecified`. Legacy `warm` booleans map to the first two states.
These are not interchangeable with cold/warm OS pages or desktop-process reuse.

## Stage-isolation experiments

```text
cargo build -p anagram-core --example ordering_probe --release
cargo xtask benchmark --stages --candidate-budgets 500,5000 --timeout-ms 10000 --deep-all --ordering-probe build/release/examples/ordering_probe.exe --output .codex/temp/quality-stages.jsonl
cargo xtask quality --stages .codex/temp/quality-stages.jsonl --output .codex/temp/regraded-stages.json
```

This research-only runner checks gold-word admission; randomizes prefix/diverse
and 500/5,000-candidate interventions; returns every ranked row to expose deep
selection; and projects the normal final display limit. `--deep-all` adds a
separate full-analysis intervention, with deadlines retained. No solve-result
cache is used. It also supplies gold bags through required words with exact
ordering: that oracle bypasses admission/search and is **not** production success.

The optional existing native `ordering_probe` supplies frozen retained-order
pools and feature contributions. One-at-a-time objective ablations remove the
feature's tie-break influence too. Six-word pools retain at most 512 of 720
evaluated orders; reports explicitly flag incomplete pools. This tests ordering
objectives, not full corpus-rescored quality or human naturalness. Raw requests,
responses, summaries and input hashes remain under `.codex/temp` by default.

## Paired generation performance

Preserve a known baseline CLI in a fresh project-local temporary path, then run:

```text
cargo xtask benchmark --mode generate --before .codex/temp/baseline-cli.exe --candidate-budget 500 --repeats 3 --output .codex/temp/generation-paired.jsonl
```

This pairs generation requests under the same desktop admission settings,
randomizes both case/trial order and before/after order within each pair, and
records full outputs and input hashes. Output comparisons include candidate
order, inventory/admission counters and exhaustion/truncation, not just bag sets.
Engine version labels are excluded from semantic equality. Equality is a
diagnostic, not a gate preventing intentional native improvements. A pair that does not finish
has unknown output equality and no completion-speedup estimate, rather than
vacuous success or a latency gain inferred from equal timeout ceilings.

`preferences.v1.json` holds human pairwise judgments separately from curated
relevance. Randomize the presented alternatives and record their actual left/right
positions and nonzero `presentation_seed`, then save the chosen side. The file is empty
until actual judgments are collected; fabricated human-preference accuracy is
not a benchmark result. Weakly supervised pairs belong in a separate dataset.

## Native regression and historical evidence

`workloads.v1.json` carries forward all five original CLI stress requests with
100,000 candidate caps. `cargo xtask benchmark --workloads
tests/quality/workloads.v1.json` verifies native result invariants and cache
roundtrips without historical scores/orders or fabricated quality labels.
It emits operational summaries rather than a gold-recovery report.

`native-baseline.v1.json` records an explicitly identified native revision and its
observed results; it is descriptive evidence, not a byte-for-byte score gate.
Changes should be judged using intended behavior, useful answers, actual judgments
and equal-workload performance evidence. Historical migration JSON remains under
`tests/reference`; the old Python implementation/tests/runners are local-only
archived source, not executed by current checks.
