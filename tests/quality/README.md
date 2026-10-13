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

## Next native baseline protocol

Status: **designed, not yet collected/frozen or accepted as a quality gate**.
This is the canonical Phase 5 specification; `PLAN.md` tracks completion and
`.codex/docs/AUDIT.md` records evidence. Keep the name **TajsBench** and extend
the existing Rust tools rather than creating a parallel benchmark project.

### Purpose and four separate tracks

1. **Correctness:** independent exact residual-letter/constraint checks, small
   exhaustive generation, lifecycle/cache/intake regressions. These are hard
   invariants, not human preference scores; keep them independent of the scorer.
2. **Useful answers:** does the displayed list contain sensible exact anagrams
   that people can use? Sentences, coherent titles and noun phrases all qualify.
   Both accepted sanity answers should be shown if found, not treated as mutually
   exclusive positives. Judge answers individually; two useful labels do not
   establish a relative preference or a pairwise tie.
3. **Reconstruction and diagnosis:** can known desired phrases survive admission,
   generation, shortlist, ordering and display? Target Recall/MRR/NDCG are useful
   here, but a useful alternative is not a failed usefulness result merely because
   it differs from an original phrase. Published solutions are non-exhaustive.
4. **Performance and resources:** how quickly/reliably does the normal profile
   deliver useful results, and what work/memory/storage does it require? Keep
   empty/reuse solver caches, uncontrolled OS pages, independent processes and
   installed-app observations separate. Do not combine these tracks into an
   arbitrary weighted score.

### Collection target and honest splits

Start with **120 independent letter bags**: 80 development, 20 validation and
20 locked pilot holdout. This is an initial collection budget, **not a claim of
statistical power** or a sufficient release threshold. All nineteen current
curator cases belong to the exposed development pool in the next dataset; leave
v1/v2 unchanged for historical reports. Their existing `test` values are not
proof of untouched holdout status. Collect at least sixty independently sourced
or actual user-contributed bags, and report them separately from authored cases.

Stratify collection/reporting by source origin, intended task and puzzle length:
short/name puzzles; ordinary clauses; titles/noun phrases; ambiguous bags with
several useful answers; and longer/morphological/constraint-bearing puzzles.
Cover the normal 1–6-word and up-to-40-letter product envelope, with deliberate
stress/Extended-mode cases in a separate suite. Do not infer a real user traffic
distribution from this deliberately balanced pool. Sparse cohort results are
descriptive, not independent statistically established wins.

Group before splitting by the English-v1 **residual inventory after subtraction**
and source family. Shuffling, spelling/case/punctuation changes, parameter
variants and related quoted-source cases are not new independent puzzles.
Keep family members in one split; count constraint variants as contexts on the
same bag, not more independent quality samples. Preserve request semantics so
a useful phrase violating a required/excluded-word constraint cannot qualify.
Record source URL/date/version or authored provenance, rights/quotation scope,
original puzzle context, actual constraints, answer origin and exposure history.
Do not claim multilingual coverage from English normalization.

Use published answers and project-authored phrases as **curator/reconstruction
targets**, not automatic human naturalness labels. An original poster's explicit
endorsement can be recorded with its exact context; publication alone is not a
standalone-usefulness judgment. Keep synthetic empty/impossible/error cases in
engineering tests, with emptiness only asserted where the chosen finite domain
is proven exhaustive. A deadline or candidate cap is not proof of no answers.

### Judging actual output lists

For each bag/context, deduplicate the union of actual baseline/candidate top-ten
outputs for judging, hide revision names and randomize presentation. Judge each
exact, constraint-valid phrase **useful / not useful / unsure**. Accept sensible
titles/noun phrases without requiring a full sentence. Record brief reasons for
negative/uncertain labels rather than automatically treating rarity, proper names,
humour or unfamiliarity as invalidity. Unreviewed/unsure items remain unknown.
No agent-generated preference should be labelled as actual human feedback.

Add pairwise preference only where helpful, allowing **left / right / tie /
neither / unsure**, with recorded left/right presentation and seed. The current
preference schema supports only left/right and must be extended before this
collection. Standalone usefulness and relative preference remain different facts.
Record judge origin, context, rubric version, disagreement and adjudication;
do not erase disagreement by silently changing an old label. Version any label
expansion and rescore **both** baseline and candidate against that same version.

### Primary useful-delivery metrics to implement

The current report v2 preserves target metrics and sparse useful-row counts.
It does **not** yet provide the following complete product acceptance report.
Extend/version the native evaluator rather than claiming existing counts already
establish useful delivery on the whole dataset:

- **Useful-success at 1/5/10:** fraction of attempted solves with at least one
  actual judged-useful answer in those visible positions. Failed/no-delivery
  attempts contribute operational zero. An unjudged delivered phrase is unknown,
  not an invented negative. Report lower bound (known useful successes / all
  attempts) and upper bound ((known successes + unresolved possible successes) /
  all attempts), plus the unresolved count and judgment coverage. Missing dataset
  runs are missing coverage, not quietly fabricated attempts.
