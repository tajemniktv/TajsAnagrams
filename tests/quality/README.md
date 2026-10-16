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

`cases.v3.json` is the previous **development** reconstruction suite (43 bags): all
19 v2 cases unchanged, 19 additional synthetic/ambiguity controls and five already
exposed public reference bags needed for the accepted model reviews. It retains
Voldemort and the product sanity phrase and adds Hips Don't Lie, Testing Shit,
ordinary clauses and noun phrases. The two single-word references Listen/Silent
explicitly exercise ambiguity. New synthetic inputs normally use sorted letters;
TestingShit retains the user's input. Authored targets are legitimate reconstruction
checks, not observations of actual user satisfaction. Cohorts preserve origin,
category, template/source family and development exposure. Old inherited `test`
tags do not magically create a holdout. V1/v2 and their historical captures remain.

`cases.v4.json` is the current 50-bag development suite: v3's 43 records are
unchanged, with seven previously exposed public bags added to cover the complete
desktop review. Public references remain reconstruction targets, not unique valid
answers; their source-theme constraints are not silently imposed on unconstrained
native requests. Older datasets and captures remain frozen.

`human-judgments.v1.json` preserves the owner's 532 exported standalone verdicts
(59 useful, 237 not useful, 236 unsure), rendering, exact inventory and context
provenance. All 131 earlier votes are unchanged. Three private notes and the one
real pairwise preference remain in the exact local export, not fabricated into
new public labels. This is 17 observed case contexts (16 older experiment bags
and a desktop bag), not human coverage of all 50 cases. Compare it separately:

```powershell
cargo xtask quality --runs CAPTURE.jsonl --cases tests/quality/cases.v4.json --human-judgments tests/quality/human-judgments.v1.json --output REPORT.json
```

Standalone matching checks exact inventory plus case/whitespace-folded wording;
apostrophes and punctuation remain meaningful. Opaque desktop IDs are not puzzle
identities. Conflicting context votes abstain, as do Maybes; neither task fit nor
a model majority replaces the owner. Per-word-count top-ten panels are separate
from flattened prefixes. Registered `cargo xtask review` views can also use
`--human-judgments FILE`; tests/builds refresh the owner and model panels without
filling or overwriting the annotation CSV.

### Phase 7 measured research, not a promoted ranker

Current development interfaces also support broader-source and ordered-search
experiments (none changes normal Settings or the installed strategy):

```powershell
cargo xtask research corpus --text DETAILED_TATOEBA.tsv --train-output NEW.txt --heldout-output NEW.txt --output MANIFEST.json
cargo xtask research order-evidence --groups ORDERS.json --text SENTENCES.txt --output EVIDENCE.json
cargo xtask research ordered-search --before build/release/anagram-cli.exe --dictionary DICTIONARY.txt --arpa MODEL.arpa --cases tests/quality/cases.v4.json --beam-width 64 --work-cap 200000 --timeout-ms 10000 --output NEW.jsonl
```

The ordered-search JSONL can be regraded by `quality` using the same cases and
owner fixture. Treat result transport, nonempty delivery, capped/pruned search
and true exhaustion as different outcomes. Native and experimental score scales
differ; compare actual displayed ranks, exact references and separately judged
usefulness, not score magnitudes. Research keeps one winning order per word bag
and ten distinct bags per word-count bucket, matching the default display scope.
Raw model/corpus artifacts and local captures are not mandatory CI dependencies.

`phase7-experiment.v1.json` freezes the owner validation, real 5-gram experiment
and randomized native shortlist comparison. The model is **not bundled**:

```powershell
cargo xtask research train-ngram --text SENTENCES.txt --arpa-output NEW.arpa --output TRAINING.json
cargo xtask research ngram --arpa NEW.arpa --groups ORDER_GROUPS.json --output SCORES.json
```

Training is Rust-native interpolated modified Kneser-Ney, order five; lower
orders use continuation counts, singletons become UNK, per-count discounts are
estimated (recorded 0.75 fallback for degenerate count-of-counts), and there is
no pruning or benchmark-answer input. The bounded input is one already-tokenized
sentence per line. Outputs are new files, not overwritten models. ARPA scoring
maps OOVs **before** context lookup, including history; otherwise observed UNK
transitions are silently lost. OOV counts, BOS/EOS, worst conditional, total,
mean, direct-hit/backoff and same-bag margin diagnostics remain explicit.

