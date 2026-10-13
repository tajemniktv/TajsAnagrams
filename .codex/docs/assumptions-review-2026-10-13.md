# TajsAnagrams: assumptions and discriminating evidence

Date: 2026-10-13. Scope: the quality/search/data roadmap in `PLAN.md`, not a
general code audit. No solver, corpus, model, desktop settings or installed app
was changed. This report distinguishes observed failures from proposed remedies.

## Conclusion

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

## Evidence and reproduction boundary

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

## Observations that discriminate between explanations

### 1. The sanity phrase has admission, search, shortlist and order distinctions

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

### 2. Most other fixtures do not fail their isolated ordering test

With all gold words required, six of seven cases return the expected order:
knowledge_power, actions_words, voldemort_reveal, small_birds, keys_table,
and quiet_machine. The sanity phrase is the exception. With normal diverse-500
generation, only knowledge_power includes its gold bag among the six completed
generation calls; small_birds times out.

This supports investigating upstream admission/search before blaming ordering
for every miss. It does not establish general ordering quality, since the inputs
are supplied by an oracle and the fixtures are few and familiar.

### 3. The birds timeout is reproducible without ranking

`small_birds` diverse-500 **generation alone** reaches the 10-second deadline.
Prefix-500 generation completes in about 0.57 seconds and its full solve in
about 1.09 seconds, but neither output contains the gold bag. The gold-only solve
returns the expected order in about 0.70 seconds.

Thus deep linguistic ranking is not necessary to reproduce this timeout.
Strategy-sensitive work in the generation endpoint is a stronger explanation
than a universally expensive ranking stage. One run does not establish a stable
speedup or prove precisely which generation operation dominates. Faster wrong
results are not a quality win.

### 4. Proper names are a different failure class

`voldemort` is absent from the local baseline dictionary and its single-word
generation produces no bag. Supplying the complete required-word bag produces
`i am lord voldemort`. This supports a vocabulary/admission limitation for that
fixture, unlike the sanity phrase. It does not prove ESDB is a better production
lexicon or that admitting every proper name improves overall quality.

### 5. Wide output reveals a separate presentation limitation

At output limit 5,000, prefix returns 4,849 shallow rows with an empty display
phrase and empty `best_order`; diverse returns 4,525. Their underlying `words`
bags are all exact anagrams. The target bag is one such shallow row.

Do not mistake an empty displayed phrase for corrupt letter arithmetic, or
interpret merely increasing the display limit as a useful-result improvement.
TajsBench rejects such rows as non-anagrams because it evaluates displayed
phrases, so it cannot currently grade this wider shallow-output diagnostic.

## Assumptions the current evaluation does not justify

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

## Next discriminating experiments, in order

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

## Validation and remaining limits

Four existing `tests/quality` unit tests passed. The probes made 33 completed
invocations including one recorded generation deadline; an earlier wide-output
probe stopped on the empty-display assertion and was repeated with explicit
diagnostic capture. No app build/install/restart was performed because production
code was not changed. The findings justify narrower experiments, not a corpus
switch, model adoption, or a claim that user-perceived quality has improved.
