# TajsAnagrams quality audit: evidence, experiments and improvements

Date: 2026-10-13. This is the single consolidated audit; the initial investigation
is retained below as a historical appendix.

## Outcome

Implemented the measurement corrections and ran the stage-isolation experiments.
The evidence supports several distinct losses in gold-phrase recovery, plus an
ordering-feature representation limitation for the product sanity phrase. It
does not support switching dictionaries, adopting a language model, removing
grammar, or blindly raising search budgets as a complete remedy.

Production Rust/desktop behavior and corpora remain unchanged. Research tooling,
tests, evaluator semantics and the roadmap checkpoint changed. There are still
no actual human judgments, and these seven familiar cases are not a promotion
gate or an untouched held-out set.

## Implementation

- TajsBench report schema v2 defaults to the native desktop display order:
  ascending word-count buckets, preserving native order inside each bucket.
  The former global final-score sort remains an explicitly labelled diagnostic.
- Primary grouped metrics include terminal failures as zero-relevance attempts;
  conditional successful-call metrics and latency are named separately.
- Cache states and trials are distinct strata. Multiple snapshots of a trial
  are not independent attempts. Budget coverage separates unique puzzle IDs
  from trials and does not count cold/warm repeats as additional cases.
- Unobserved budget rankings remain null. `delivery_metrics` measures operational
  delivery over all recorded attempts; no delivery contributes zero without
  inventing an intermediate ranking. Previously delivered partial snapshots
  remain delivery evidence if a later snapshot records a failure.
- Exact shallow bags with no chosen order keep their blank display positions and
  score zero. The evaluator no longer crashes on these diagnostic-wide outputs
  or fabricates an order from their underlying words.
- The baseline recorder defaults to the native desktop request profile, checked
  against `Settings::default` by a regression test. Legacy diverse/500 remains
  explicit. Variant names, requests, corpus/binary hashes, cache state and trials
  are recorded; subprocess/JSON/deadline failures remain evidence.
- The stage recorder isolates single-word admission, generation, deep selection,
  full analysis, chosen orders and a projection of the final display limit.
  Required-word oracle controls bypass admission/search and are not end-to-end
  successes. Retained-order ablations remove a feature's tie-break influence too.

## Reproduction and limits

Native binaries were rebuilt offline using the persistent project `build`
directory, from native source at `660a227` (the working changes were Python/tests/
documentation, not production Rust). SHA-256:

- CLI: `67bf25d36c04ab9a0407c0d093a36fb3879fbaec838d7cafea5a4c29cf43bed5`
- ordering_probe: `6fc0faaf1b2c151a22016c46b1dcbc7db2d9f5daffa4c696ca1b1b0f5f440596`

All raw runs, hash manifests and reports are project-local ignored artifacts in
`.codex/temp/quality-controlled-2026-10-13/`, not included in the commit. They use
local baseline dictionary/ngram/WordNet data, no phrase DB and no learned model.
The source HEAD and dirty-state marker are in each manifest; tool request/seed
identities and full responses are retained. No external artifacts were fetched.

Commands (run serially to avoid shared CPU pressure):

```text
cargo build -p anagram-cli --release --offline
cargo build -p anagram-core --example ordering_probe --release --offline
python tools/run_quality_baseline.py --timeout-ms 10000 --repeats 3 --seed 13 --output .codex/temp/quality-controlled-2026-10-13/desktop.jsonl
python tools/tajsbench.py --runs .codex/temp/quality-controlled-2026-10-13/desktop.jsonl --ranking both --output .codex/temp/quality-controlled-2026-10-13/desktop-report.json
python tools/run_quality_stages.py --deep-all --seed 13 --ordering-probe build/release/examples/ordering_probe.exe --output .codex/temp/quality-controlled-2026-10-13/stages.jsonl
python tools/run_quality_baseline.py --timeout-ms 10000 --shuffle-input --seed 13 --output .codex/temp/quality-controlled-2026-10-13/shuffled.jsonl
```

The builds in this run used `CARGO_INCREMENTAL=0`; no build directories were
cleaned or relocated. These are CLI adapter measurements with a controlled
10-second deadline, **not** the full desktop runtime's 120-second default limit,
installed desktop tests, or visual/user acceptance. OS filesystem cache was
uncontrolled; “disk cache empty” means an empty solver-result cache, not cold OS
pages. Three trials of seven cases are still only seven independent puzzles.

## 1. Corrected baseline: rankings genuinely differ