Four Standard Ebooks texts supplied 281,671 training tokens / 16,902 sentences;
the plain artifact is 35,957,972 bytes. Every tenth sentence was reserved for a
same-book natural/reversed sanity check: 1,852 wins and three ties among 1,879.
That is not an author/genre holdout or an anagram-quality result. Original source
licenses, commits, hashes and extraction manifest remain local; see
`DATA_RESEARCH.md` for the source/packaging boundary.

One offline scoring batch (23,706 orders, including two ARPA scans and JSON
output) takes 2,696 ms with a sampled process RSS peak of 100,442,112 bytes.
This is one research-process observation with uncontrolled OS/background state,
not per-solve runtime latency or a guaranteed peak. Production loading, caching
and integration are not implemented or implied by that measurement.

The frozen **actual** displayed/owner-reviewed bags yield 1,560 retained-order
pools / 23,706 orders. Unjudged permutations are not new human observations.
The probe emits native `display_phrase`, so contraction letter keys such as
`dont` are scored/matched as displayed `don't`, not falsely treated as an OOV.
An exploratory grammar + 0.10 mean-log-probability blend raises the mean within-
puzzle Yes/No concordance from 0.486 to 0.560 on 15 puzzles, but selects only
56 known-positive orders versus 58 for native ordering. Restricting the blend to
fully in-vocabulary bags retains 58 but concordance falls to 0.486: the apparent
gain is entangled with OOV/domain coverage. No representative/statistically
significant quality improvement is established; these order features omit the
production row's lexical priors, phrase corpus and corpus-rescoring stage.

The distinct-family shortlist proposal was also **rejected**, not shipped:
50 randomized pairs with the same lexical60 dictionary and 10-second deadline
give 13/50 intended top-ten hits in both treatments (47 completed each). Known
useful-answer delivery falls 14 to 13 puzzles; the permissively approved
`no chile beachs` is displaced in books-02. Deep work rises 9,780 to 11,005 rows;
attempt medians are 1,261 versus 1,300 ms, one timing observation per case and
background/OS effects uncontrolled. Maybes/new outputs remain unknown. The
experimental patch and binary hashes are frozen with the evidence. Production
retains row-seeded family expansion; no intended-answer exception was added.

Phase 7's production gate stays open: source/domain coverage, OOV-stratified
comparison, phrase/cohesion ablation and end-to-end footprint/latency need evidence
before managed model packaging or Phase 8 language-guided search is promoted.

For this suite, recovering **any intended reference within the first ten of its
displayed word-count group** is a reconstruction success; first place is not
required. The app has separate groups, so `intended_answer_recovery` records each
reference's bucket rank and flattened display rank separately. It preserves
punctuation and failed-attempt denominators. Traditional target Recall/MRR/NDCG
remain available as diagnostics, not the sole product-success definition. Another
coherent exact result can be useful whether or not the named target appears.

The owner explicitly accepted the two returned model reviews for development
evaluation on 2026-10-15. `model-judgments.v1.json` freezes both 131-row verdict
sets, explanations/lookup flags, source hashes and reported provenance, without
publishing private library identifiers or human notes. Use:

```text
cargo xtask quality --runs FILE --cases tests/quality/cases.v3.json --usefulness tests/quality/usefulness.v3.json --model-judgments tests/quality/model-judgments.v1.json --output FILE
```

`model_standalone_usefulness` reports each reviewer separately, both at a flattened
prefix and within each word-count bucket. Maybe remains unreviewed for binary
success/noise bounds; it is also counted explicitly as uncertain in each source's
judgment totals. A lookup flag does not silently overrule an accepted verdict.
Human fixtures are unchanged. Conflicting human/model and model/model verdicts
stay separate rather than majority-voted into fabricated consensus truth. Acceptance
lets development move forward without another mandatory adjudication round; it
does not authenticate identities, erase Sol's disclosed label exposure or establish
representative population quality. Freeze a new judgment-set version when labels
change and rescore every compared treatment against the same version.