- **Distinct useful answers at K and known-useful coverage:** showing both
  accepted sanity answers is desirable. Report the number of distinct judged-useful
  phrases delivered, unreviewed positions and coverage of the finite known useful
  set; do not call that set every possible useful answer. Exact duplicates consume
  visible slots but must not multiply useful counts. Distinct bag/order counts
  are descriptive diversity diagnostics, not automatic semantic-diversity scores.
- **Conditional judged-only metrics** may accompany bounds, explicitly labelled;
  never replace the operational denominator with whichever subset looks best.
- **Paired changes:** report wins/losses/ties and deltas on the same bag/context
  pool. Average timing repeats within each bag/cache stratum before estimating
  quality uncertainty; cluster confidence calculations by bag/source family.
  Do not treat three trials, two cache states or several snapshots as new puzzles.
  With sparse judgments, report uncertainty/bounds rather than declaring a winner.

Keep known-target Recall/MRR/NDCG as a separately named reconstruction panel.
Report source/task/length/constraint cohorts without silently changing weights
after seeing a result. Use actual desktop bucket/display order as primary;
global-score sorting is diagnostic. Preserve blank shallow-result positions.

At 50/200 ms, 1/5/10 seconds and the declared terminal deadline, count only
actually observed deliveries. Today's terminal CLI capture cannot establish
first-useful-result time, cancellation-to-terminal lag in the installed UI, or
unobserved intermediate rankings. Add native job/snapshot instrumentation before
claiming those measurements; separate bounded endpoint experiments from anytime
traces. Include failure coverage, all-attempt and completed-call latency, work
counters, sampled RSS (not guaranteed peak), and observed persistence sizes.

### Freeze one reproducible native baseline

Preserve `native-baseline.v1.json` as pilot evidence, not the new promotion
baseline: it was captured from a dirty working tree, one trial per cache state,
and retains only top-three-per-bucket projections. Those projections cannot
stand in for full displayed top-ten lists or complete reranking pools.

For the replacement baseline, freeze a **clean signed native revision**, full
executable/measurement-tool hashes, compiler/OS/hardware and exact corpus,
request/profile, dataset and judgment-set identities. Retain full raw JSONL
responses and failures under a versioned project-local artifact directory;
check in the public-safe manifest/labels/report and hash the retained captures.
Record binary-to-source build correspondence instead of assuming a hash proves it.
Do not publish private user puzzle context or unapproved corpus/model artifacts.

The primary profile mirrors native desktop defaults; separate diagnostic caps,
required-bag controls and Extended/stress profiles. Start with three paired
timing trials per bag/cache state, randomized native revision order on the same
machine and identical inputs. Inspect variability and add trials where useful.
Solver cache empty/reuse does not imply cold/warm OS pages. Keep instrumentation
overhead consistent and never equate a terminal timeout with completed latency.

Use the existing stage runner to distinguish admission, generation, shortlist,
chosen order and final display. Hold candidate pools fixed for ranking ablations
and order pools fixed for objective ablations; separately measure end-to-end
effects at equal time/work budgets. Required-word interventions deliberately
bypass admission/search. Document incomplete pools and unobserved absences.

The locked holdout must remain untuned, with actual exposure tracked. Collecting
or assigning a `test` tag alone does not make it untouched. If developers inspect
and adapt to it, retire/replenish that holdout. Twenty pilot bags have coarse
resolution and may need expansion; choose a minimum practically useful effect
and resource/latency tradeoffs **before** final evaluation, using pilot variability.
Correctness fixes can land for their own explicit reason; quality promotion
requires comparable, adequately judged evidence and explained cohort tradeoffs.
Neither this design nor a larger unjudged fixture file passes that gate.

### Sourcing brief for Echo or another researcher

Find an initial **40–60 candidate English letter bags from actual public puzzle
requests/use cases**, not just another list of generated anagrams or famous
single-word pairs. Seek a mix of titles/phrases, names, clauses, ambiguous
multi-answer requests, longer puzzles and explicit constraints. For each, return
the minimal puzzle text, source link/date, original context/constraints, published
answers, whether the asker explicitly accepted them, and provenance/quotation
notes. Verify exact letters independently; flag discrepancies rather than fixing
them invisibly. Avoid copying unrelated personal information or whole posts.

Deduplicate by residual bag/source family, flag overlap with existing fixtures
and record who has seen the case/answers. A published answer is not uniquely
correct, and a researcher's generated alternative is not a human label. Return
candidate source packets; do not promote them to judged positives/negatives or
declare them locked holdouts automatically. This is a ready research brief, not
an already dispatched task or completed online collection.