The desktop-profile baseline recorded three trials for each case, each with an
empty result cache followed by reuse: 42 invocations. Per cache state:

| Ranking definition | Attempted | Completed | Timed out | Recall@1 | Recall@5/10 | MRR |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Desktop display order | 21 | 18 | 3 | 14.29% | 14.29% | 0.1429 |
| Global final-score diagnostic | 21 | 18 | 3 | 0% | 14.29% | 0.0286 |

Only `knowledge_power` recovered its expected phrase. `actions_words` timed out
in all three trials in both cache states. All 18 completed empty-cache calls
reported `cache.hit=false`; all 18 completed reuse calls reported `cache.hit=true`.
Reuse attempts after failures also failed; they were not labelled successful hits.

Completed-call p50/p95 were about 2.14/7.58 seconds for empty result caches and
195/217 ms for actual reuse. These are descriptive timings of completed calls,
not an end-to-end latency guarantee. At five seconds, empty-cache coverage was
15/21 trials across five cases; reuse coverage was 18/21 across six cases.

Regrading the historical six-case snapshot yields 1/6 = 16.67% top-one recovery
per cache state, rather than the old conditional 1/5 = 20%. Do not compare that
number against this new seven-case, different-profile run as an algorithm gain
or regression. The solver was not modified by this research work.

## 2. Generation is usually the earliest loss

The stage matrix holds the desktop admission/scoring profile fixed and varies
only strategy and candidate cap. Each attempted configuration has seven cases
and one trial; deadlines do not count as completed generation or observed absence.

| Strategy / cap | Completed generation | Gold bags observed | Cases with gold bags |
| --- | ---: | ---: | --- |
| Prefix / 500 | 7/7 | 1/7 attempts | knowledge_power |
| Diverse / 500 | 6/7 | 1/7 attempts | knowledge_power |
| Prefix / 5,000 | 6/7 | 3/7 attempts | knowledge_power, testing_anagram_solver, quiet_machine |
| Diverse / 5,000 | 5/7 | 1/7 attempts | knowledge_power |

All 29 ordinary gold words pass single-word admission; `voldemort` does not.
Disabling its frequency cutoff still admits no bag, separating this absence
from a frequency-threshold explanation. The required-word oracle orders its
complete phrase correctly, but does not prove ordinary admission works.

`small_birds` diverse generation times out at both caps; prefix completes.
`actions_words` generation times out at 5,000 for both strategies. Those
generation-only outcomes rule out deep ranking as a necessary cause of these
observed deadlines. Bigger caps and more deep work are not automatic remedies.

## 3. Search, shortlist and display do not imply one another

For the sanity phrase under prefix/5,000, the gold bag appears at position 2,605.
Normal selection leaves it shallow (144 total deep rows). Full analysis evaluates
all 5,000 bags, but its chosen order is still `solver testing anagram`, at rank
1,388 within the three-word bucket. The gold phrase is never displayed. Under
desktop-profile diverse/5,000, the bag is absent; the earlier legacy-profile
result must not be generalized to this different admission configuration.

For `quiet_machine` under prefix/5,000, generation finds the bag, deep selection
analyzes it, and ordering chooses `this machine runs quietly`. It is first in its
own word-count bucket but position **11** in the desktop presentation, after ten
shorter results. Full deep analysis does not change that position. This is an
observed presentation-order effect, not a failure to choose the correct order
or evidence that the result was removed by the per-bucket display limit.

The stage matrix produced 129 native invocations: 31 admission probes, 28 each
of generation/normal-selection/full-analysis probes, seven gold-bag oracles and
seven native order-pool probes. Twelve invocations recorded native deadlines;
the other 117 retained results. Oracle ordering returned six of seven expected
phrases; the sanity phrase remained the exception.

## 4. Weight-only tuning cannot strictly prefer the sanity gold

The native exact ordering probe evaluates and retains all six sanity permutations.
Its four objective features give the competing phrase these values:

| Feature | solver testing anagram | testing anagram solver |
| --- | ---: | ---: |
| Grammar normalization | 0.658418 | 0.546738 |
| Structure normalization | 0.763600 | 0.420000 |
| Valency normalization | 1.000000 | 0.080000 |
| Syntax coverage | 1.000000 | 1.000000 |

The expected phrase is Pareto-dominated by that competitor: every feature is
no better, and three are strictly worse. Therefore **no nonnegative linear
reweighting of these four feature values can make the expected phrase strictly
outscore that competitor**. This is a statement about this frozen feature
representation, not all possible ranking algorithms or corpus evidence.