`lexicon-experiment.v2.json` records the first 43-case development measurement and
a selected twelve-case, three-trial confirmation, with input/executable/capture
hashes, strict target ranks, separate accepted-model coverage and stage evidence.
It is a measured baseline, not a claim that the current results are outstanding
or a generalization gate. The historical v1 experiment remains unchanged.

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
preference. Report **v3** retains every displayed position (including duplicates)
for target metrics and gives repeated target phrases no additional relevance
credit. Historical v2 reports remain dated evidence; do not compare their scores
with v3 without rescoring both raw captures. `standalone_usefulness` covers all
recorded attempts per variant/cache/ranking, not only already-judged bags. It
reports useful/not-useful/unreviewed positions, operational useful-success bounds
at 1/5/10, per-attempt distinct useful answers, and coverage of the finite known
useful set. Duplicates occupy slots but do not multiply distinct useful counts.
Unknown delivered phrases remain unresolved possible successes; failed and empty
terminal deliveries score operational zero. Blank shallow bags are not phrases
to judge. Coverage with no relevant denominator is null, not a perfect score.

Usefulness and preference matching folds case/whitespace only: punctuation and
accents remain part of the rendered phrase. Case ID fixes the context; do not
reuse a case ID for a different task/rubric. Legacy target matching still strips
punctuation as a separately identified reconstruction diagnostic. This is an
explicit matching policy, not automatic permission to transfer a judgment across
different punctuation, capitalization-sensitive tasks or contexts. Two phrase labels
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
and equal-workload performance evidence. Historical migration JSON is recoverable
through the `tests/reference` archive index; the old Python implementation/tests/runners are local-only
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

### Primary useful-delivery metrics and remaining work

Report v3 implements terminal attempt-denominated useful-success bounds,
judgment coverage and distinct/known-useful counts below. Paired bag/family-clustered
uncertainty, cohort reporting and actual anytime usefulness instrumentation remain
pending. Two labels do not establish a complete product acceptance report:

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
declare them locked holdouts automatically. The first returned packet was
received on 2026-10-14; its intake is described below. This is not a claim that
the whole collection target or blinded human evaluation is complete.

### Public puzzle intake and development dry run (2026-10-14)

Echo's review and ZIP are preserved byte-for-byte under project-local
`.codex/temp/echo-research-2026-10-14`. Attached programs were inspected but not
executed; they are evidence, not repository instructions or new Python tooling.
`cargo xtask research puzzles` independently verifies the manifest, source/family
references, candidate residual uniqueness, ASCII inventories, explicit removals,
word/character counts, ordered lengths, initials, token shapes, equivalent inputs
and proposed corrections. Unknown structural rules fail closed. The packet's
ASCII contract is deliberately distinct from production NFKD normalization.

```text
cargo xtask research puzzles --packet .codex/temp/echo-research-2026-10-14/public-puzzles-2026-10-13 --output .codex/temp/echo-research-2026-10-14/native-intake.json
cargo xtask research puzzles --packet DIR --case-ids ID,ID --workloads-output .codex/temp/puzzles.workloads.json --output .codex/temp/puzzles.intake.json
cargo xtask benchmark --cli build/release/anagram-cli.exe --workloads .codex/temp/puzzles.workloads.json --timeout-ms 10000 --guard-ms 15000 --output .codex/temp/puzzles.runs.jsonl
cargo xtask research puzzles --packet DIR --runs .codex/temp/puzzles.runs.jsonl --output .codex/temp/puzzles.delivery.json
```

The pool has **56 unique candidate bags, 25 source families, 30 source posts and
nine quarantine records**. All 65 independently recomputed checks agree with the
attachment, including nine letter-mismatched proposals and two exact proposals
failing a character-count rule. All 56 candidates have at least one structurally
valid published answer; that proves neither semantic correctness nor usefulness.
There is no overlap with the nineteen current v2 bags checked by default. Forty-eight
inputs are source-defined (46 text, two count vectors); eight are paired with or
reconstructed from answers. Report these origins separately. Thirteen book inputs
share one source family. This is a convenience pool of mostly thematic puzzles,
not a real traffic sample, 56 independent families, or a held-out gold set.

Retain the authentic input, source context/constraints, published proposals and
quarantine reasons alongside the exact executable request. Explicit removal
inventories become a documented residual-letter projection; source-specified word
counts can map to native bounds, with no desired answer in hints/required words.
Ordered word lengths may supply a word-count bound, but their positional constraint
remains unsupported. Semantic themes, punctuation shapes, character preservation,
initials and other unsupported rules are visible in the report, not silently
claimed enforced. Over-40-letter or over-six-word inputs stay in Extended/stress,
not the normal dry run. Quarantine never becomes eligible just because one
alternative is exact. Identity/mere-reordering eligibility needs task review;
neither should receive automatic usefulness credit.

The 12-family dry run retained full desktop-order responses, failures and hashes
locally. Eleven families have source-defined inputs; `gaps-thirteen` is an explicit
answer-derived calibration case. One empty/reuse cache trial per bag at 10 seconds
is a feasibility experiment, not a performance comparison or baseline freeze.
Delivery diagnostics enforce the documented request, independently check output
invariants and test original structural rules against actual rendering. Published
order recovery remains separate from literal constraint satisfaction and human
usefulness. All inputs/answers are development-exposed to Echo and Codex; no new
human labels or holdout are created, and source licensing/quotation notes remain
with the local packet rather than republishing whole posts.

Next judge a small blinded actual-output pool with task context, adjudicate
identity/rendering policy, expand source-family diversity and compare native corpus
admission policies before freezing a baseline. Do not patch in the published
names just to make this exposed sample score better.

### Broad lexical policies and human-readable review

`cargo xtask research lexicon --db DB --variant-dir .codex/temp/LEXICONS` exports
the original three answer-independent interventions: the unchanged normalized baseline;
baseline union ESDB US/A size <=80, variants <=1, tagged names and contractions;
and that union plus literal alphabetic/apostrophe unigram tokens at Zipf >=3.
The last is deliberately a frequency-assisted noise challenge, not a validated
English dictionary. No puzzle/answer file enters the export function. ESDB tags
are incomplete; compounds/abbreviations and uncertain name classes are excluded.
All treatments retain baseline keys and inherit the same runtime admission,
candidate, ordering, corpus, output and timeout policies. None is auto-installed.

The follow-up `lexical60` treatment unions names80 with **all** literal ASCII
alphabetic/apostrophe ESDB US/A size <=60, variants <=1, non-abbreviation entries,
without restricting POS/class. This tests whether a common-word source closes
admission gaps without the unsupported raw-frequency-only expansion. It neither
reads benchmark targets nor special-cases profanity/fictional names. ESDB tier
and policy remain recorded; per-lexeme tier-aware ranking is still future work.

`cargo xtask benchmark --workloads FILE --dictionaries VARIANTS.json --repeats 3`
randomizes native vocabulary treatments within each case/trial, pins file hashes
and gives every treatment its own empty/reuse solver cache. Compare all attempted
calls and per-bag medians, retaining failures; three timing repeats do not create
three independent human quality samples. Record competing build/process activity.
All outputs remain exact-letter checked, but semantic noise must be judged rather
than inferred from the proportion of newly admitted tokens or model scores.

`cargo xtask review` refreshes **`.codex/docs/TAJSBENCH.md`** without running a new
solve. Native tests, `cargo xtask check` and normal desktop/CLI build scripts call
the same renderer. It shows stored native pilot phrases/failures and, when a local
experiment is registered, a variant-hidden, randomized, deduplicated union of actual
top-ten outputs with original task context, source constraints and explicit native
projection limits. The historical top-three-per-bucket pilot loses original full-list
positions: its retained-position numbers are not genuine full-list ranks or @K proof.

Register a completed capture with:

```text
cargo xtask review --runs FILE --workloads FILE --dictionaries VARIANTS.json --judgments .codex/temp/judgments.csv --summary .codex/temp/report.json
```

Add `--model-judgments tests/quality/model-judgments.v1.json` for a compatible
development pool to retain accepted model diagnostics in the registered review.
Normal checks/builds then refresh the intended-reference rank table and separate
model coverage panels automatically, inside the unblinded diagnostics section.
They never fill or overwrite human CSV cells. Request comparison uses the strict
native request schema rather than JSON numeric spelling (`6` versus `6.0`);
unknown fields, different constraints and genuinely different options still fail.

Do not edit the generated Markdown. Edit the separate CSV (`standalone` and optional
`source_task`: useful/not_useful/unsure). Blank/unsure stay unknown; context hashes
prevent reuse across changed tasks, and existing human cells/notes survive refresh.
Inspect the comparison panel only after judging if variant-hidden presentation matters.
The pilot has its own `.codex/temp/tajsbench-pilot-judgments.csv`; partial historical
judgments cannot reconstruct full-list coverage. Missing local research artifacts
produce an explicitly marked pilot-only automatic view, not a production build
dependency or fake comparison. Explicit `--runs` still fails on invalid evidence.

`usefulness.v2.json` versions four actual standalone positives across three bags,
plus four qualitative/uncertain responses which remain nonbinary. It contains public
case IDs beyond v1/v2 curator datasets, so use the registered workload review for
that combined pool rather than feeding it to an incompatible old `--cases` file.
Rescore baseline and candidate with the same label version. Human verdicts here
are personal and unblinded; no judge calibration or native-speaker agreement is assumed.