The native objective ranks the gold fifth of six. Removing grammar, structure
or syntax-coverage weight leaves it fifth; removing valency moves it to fourth.
None changes the winner. The probe classifies the gold as a clause; the evidence
motivates testing title/compound/fragment representation or independently useful
corpus evidence, rather than assuming a coefficient adjustment will suffice.
It does not prove which new feature or model would help.

On the seven frozen retained-order pools, native ordering recovers six gold
orders at rank one. Grammar-objective removal recovers zero, structure removal
five, and valency/syntax removal six. Thus removing grammar is not a supported
general remedy. The six-word keys case retains 512 of 720 evaluated orders;
its ablations are only over that retained subset, not an exhaustive new search.
These are objective-feature ablations, not modules removed from the full solver.

## 5. Source-order leakage control

All seven benchmark inputs exactly preserve their expected phrase's normalized
letter sequence, with spaces removed. That could make segmentation-based methods
appear effective without demonstrating anagram generalization.

A seeded input shuffle was run for all seven cases, with one empty/reuse trial
each (14 invocations). For all six completed cases, **full result buckets were
identical to all three unshuffled trials**, not merely equal top-one metrics.
`actions_words` again timed out in both cache states. Thus source order did not
explain the observed output for the six completed cases. This does not remove
the fixture-design risk for future input-order-aware methods; retain shuffled
controls and collect independently sourced, genuinely held-out cases.

## Validation and next decision

- CLI and existing ordering-probe release builds passed offline.
- All 22 quality tests passed; the suite now covers actual native defaults,
  display ordering, unconditional failure denominators, cache/trial/snapshot
  handling, partial delivery, blank rows, malformed processes, exact inventories,
  feature/tie-break ablations and shuffled-input controls.
- Re-evaluating all 129 frozen stage records with the final recorder code
  reproduced their diagnostic summaries exactly.
- No desktop app build/install/restart or production scoring/data changes occurred.

The most discriminating next research is (1) preserving useful bags through the
shortlist on a frozen candidate pool, and (2) testing a compound/title-sensitive
feature that contributes information not already dominated here. Run admission,
frequency and order-scoring changes separately, not as a bundled replacement.
Promotion still requires independent held-out cases, genuine human judgments,
predeclared quality/resource thresholds and desktop acceptance. None is supplied
by these 185 diagnostic invocations.

## Historical appendix: initial assumptions review before measurement fixes

Date: 2026-10-13. Scope: the quality/search/data roadmap in `PLAN.md`, not a
general code audit. No solver, corpus, model, desktop settings or installed app
was changed. This report distinguishes observed failures from proposed remedies.

### Conclusion

The evidence supports multiple independent bottlenecks, not a single diagnosis
of an inadequate language model. For the product sanity phrase, lexical
admission succeeds; small candidate budgets miss its bag; larger pools find it
but the deep shortlist skips it; analyzing the bag still selects the wrong order.
Replacing the dictionary or increasing the candidate cap alone cannot resolve
all these observed failures. A higher-order model remains an untested remedy.

The existing benchmark measures recovery of seven particular curated phrases,
not general human preference. Its ordering and baseline settings also differ
from desktop presentation. Correct those measurement assumptions before using
it to accept a production ranking or data change.

### Evidence and reproduction boundary

- Source inspected: `32f86579407d42c9c9757690ea4a25637366be65`; initial tracked tree clean.
- Tested the existing `build/release/anagram-cli.exe`, SHA-256
  `9695e4edeeb9f4ab73de50b409e2962cfc74be2577ae217b9c3cd0a6bcb73537`.
  It was **not rebuilt**, so these runtime observations identify that binary,
  rather than proving its correspondence to the inspected source commit.
- Used local `normal_user_v2.txt`, `count_1w.txt`, `count_2w.txt`, and WordNet 3.1;
  no phrase database or learned model. No external data was fetched or processed.
- Every invocation used a 10-second CLI deadline, a 40-second subprocess guard,
  and no solve-result cache. OS filesystem caching was uncontrolled. These are
  single-run diagnostics, not comparative latency estimates or desktop tests.
- Requests start from `tools/run_quality_baseline.py::request(case, 500)`.
  Strategy, candidate budget, output limit, and deep-all changes are identified
  below. The oracle sets `generation.required` to every gold word: this deliberately
  bypasses normal admission/search, so it is not end-to-end success evidence.
- Local reproducibility artifacts: `.codex/temp/assumption_probe.py`, plus
  `.codex/temp/assumptions-2026-10-13/{manifest.json,runs.jsonl,summary.json,followup-summary.json}`.
  The manifest hashes all corpus inputs; JSONL retains full requests/responses.
  Run the script normally, then with `--followup`; runs append to JSONL. These
  project-local artifacts are ignored and are not included in this report commit.

### Observations that discriminate between explanations

#### 1. The sanity phrase has admission, search, shortlist and order distinctions

Target: **Testing Anagram Solver**.

| Intervention | Observation | Supported interpretation |
| --- | --- | --- |
| Generate each target word separately, one-word bags, min Zipf 2.5 | All three admitted | Missing target vocabulary is not the cause here |
| Diverse / prefix, 500 candidates | Gold bag absent in both | No downstream reranker can recover this bag from these pools |
| Diverse, 5,000 candidates | Gold bag at enumeration position 4,285 | Larger budget restores candidate inclusion in this configuration |
| Prefix, 5,000 candidates | Gold bag at position 1,072 | Strategy changes enumeration position; neither 500-candidate variant reaches it |
| Return all 5,000 rows, otherwise normal shortlist | Gold bag has `deep=false` for both strategies | Candidate inclusion does not imply deep analysis |
| Prefix, 5,000, `deep_all=true`, all rows returned | Gold bag becomes `solver testing anagram`, rank 1,765 within the three-word bucket | Bypassing the shortlist does not fix order or competitive bag ranking |
| Give only the gold bag via required words | All six permutations evaluated; winner is `solver testing anagram` | For this diagnostic, beam truncation is not the explanation; order scoring/selection disagrees with the fixture |

The isolated bag scores 73.866 while the same bag in the fully analyzed pool
scores 60.134. Do not compare isolated-oracle scores directly against a different
candidate pool: the intervention changes scoring context as well as generation.

A CLI proxy using the desktop's request defaults (prefix, 10,000 candidates,
min Zipf 1.8, no repeats, common short words plus `hi`, phrase bonus 6, ten
results per group) finds the gold bag at position 2,605 but does not display it.
The output begins `several migrants tango`. This is not an installed-app test:
it retains local CLI corpora, no phrase DB, and a 10-second rather than the
desktop's default 120-second runtime deadline.

#### 2. Most other fixtures do not fail their isolated ordering test

With all gold words required, six of seven cases return the expected order:
knowledge_power, actions_words, voldemort_reveal, small_birds, keys_table,
and quiet_machine. The sanity phrase is the exception. With normal diverse-500
generation, only knowledge_power includes its gold bag among the six completed
generation calls; small_birds times out.

This supports investigating upstream admission/search before blaming ordering
for every miss. It does not establish general ordering quality, since the inputs
are supplied by an oracle and the fixtures are few and familiar.

#### 3. The birds timeout is reproducible without ranking

`small_birds` diverse-500 **generation alone** reaches the 10-second deadline.
Prefix-500 generation completes in about 0.57 seconds and its full solve in
about 1.09 seconds, but neither output contains the gold bag. The gold-only solve
returns the expected order in about 0.70 seconds.

Thus deep linguistic ranking is not necessary to reproduce this timeout.
Strategy-sensitive work in the generation endpoint is a stronger explanation
than a universally expensive ranking stage. One run does not establish a stable
speedup or prove precisely which generation operation dominates. Faster wrong
results are not a quality win.

#### 4. Proper names are a different failure class

`voldemort` is absent from the local baseline dictionary and its single-word
generation produces no bag. Supplying the complete required-word bag produces
`i am lord voldemort`. This supports a vocabulary/admission limitation for that
fixture, unlike the sanity phrase. It does not prove ESDB is a better production
lexicon or that admitting every proper name improves overall quality.

#### 5. Wide output reveals a separate presentation limitation

At output limit 5,000, prefix returns 4,849 shallow rows with an empty display
phrase and empty `best_order`; diverse returns 4,525. Their underlying `words`
bags are all exact anagrams. The target bag is one such shallow row.

Do not mistake an empty displayed phrase for corrupt letter arithmetic, or
interpret merely increasing the display limit as a useful-result improvement.
TajsBench rejects such rows as non-anagrams because it evaluates displayed
phrases, so it cannot currently grade this wider shallow-output diagnostic.

### Assumptions the current evaluation does not justify