`usefulness.v3.json` preserves those four positives and adds the latest qualitative
batch feedback. Conditional name readings, plausible-but-awkward grammar and
contrived meanings remain nonbinary; the twelve shown rows come from six puzzles,
not twelve independent observations. V2 remains unchanged as a hash-pinned input
to the earlier lexical experiment. New human-feedback rendering uses V3; no old
capture is silently regraded into a different historical label version.

The opt-in desktop **Developer review** tab collects real native captures with
actual per-word-count-bucket ranks, per-output verdicts/reasons/notes, separate task
fit and randomized pairwise judgments. See [desktop review instructions](../../apps/desktop/README.md#developer-review-opt-in).
Its local review JSON is evidence for later analysis, not an automatic quality
gate or direct replacement for versioned fixtures. Calibration examples are
explicitly separate. Source-family metadata is retained when matching workload
JSON is supplied; missing families are unknown. No row-level significance claim
or twelve-sample statistical comparison is manufactured.

Quick review defaults to one new distinct contextual output and Yes/Maybe/No,
with optional diagnostics rather than forced reasons or repeated observation
selection. Model-assisted labeling may be used as a separate proposal layer,
not stored as personal human judgments. For an independent model pass, hide human
labels/notes, scores, treatment identities, ranks and source answers; require the
exact displayed phrase, abstention and brief diagnostic reasons. Compare only
after fixing its first-pass labels, using held-aside puzzles/source families where
possible. Report agreement/abstention on definite human labels separately from
human-uncertain cases. No 131-row sample is 131 independent puzzles, and no general
LLM judging result establishes anagram-specific calibration. Keep exact arithmetic
and rank validation in deterministic tools; an LLM's explanation is not that proof.

Returned model reviews can be compared offline without modifying human labels:

```text
cargo xtask review models --manifest .codex/temp/review-inputs.json --output .codex/temp/model-comparison.json --readable .codex/temp/model-comparison.md
```

The manifest contains `packet` (the frozen Markdown packet), `mapping` (its private
item-to-human-row mapping with `export_sha256`), `human_export`, optional
`workloads`, and a `reviewers` array. Each reviewer has `id`, `path`,
`reported_model`, optional `reported_reasoning_effort`, and `provenance_note`.
Input paths resolve relative to the manifest; output directories must already
exist. Model JSON can put provenance at the root or under `metadata`; judgments
use the packet's item IDs, three-way verdict/reason schema, explanations and
verification flags. Preserve original metadata, including null identities and
any disclosed prior label exposure. Reported model/effort is not authentication.

The Rust command verifies the human-export hash, exact packet/mapping/capture
links, unique complete model coverage, optional phrase/puzzle/hash claims and
summary counts before reporting. JSON retains all proposals and three-way
matrices; Markdown shows grouped phrases, puzzle counts, abstention coverage and
an adjudication queue. Family enrichment requires an exact case/request match;
unknown families stay unknown. Output paths cannot replace evidence inputs.
This is language-review intake, not another exact-letter validator or English
oracle. Human uncertainty, model uncertainty and definite disagreement remain
distinct. Model consensus does not relabel a human record, establish independent
agreement or promote a corpus/ranker. No model network calls or production
dependencies are introduced. Test fixtures are synthetic and not real judgments.

The Markdown view also has expandable actual-rank diagnostics for the first
empty-cache trial of each treatment on each bag. These retain duplicate slots
and failures, with treatment names/scores hidden. They are selected diagnostic
observations, not all trials; the separate randomized pooled table is unranked.

### Ambiguity is a separate correctness diagnostic

Finite-vocabulary cardinality, meaningful English, an author's intended answer and
task difficulty are distinct. The independent exhaustive Rust enumerator now has
frozen zero/singleton/multiple/constraint-dependent/repeat cases for both strategies
and caps. Zero/exactly-one require exhaustion; finding two distinct canonical bags
proves multiple even when capped. A single capped discovery remains unknown. These
are explicitly synthetic small domains, with word order not creating another bag.

The review JSON also groups every frozen research dictionary's normalized single
tokens by inventory/length and verifies the illustrative jazz/listen/rhythm/etc.
counts in that exact domain, before request-specific admission. Punctuation/case
aliases collapse. Those mechanically exhaustive counts do not prove semantic or
multiword uniqueness, and no monotonic length/difficulty relationship is assumed.