1. **Recall@1 is what the desktop user sees.** `tajsbench.py::extract_rows`
   globally sorts by final score. `Results.svelte` displays ascending word-count
   buckets in their native order; native ranking prioritizes deep rows and uses
   pre-score for shallow rows. These are different ranking definitions, even if
   they happen to agree on an individual result.
2. **The diagnostic configuration represents defaults.** The runner uses
   diverse/500/min-Zipf-2.5/repeats/40 rows, whereas desktop defaults use
   prefix/10,000/min-Zipf-1.8/no-repeats/10 rows, among other differences.
3. **Historical 20% means 20% success over all requests.** Re-reading the saved
   September baseline gives one top-one hit out of five successful calls per
   cache state, with six attempted. Counting the timeout as failure gives 1/6
   (16.7%). That historical run omitted the now-present Voldemort case.
4. **Budget coverage counts distinct puzzles.** `quality_at_budgets` keys by
   `(case_id, warm)` but labels the count `covered_cases`; the historical
   five-second report says ten for a six-case dataset. At 200 ms only the five
   disk-cache-warm successes qualify. This cannot demonstrate a fresh 200-ms
   solve. Report cache states and attempted-case denominators separately.
5. **An unlabelled answer is unnatural.** All seven fixtures have one positive;
   `preferences.v1.json` has zero judgments. Negative labels include grammatically
   interpretable alternatives such as `Power Is Knowledge` and `At Dawn Sing
   Small Birds`. Intended-phrase recovery and preference among valid anagrams
   are different tasks. Neither labels the other automatically.
6. **Bag-disjoint splits remove all leakage.** Several test targets already
   occur in the project's ordering registry and have been used during prior
   development. Bag separation is useful but does not make familiar fixtures
   an untouched generalization test.
7. **Bigger/newer data or a 5-gram model is established as the remedy.** The
   roadmap records vocabulary coverage, a small WordNet smoke comparison and
   synthetic ARPA tests, not controlled end-to-end wins. Larger vocabularies
   could also increase competition and search work. That is a hypothesis to
   measure, not a demonstrated regression or improvement.

### Next discriminating experiments, in order

| Question | Controlled experiment | Decision rule |
| --- | --- | --- |
| Can benchmark scores represent the product? | Freeze cases, requests, binary/data hashes and desktop display order. Report full attempted coverage, deadline outcomes and cold/disk-warm/process-warm separately. Keep final-score ranking as a separately named diagnostic. | Do not accept a model or search change using a different ranking definition or denominator. |
| Which candidate stage loses useful answers? | Hold admitted vocabulary and gold bags fixed; record inclusion at generation, pre-shortlist, deep selection and displayed output. Compare prefix/diverse at fixed work and, separately, matched wall-clock budgets. | Change search only if candidate recall improves at the chosen budget; change shortlist if generated gold is systematically skipped. |
| Is the sanity failure scoring or an unfair target preference? | Retain all six permutations with feature contributions; blind judges to system/source and separately ask intended-phrase recovery and standalone naturalness. Disable grammar/structure/valency/collocation components one at a time on fixed pools. | A scoring change must beat current ordering on independent cases, not just force the named sanity answer upward. |
| Will lexical or language-model replacement help? | Separate admission-list-only, frequency-only, and fixed-pool order-score-only ablations. Include proper names and morphology plus common-word controls. Measure false positives and search expansion as well as gold coverage. | Coverage alone cannot authorize replacement. Promote only after paired held-out quality and resource results satisfy predeclared thresholds. |
| Does a gain generalize? | Expand development cases across lengths, repetition, names and ordinary clauses; collect real judgments and freeze a new family/source-disjoint held-out set before tuning. Randomize run order, repeat timings, and report paired per-case differences with uncertainty. | No human-preference claim without judgments; no latency win from a single sequential timing; no generalization claim from seven familiar targets. |

Do not bundle lexicon, search, shortlist and scorer changes into one initial
experiment: that would hide which intervention actually helped. Prefer the
smallest successful intervention over assuming the whole proposed architecture
is necessary. No experiment proposed here authorizes use of previously flagged
ARPA artifacts, resolves redistribution rights, or substitutes for desktop
runtime/visual acceptance.

### Validation and remaining limits

Four existing `tests/quality` unit tests passed. The probes made 33 completed
invocations including one recorded generation deadline; an earlier wide-output
probe stopped on the empty-display assertion and was repeated with explicit
diagnostic capture. No app build/install/restart was performed because production
code was not changed. The findings justify narrower experiments, not a corpus
switch, model adoption, or a claim that user-perceived quality has improved.
