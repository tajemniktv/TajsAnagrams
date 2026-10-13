# TajsAnagrams — quality, search and data roadmap

Status: the native Rust engine and installed Windows desktop application are
already implemented. The completed migration, parity work, performance evidence,
desktop acceptance and historical milestones have moved to
[`IMPLEMENTED.md`](IMPLEMENTED.md). This file is deliberately forward-looking.

### Current next step — Phase 5 native baseline design

TajsBench is the existing name; do not start a second TajsAnaBench project.
The Rust measurement/evaluation tools exist, but a trustworthy product baseline
does not yet exist. Phase 5 now has a concrete protocol in
[`tests/quality/README.md`](tests/quality/README.md#next-native-baseline-protocol).
The nineteen inspected curator cases, two actual phrase judgments on one bag,
empty preference file and dirty working-tree snapshot are pilot evidence only.
Design is recorded; collection, new report metrics and a clean baseline freeze
are still required before a broader quality gate can be accepted.

### 2026-10-13 measurement checkpoint (Phase 5, still not gated)

- Native authority: Python oracle/parity/legacy workflows are retired historical
  source. Rust `xtask` owns contract generation, quality evaluation, isolated
  native measurement/stage experiments and active lexical/n-gram/diversity
  research. Independent invariants and exhaustive small-input generation replace
  migration equivalence as ordinary gates. Original stress requests survive as
  explicit native workloads. Stable F-15–F-18 and validation are recorded in
  `.codex/docs/AUDIT.md` Phase D, superseding Phase C's active-oracle policy.
- `cases.v2.json` adds twelve bag-disjoint, project-authored title/clause targets
  with sorted input letters to the seven familiar fixtures. They are curator
  labels, not invented human judgments or untouched holdout evidence; nineteen
  cases still do not justify a comprehensive quality or corpus/model promotion gate.

- A correctness-preserving final-slot feasibility check now avoids allocating
  impossible search children. Three paired trials observed generation median
  reductions of roughly 20–40% on several larger cases; 21 completed real-output
  pairs and 500 ordered reference cases preserved exact enumeration and stop
  semantics. Native/host/quality validation passed, and the local desktop was
  built, installed and restarted. This is a performance improvement, not proof
  of improved ranking or a universal solve-latency percentage gain.
- Two actual user usefulness judgments now live in `usefulness.v1.json`,
  separately from frozen exact-target recovery and still-empty pairwise
  preference data. Both the sanity target and “Several Migrants Tango” were
  judged useful. Unreviewed phrases remain unknown; poor target recovery alone
  is not evidence of poor naturalness. Broader independent judgment is required.
- TajsBench evaluation report v2 defaults to actual desktop bucket/display order;
  global final-score sorting is now a separately named diagnostic. Primary
  quality includes failed attempts, with conditional successful-call quality
  reported separately. Latency-budget reports separate cache states and count
  unique cases independently from repeated trials and snapshots.
- The baseline recorder now defaults to the native desktop request profile;
  a regression test checks for drift against `Settings::default`. Explicit
  native strategy/candidate/deadline overrides remain available. These are CLI measurements,
  not installed-desktop or human acceptance evidence.
- `cargo xtask benchmark --stages` measures word admission, candidate inclusion,
  deep selection, chosen order and final display separately. Optional frozen
  native retained-order feature ablations test scoring hypotheses without
  changing production behavior. Required-word oracle interventions deliberately
  bypass admission/search and must not count as end-to-end quality wins.
- The seven familiar fixtures and empty pairwise-preference dataset remain too
  small to authorize corpus/model replacement or general quality claims. Freeze
  independent holdout cases and collect real judgments before tuning/promoting
  a ranking change. Phases 6–8 are not accepted by this tooling work.

### 2026-09-27 implementation checkpoint (Phases 5–7)

- **Phase 5 in progress, not gated:** `tests/quality/cases.v1.json` starts a
  curated, split-by-letter-bag gold set; `tools/tajsbench.py` validates exact
  anagrams and reports grouped Recall/MRR/NDCG, latency-budget snapshots,
  work/size counters and cold/warm percentiles when recorded runs supply them.
  Pairwise judgments have a separately versioned, currently empty file and
  randomized presentation helper. Four targeted tests pass. The initial seven
  cases are not yet a representative quality gate: collect broader adversarial
  cases, genuine human judgments and comparable baseline runs before accepting
  a ranking/data change.
  A first optimized native CLI snapshot (`diverse`, 500 candidates, 10 s
  timeout, no phrase DB) completed 5/6 then-present cold cases and timed out on
  `small_birds`; among those five, Recall@1 was 0.20, including **no** hit for
  `testinganagramsolver`. Disk-cache warm repeats also had Recall@1 0.20.
  Raw runs/report remain in project `.codex/temp`; this small diagnostic is not
  a frozen acceptance threshold.
- **Phase 6 research only:** ESDB tier ingestion via `tools/research_lexicon.py`
  was tried against the local `normal_user_v2` snapshot. Its 225,552 keys
  include all 30 unique words in the current gold cases, with 63,913 keys not
  in the baseline and 19,482 baseline keys absent. This is coverage, **not**
  quality evidence. OEWN 2025 and 2025+ WNDB load through the existing Rust
  parser; a seven-word smoke comparison preserved the sampled POS/frame facts.
  `tools/research_tubelex.py` imports separate count/video/channel signals;
  the English frequency file covers all 30 sampled gold words. The baseline
  dictionary covers 29/30 (`voldemort` is absent). No corpus was
  switched or packaged. Broader quality, licensing/attribution, proper-name
  and performance checks remain.
- **Phase 7 research scaffold only:** `tools/research_ngram.py` computes
  ARPA backoff order features from a reviewed plain-text model; a synthetic
  scorer test passes. The TUBELEX KenLM repository currently flags its English
  and Spanish ARPA archives as unsafe. The downloaded English archive was
  removed without opening it. Do not use or ship those flagged files absent
  a resolved provenance/security review. No real-model ablation or production
  packaging has happened, so the Phase 7 gate has not passed.

The next goal is no longer “port the Python solver”. It is:

> **Make TajsAnagrams quickly surface exact anagrams people find useful,
> including fluent sentences and coherent titles/noun phrases, with effectively
> zero setup.**

For the sanity bag `testinganagramsolver`, **Testing Anagram Solver** and
**Several Migrants Tango** are both actual user-accepted useful answers.
Recovering the named target is a separate reconstruction diagnostic, not the
sole definition of success or a reason to label every alternative useless.
A user should be able to open the application, enter a puzzle, press Solve and
find useful exact results near the top. They should not need to understand corpora,
language models, training, feature weights, cache identities or any other piece
of machinery that exists solely because software engineers enjoy inventing
new ways to expose implementation details.

---

## Product invariants

These are requirements, not suggestions.

1. **One normal solver experience.**
   - Normal UX is input → Solve → ranked results.
   - The production pipeline may internally contain a lexicon, statistical
     language model, learned ranker, phrase index and multiple scoring stages.
     Those are implementation details, not user-selectable “providers”.
   - Do not make users choose between “baseline”, “5-gram”, “ML”, “neural”,
     “reranked” or similarly meaningless implementation modes.

2. **No user-managed model lifecycle.**
   - No ordinary-user model picker.
   - No “download a model to improve results” prompt.
   - No user training requirement.
   - No path field pointing at a ranker model.
   - No expectation that users know what training data, epochs or feature weights are.
   - If a trained model wins the benchmark and is distributable, it is versioned,
     bundled/provisioned by TajsAnagrams and becomes part of the solver.
   - If it loses, it remains research tooling and does not appear in the release UI.

3. **No LLM dependency in the foreseeable roadmap.**
   - Do not make solving depend on an LLM, hosted API, transformer chat model,
     multi-gigabyte Gemma/Llama/etc. download or prompt-based “naturalness rating”.
   - No network should be required to solve after installation/data provisioning.
   - Neural/transformer experiments are explicitly outside the active roadmap.
     Reconsideration requires a deliberate plan change backed by a large,
     reproducible quality gain and a deployment footprint compatible with the
     zero-friction product goal.

4. **Exact letter correctness remains non-negotiable.**
   - Statistical or learned components may decide *which* valid answer is better.
   - They never get to invent, omit or duplicate letters.
   - Exact generation remains available as a correctness oracle and for exhaustive
     functionality even if the default search becomes quality-directed.

5. **Quality changes must be measured.**
   - “More AI”, “larger corpus”, “more words” and “newer model” are not acceptance criteria.
   - Every ranking/search/data change must beat the frozen current baseline on an
     anagram-specific benchmark or have another explicit reason to land.

6. **Research flexibility, product simplicity.**
   - Development builds/CLI tools may compare alternative corpora, rankers,
     features and search algorithms.
   - The release application selects one supported production configuration.
   - Advanced UI may expose *puzzle semantics* such as required/excluded words or
     exhaustive search. It should not expose the internal scoring implementation.

---

## Current baseline

The completed system already provides, among other things:

- a native Rust exact/lazy anagram generator;
- lexical admission and frequency filtering;
- required/hint/excluded-word constraints and bounded regex exclusions;
- prefix/diverse/exhaustive generation semantics;
- WordNet morphology, POS information and verb frames;
- hand-built grammar, clause, auxiliary, comparative, structure and valency scoring;
- exact/beam retained-order search, diversity and refinement;
- unigram/bigram evidence, phrase attestation and phrase cohesion;
- an explicit-feature linear ranker and deterministic pairwise training tools;
- bounded caches, provenance, cancellation/deadlines and detailed progress;
- parallel deep ranking and phrase preparation;
- a native Tauri/Svelte Windows desktop app with no Python/Node runtime dependency;
- multi-dictionary word search;
- independent native correctness/regression checks and measured empty/reuse cache evidence.

The historical proof, exact checks and migration notes live in
[`IMPLEMENTED.md`](IMPLEMENTED.md).

The current corpus stack, however, is still historically pragmatic:

- Feldman `large.txt`-derived dictionary;
- Norvig / Google Web 1T-derived unigram and bigram counts;
- Princeton WordNet 3.1;
- optional Wikimedia-title phrase SQLite;
- current research/experimental explicit learned-model tooling, which is not the desired final UX.

The repository's own provenance audit already records unresolved redistribution
rights for the Feldman and Norvig-derived data. That is now both a packaging
problem and a quality-improvement opportunity.

---

## Target production architecture

The desired direction is a **single integrated local solver**, approximately:

```text
managed lexicon + frequency data + language model
                    │
                    ▼
          exact letter constraints
                    │
                    ▼
       quality-directed candidate search
                    │
                    ▼
               bag scoring
                    │
                    ▼
          sequence/order search
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
     n-gram      grammar      corpus
       LM        /valency     evidence
        └───────────┼───────────┘
                    ▼
             learned ranking
                    │
                    ▼
          diversity + presentation
                    │
                    ▼
                 results
```

This is still one solver from the user's perspective.

The old exact generator remains valuable as:

- the exhaustive mode implementation;
- a regression/correctness oracle;
- a fallback if a quality-directed search reaches a deadline;
- a source of complete candidate sets for small benchmark cases.

---

## Phase 5 — Establish the native TajsBench baseline

The historical ordering/migration registry froze earlier behavior. The current
native evaluator and nineteen curator targets establish useful measurement
machinery, not representative human quality or a reliable promotion threshold.

Use the protocol in `tests/quality/README.md` as the canonical specification.
Do not count a larger authored fixture file as completion of this phase.

### 5.1 Benchmark dataset

- [x] Versioned cases exist: seven familiar targets plus twelve authored targets.
- [x] Design an initial **120-independent-bag collection target**, not a power
      guarantee: 80 development (including all nineteen already inspected),
      20 validation and 20 locked pilot holdout. Existing `test` tags do not make
      previously examined cases untouched; preserve their historical files.
- [ ] Collect at least 60 independently sourced/user-contributed bags across that
      pool; report their results separately from authored engineering fixtures.
- [ ] Cover sentences, titles/noun phrases, ambiguous multiple-useful-answer bags,
      short/name puzzles, morphology, rarities and constraint-bearing requests.
- [ ] Store source rights/provenance, curator versus human labels, semantic
      context, bag-family identity and exposure history. Keep parameter variants,
      shuffled strings and near-duplicate source families together across splits.
- [ ] Keep adversarial/empty/error/cancellation/corpus-budget checks in an
      engineering suite; do not invent human negative labels for their outputs.

### 5.2 Human preference data

- [x] Two actual standalone useful labels exist; no pairwise judgments exist yet.
- [ ] First judge outputs independently: useful / not useful / unsure, accepting
      coherent titles and noun phrases as well as fluent sentences.
- [ ] Judge a deduplicated, variant-blinded union of actual baseline/candidate
      top-ten outputs per bag; include an original target only as a labelled
      curator/reconstruction reference, not automatic proof that alternatives fail.
- [ ] Add randomized pairwise preference with ties/neither/unsure when comparing
      two already inspected valid outputs. Record presentation, judgment origin
      and disagreement; current preference schema must be extended before use.
- [ ] Freeze the rubric and labels before final candidate scoring. New labels
      change the judgment-set version and require rescoring both revisions.

### 5.3 Metrics

- [x] Native reporting already preserves target Recall/MRR/NDCG, failures,
      cache/trial strata, observed budgets and work/latency counters.
- [ ] Add **useful delivery at 1/5/10**, judgment coverage and lower/upper bounds
      when actual output judgments are incomplete. Failed/no-delivery attempts
      stay in operational denominators; unknown judgments are not negative labels.
- [ ] Report distinct judged-useful answer counts/known-useful coverage at K,
      alongside success-at-K. Showing both accepted sanity answers is desirable;
      a single useful hit should not hide whether the rest of the list helps.
- [ ] Add paired revision deltas with bag-clustered uncertainty; timing repeats
      must not become independent quality cases or confidence samples.
- [ ] Report exact-target recovery separately, plus name/title/sentence,
      source-origin, puzzle-length and constraint cohorts; avoid a composite
      score with arbitrary weights. Small slices remain descriptive.
- [ ] Separate actual anytime delivery from terminal CLI observations; a final
      result cannot establish intermediate 50/200 ms quality or first-result time.
- [ ] Report sampled RSS as sampled, persistence sizes as observed, and raw
      attempt latency/failure coverage alongside completed-call p50/p95.

### 5.4 Ablation harness

- [x] Native stage isolation and paired executable measurements exist.
- [ ] Attribute each candidate change using controlled candidate/order pools and
      equal end-to-end budgets, including admission, generation, shortlist,
      chosen order, final display and a separately labelled required-bag control.

Compare one change at a time:

```text
current baseline
+ new lexical source
+ contextual diversity
+ 5-gram probability
+ LM-guided search
+ richer learned ranker
+ data-layout/performance changes
```

### 5.5 Freeze and acceptance

- [ ] Freeze a clean, signed native source revision; record exact executable,
      corpus, settings, dataset and judgment hashes, compiler/OS/hardware and
      measurement-tool identity. Retain full raw outputs, not just top-three
      projections, and identify installed-app versus CLI observations explicitly.
- [ ] Capture the default desktop-equivalent profile and separate diagnostic/
      stress profiles; use paired runs on the same machine, inputs and budgets.
- [ ] Start with three paired timing trials per bag/cache state, inspect
      variability and expand trials where needed; do not call this a power study.
- [ ] Complete sufficient actual top-output judgments to evaluate the desired
      practical improvement. Set the minimum useful effect and latency/resource
      tradeoffs before final candidate evaluation, using pilot variability.
- [ ] Reserve the locked holdout from tuning and record all exposure. Twenty
      pilot holdout bags are a starting allocation, not a broad significance gate.

Gate: exactness/constraints/lifecycle remain hard native checks. Promote quality
changes only with comparable, adequately judged paired evidence, meaningful
benefit and no unexplained important cohort/resource regressions. Admit
correctness fixes or deliberately chosen tradeoffs for explicit reasons rather
than demanding every change increase one target-recovery percentage. No new
quality gate is accepted merely by writing this design or capturing a pilot.

---

## Phase 6 — Replace the lexical/data foundation

Do this as a benchmarked migration, not a silent corpus swap.

### 6.1 English Speller Database / SCOWL successor

Investigate the English Speller Database (ESDB) as the main lexical source.

Desired properties:

- maintained/versioned source;
- reviewed redistribution terms;
- commonness tiers rather than a giant flat “valid/invalid” set;
- spelling/dialect metadata;
- inflection and basic lexical metadata where available.

Prototype an internal mapping such as:

```text
core/common       → safest normal vocabulary
extended          → valid but less common
word-game/obscure → admitted only with a strong penalty/evidence
```

Do **not** simply maximize dictionary size. Every junk lexeme increases both
branching factor and the probability that a technically valid monstrosity displaces
the phrase a human wanted.

- [x] Build an ESDB ingestion prototype (offline tier/coverage comparison only).
- [ ] Compare practical vocabulary coverage against the current `normal_user_v2`.
- [ ] Quantify false-positive/obscure-word behavior on TajsBench.
- [ ] Review exact redistribution/attribution requirements before packaging.
- [ ] Preserve source tier/commonness as a feature rather than discarding it after
      producing a flat word list.

### 6.2 Open English WordNet

Evaluate migration from Princeton WordNet 3.1 to current Open English WordNet.

- [x] Test the current OEWN release through the available WNDB-compatible export
      before rewriting the parser (2025 and 2025+ load/smoke only).
- [ ] Compare morphology/POS/frame coverage and ranking changes.
- [ ] Evaluate the `+`/proper-name data separately.
- [ ] Proper names should be usable when they genuinely improve an anagram, but not
      receive the same prior as ordinary common vocabulary merely because they exist.
- [ ] Preserve the current WordNet 3.1 snapshot as a reproducible benchmark input
      until the migration has passed quality and parity checks.
- [ ] Record CC BY attribution/version information in packaged-data provenance.

### 6.3 TUBELEX familiarity features

Prototype TUBELEX English frequencies as a replacement/augmentation for the old
Google Web 1T-derived unigram familiarity signal.

Capture at least:

```text
log frequency
video/context diversity
channel/source diversity
```

Rationale: raw occurrence count and breadth of usage answer different questions.
A token repeated absurdly often in a tiny number of contexts should not necessarily
look more human-familiar than a word used moderately across thousands of sources.

- [x] Import TUBELEX English word statistics into the research data pipeline.
- [x] Add contextual-diversity features beside frequency rather than replacing
      frequency with a single magic number (research report only; not runtime).
- [ ] Compare against the current Zipf signal on bag-quality and lexical-admission
      benchmarks.
- [ ] Check licensing/redistribution for the exact artifacts to be shipped.

### 6.4 Canonical lexeme representation

Separate anagram arithmetic from human-facing lexical identity.

Target shape, conceptually:

```rust
Lexeme {
    display: "don't",
    key: "dont",
    signature: CompactInventory,
    commonness: ...,
    frequency: ...,
    contextual_diversity: ...,
    pos_mask: ...,
    dialect/source_flags: ...,
    proper_name: ...,
}
```

`key` participates in exact letter accounting. `display` is what the person sees.

- [ ] Replace the small handwritten contraction presentation table with lexical
      display forms where data permits.
- [ ] Preserve normalized keys for exact hashing/cache identity.
- [ ] Design for apostrophes/hyphens/open compounds without making punctuation part
      of the letter inventory.

Gate: the new lexical foundation must improve or preserve TajsBench quality while
providing a legally reviewable production-data story.

---

## Phase 7 — Add a real 5-gram statistical language model

The current unigram/bigram/phrase features remain useful, but they are not a
complete language model. Add a smoothed word-sequence model so unseen higher-order
contexts still receive meaningful probabilities.

Primary experiment: **order-5 Modified Kneser-Ney**.

TUBELEX publishes English order-5 Modified Kneser-Ney KenLM resources, which makes
it a convenient research baseline. The production implementation does not need to
ship KenLM itself if a smaller Rust-native representation is preferable.

### 7.1 Research scorer

For every retained order, compute at least:

- [ ] total log probability;
- [ ] mean log probability per word;
- [ ] minimum/worst conditional log probability;
- [ ] 2/3/4/5-gram direct-hit or backoff statistics;
- [ ] beginning-of-sequence plausibility;
- [ ] end-of-sequence plausibility;
- [ ] probability margin against the next-best order of the same bag.

- [ ] Run the scorer against the existing retained-order pool first.
- [ ] Do not change search yet.
- [ ] Measure the feature alone and in combination with current grammar,
      structure, phrase and cohesion signals.

### 7.2 Keep phrase attestation

Do not delete the phrase SQLite merely because the language model exists.

They answer different questions:

- n-gram LM: “How probable/natural is this sequence, including via backed-off evidence?”
- phrase corpus: “Has this exact phrase or a long contiguous part actually been observed?”

Use both if ablation shows both contribute.

### 7.3 Production packaging

If the experiment wins:

- [ ] convert the chosen n-gram data into a versioned managed TajsAnagrams artifact;
- [ ] make loading read-only, bounded and hash/version checked;
- [ ] provision it automatically with the application/data package;
- [ ] do not expose an “LM file path” or model selector in normal Settings;
- [ ] record data/model identity in diagnostics and cache keys internally.

Gate: clear quality gain over the current scorer at an acceptable footprint and
latency, with no new ordinary-user configuration.

---

## Phase 8 — Put language probability inside search

This is the largest planned algorithmic change.

The 2019 EMNLP work *Generating Natural Anagrams: Towards Language Generation
Under Hard Combinatorial Constraints* showed that combining exact character
constraints with language-model-guided search can produce substantially more
natural multi-word anagrams than generate-then-rank baselines.

Implement a TajsAnagrams-native version rather than copying the paper literally.

### 8.1 Quality-directed ordered search

Candidate state should conceptually include:

```text
remaining letters
selected words / partial order
n-gram LM state
accumulated sequence score
cheap lexical/bag prior
hint/required state
search priority / remaining bound
```

- [ ] Generate only words that satisfy the exact residual inventory.
- [ ] Order expansions using conditional LM probability plus cheap lexical priors.
- [ ] Correct for pathological preference toward many tiny high-frequency words.
- [ ] Reuse required/hint/exclusion semantics.
- [ ] Make cancellation/deadline checks as strong as the current exact generator.
- [ ] Preserve deterministic tie behavior for a fixed data/model version.

### 8.2 Best-first / anytime behavior

Prefer an anytime search that can improve the result set continuously:

```text
tens of ms   → useful initial candidates
hundreds ms  → stronger shortlist
seconds      → deeper refinement
```

Do not block the UI until an arbitrary candidate cap has been fully enumerated.

The ordinary user still presses one Solve button. Search-budget adaptation is an
engine concern, not a “choose algorithm” wizard.

### 8.3 Exactness claims

Do not claim that a quality-directed search found the globally highest *final*
score unless all later positive scoring terms have mathematically valid upper
bounds and the relevant search space was proven exhausted.

Report internally whether a result came from:

- exhaustive completion;
- quality-directed bounded search;
- timeout/candidate/work cap.

The UI does not need to lecture normal users about branch-and-bound theory, but
diagnostics must remain honest.

### 8.4 Preserve exact generation

Keep the existing exact unordered generator for:

- small complete candidate sets;
- exhaustive/extended functionality;
- quality benchmark generation;
- regression checks;
- fallback/recovery paths.

Gate: at equal time/work budgets, quality-directed search materially improves
Recall/NDCG/human preference over the current prefix/diverse candidate pipeline.

---

## Phase 9 — Redesign learned ranking as a built-in solver component

The current 18-feature linear model and pairwise trainer are valuable baselines,
but their grouping primarily teaches **order preference within one word bag**.
The next system should explicitly distinguish bag quality from order quality.

This distinction is internal architecture. It must **not** turn into two model
selectors in the UI.

### 9.1 Bag-quality features

Evaluate features such as:

- mean/minimum lexical familiarity;
- contextual diversity;
- lexical commonness tier;
- number/severity of rare words;
- OOV/unknown count;
- short-word count;
- duplicate-word penalty;
- WordNet/OEWN coverage;
- morphology-family evidence;
- proper-name count/strength;
- word-count and word-length distribution;
- hint/required evidence;
- cheap corpus association signals.

The bag scorer decides which combinations deserve expensive ordering/structure work.

### 9.2 Order-quality features

Evaluate:

- 2/3/4/5-gram probability/backoff statistics;
- BOS/EOS probability;
- grammar score;
- structure score;
- valency;
- syntax coverage;
- phrase attestation;
- cohesion;
- POS-sequence evidence if phase 12 proves useful;
- spoken/written register signals if multiple corpora prove complementary;
- score margins/confidence.

### 9.3 Keep the linear model as baseline

- [ ] Preserve the current explicit linear ranker for benchmark interpretation.
- [ ] Version the richer feature schema rather than silently mutating
      `anagram-explicit-ranker-1`.
- [ ] Use the old model to detect cases where a more complex learner “improves”
      aggregate metrics while creating ridiculous localized failures.

### 9.4 LambdaMART challenger

Train a gradient-boosted learning-to-rank model offline, with LambdaMART /
NDCG-oriented training as the first serious challenger.

- [ ] Group training rows by source-letter bag / puzzle identity.
- [ ] Use held-out source groups, never random row-level splits that leak near-identical
      candidates.
- [ ] Support graded relevance and pairwise human labels.
- [ ] Include hard negatives generated by the current solver.
- [ ] Compare NDCG, Recall@K, MRR and human preference to the linear baseline.
- [ ] Export the winning fitted trees to a tiny deterministic versioned format.
- [ ] Implement tree inference in Rust; do not ship a Python/XGBoost runtime.

### 9.5 Production rule

If the learned model wins:

- it becomes `ranking-model-vN`;
- it is bundled/provisioned automatically;
- it participates in cache/provenance identity;
- there is no normal user model path, training page or activation checkbox.

The current desktop Experimental training/model-selection features should be
retained only as developer/research tooling or removed/hidden from normal release
builds once the new production pipeline exists.

Gate: one selected built-in ranking model demonstrably improves held-out TajsBench
quality without meaningfully degrading latency or robustness.

---

## Phase 10 — Compact and accelerate the native hot path

Do this under profiler/benchmark control. The point is not to collect Rust tricks
like Pokémon cards.

### 10.1 Compact letter inventories

Current general inventory:

```rust
[usize; 26]
```

is 208 bytes on a 64-bit target. Normal desktop policy currently caps normalized
input far below 255 occurrences per letter.

- [ ] Introduce a request-local compact search inventory, initially `[u8; 26]`
      or an equally simple packed representation.
- [ ] Keep a wider general/public representation if needed for unconstrained APIs.
- [ ] Benchmark hashing, subtraction, copies, dead-state memory and overall RSS.
- [ ] Consider a cached 26-bit presence mask for a very cheap impossible-letter
      rejection before multiplicity checks.

### 10.2 Remove avoidable search allocations

- [ ] Replace candidate `Vec<(usize, usize)>` sparse counts with compact inline
      storage if profiling confirms allocation pressure.
- [ ] Replace per-child cloning of `Frame::chosen` with a shared mutable path stack,
      parent pointers or another ownership-safe non-cloning representation.
- [ ] Map hints to request-local integer IDs and replace `BTreeSet<String>` matched
      state with a bitset where practical.
- [ ] Compact dead-state keys accordingly.
- [ ] Recheck the 4096-state memoization policy once state entries are much smaller.

### 10.3 Specialized two-word solver

For exactly two remaining words:

```text
for candidate A:
    complement = target - signature(A)
    lookup complement signature
```

- [ ] Add a dedicated fast path if benchmarks show measurable wins over the current
      generic DFS + final-slot signature optimization.
- [ ] Preserve candidate/admission ordering and all repeat/hint semantics.
- [ ] Test against exhaustive exact output.

Meet-in-the-middle for 3+ words is research only until profiling justifies its
memory cost.

### 10.4 Adaptive exact order threshold

Replace the static intuition “exact through N words” with an estimated work budget.

Consider:

```text
factorial(word_count)
× selected_bags
× scorer_cost
```

- [ ] Benchmark exact 6/7/8-word order search under real scorer costs.
- [ ] Use exact ordering whenever estimated work is comfortably within the request
      budget; otherwise use the sequence-aware beam/search.
- [ ] Keep deterministic thresholds/versioning so cache identity remains valid.

Gate: each optimization either improves measured time/RSS at equal semantics or
does not land.

---

## Phase 11 — Compile data for startup and search locality

Human-readable corpora are convenient source artifacts, not necessarily good
runtime structures.

Build a versioned native lexicon artifact, conceptually:

```text
tajslex-v1.bin
```

with compact records containing at least:

```text
string/display offset
normalized key/signature
length
presence mask
compact per-letter counts
frequency/contextual-diversity fields
commonness/source flags
POS/proper-name metadata
```

- [ ] Make the build reproducible from reviewed source corpora.
- [ ] Include source identities, schema version and content hash.
- [ ] Prefer mmap/read-only access where it demonstrably helps.
- [ ] Avoid reparsing/renormalizing millions of text records on every cold launch.
- [ ] Consider a trie/FST only after profiling:
      - shared prefixes;
      - exact lookup;
      - possible frequency-annotated traversal;
      - persistent compact representation.
- [ ] Do not replace the current flat representation merely because tries are
      academically pleasing.

Do the same for the chosen n-gram model if a Rust-native compact representation
offers a meaningful deployment/startup benefit over the research format.

Gate: faster startup/search and/or smaller RSS with reproducible bit-identical
compiled data from the same inputs.

---

## Phase 12 — Additional linguistic experiments and release UX cleanup

### 12.1 POS-sequence model

A cheap POS n-gram/Viterbi signal is worth testing because OEWN/WordNet already
provides possible parts of speech.

- [ ] Train/evaluate a small POS trigram model.
- [ ] For ambiguous lexemes, find the best compatible POS sequence.
- [ ] Measure incremental value beyond current grammar/structure and 5-gram features.
- [ ] Integrate it into the single production scorer only if the ablation says it
      adds value. Otherwise delete the experiment.

### 12.2 Spoken vs written register

TUBELEX is intentionally conversational/spoken-like. Some good anagrams are formal,
literary, title-like or name-heavy.

- [ ] Evaluate a second legally usable written-language probability/frequency source.
- [ ] Keep spoken/written scores as separate normalized features.
- [ ] Let the learned ranker combine them; do not merge incomparable raw counts.
- [ ] Only carry the additional runtime data if held-out quality justifies its size.

### 12.3 Release UX

Normal release should converge toward:

```text
[ Type letters / phrase here                         ]
[ Solve ]

1. Testing Anagram Solver
2. ...
3. ...
```

with optional *puzzle* controls nearby, not NLP laboratory controls.

- [ ] Remove/hide user-facing model selection, model-path and training activation
      from production builds.
- [ ] Keep developer training/feature inspection available via CLI/dev build if useful.
- [ ] Make corpora/model updates application-managed and versioned.
- [ ] Prefer installation-time provisioning or one coherent app-data update over
      individual “download this dictionary / model / n-grams” choices.
- [ ] Keep Diagnostics capable of showing engine/data/model identities for debugging.
- [ ] Preserve required/hints/exclusions and exhaustive/extended controls for power
      users where they genuinely describe the puzzle rather than the implementation.
- [ ] Ensure first-run failure messages explain a broken installation/data update,
      not ask the user to become the project's corpus administrator.

Gate: a fresh user can install, launch, enter a puzzle and get high-quality results
without configuring any internal scoring component.

---

## Explicit non-goals / rejected directions

### No LLM solver

Not in the active roadmap.

Reasons include:

- multi-GB deployment;
- latency/RAM variability;
- weak deterministic guarantees;
- difficult reproducibility;
- needless setup burden;
- exact-letter constraints still need a conventional engine anyway;
- prompt-based scoring is a poor production ranking contract.

A future tiny neural scorer is not implicitly approved by this paragraph. Any
neural/transformer work requires a separate explicit decision and must satisfy the
same “ships invisibly as part of the solver” rule.

### Do not maximize dictionary size blindly

A larger dictionary can actively worsen both quality and combinatorial cost.

### Do not merge unrelated corpus counts naively

Keep spoken, written, title, frequency and contextual-diversity evidence as
separate normalized signals.

### Do not prioritize dancing links / Algorithm X

The direct natural-anagram literature found that linguistic evaluation/search
ordering, rather than exotic exact-cover bookkeeping, became the important cost.
Revisit only if profiling the new pipeline points back to exact-cover enumeration.

### Do not delete existing grammar/corpus machinery on intuition

Run ablations. Remove a component only when the benchmark shows it is redundant
or harmful.

### Do not expose research architecture as settings

An experiment being modular in code does not mean the user needs a dropdown for it.

---

## First implementation batch

Keep these patches individually reviewable:

1. **Establish the native TajsBench baseline (Phase 5).**
   - extend existing tooling, not a second benchmark implementation;
   - collect the larger provenance/exposure-aware bag pool;
   - add useful-delivery/coverage and paired comparison reports;
   - collect actual blinded output judgments and freeze full native evidence;
   - retain target-recovery diagnostics without treating every alternative as bad.

2. **Prototype the new lexical evidence offline.**
   - ESDB metadata;
   - OEWN current release;
   - TUBELEX frequency/context diversity;
   - no production corpus cutover yet.

3. **Add the TUBELEX 5-gram Modified Kneser-Ney scorer as a benchmark-only feature.**
   - score current retained orders;
   - run full ablation;
   - decide whether it earns a production role.

4. **If the 5-gram wins, make it managed production data.**
   - version/hash/provenance;
   - Rust scoring path;
   - no user model choice.

5. **Prototype LM-guided quality-directed search.**
   - keep exact generator unchanged;
   - compare quality at equal wall-time/work budgets.

6. **Compact the hottest exact-search state after profiling.**
   - compact inventory;
   - hint bitset;
   - non-cloning path;
   - inline sparse letters.

7. **Build richer BagRanker/OrderRanker datasets and train the LambdaMART challenger.**
   - keep the linear ranker as baseline;
   - export only the selected production trees to Rust.

8. **Clean production UX after the new pipeline stabilizes.**
   - one Solve path;
   - remove research model management from normal Settings;
   - keep diagnostics/dev tooling separate.

---

## Research basis

These sources motivated the new roadmap; implementation still requires local
benchmarking against TajsBench rather than treating any paper as scripture.

- Nishino, Takase, Hirao & Nagata (2019), *Generating Natural Anagrams: Towards
  Language Generation Under Hard Combinatorial Constraints*, EMNLP-IJCNLP:
  https://aclanthology.org/D19-1674/
- Nutrimatic documentation/source concepts: frequency-annotated phrase trie and
  best-first search:
  https://nutrimatic.org/2024/usage.html
- Brysbaert & New (2009), *Moving beyond Kučera and Francis*:
  https://link.springer.com/article/10.3758/BRM.41.4.977
- TUBELEX project and released frequency/model resources:
  https://github.com/naist-nlp/tubelex
  https://huggingface.co/naist-nlp/tubelex-kenlm
- English Speller Database / English word-list project:
  https://github.com/en-wl/wordlist
- Open English WordNet:
  https://en-word.net/
- XGBoost learning-to-rank / LambdaMART documentation:
  https://xgboost.readthedocs.io/en/stable/tutorials/learning_to_rank.html
- Salazar et al., *Masked Language Model Scoring* (background only; no active
  neural/transformer implementation is planned):
  https://aclanthology.org/2020.acl-main.240/
- Kauf & Ivanova (2023), *A Better Way to Do Masked Language Model Scoring*
  (background only):
  https://aclanthology.org/2023.acl-short.80/


## Phase 13 — Website integration and constrained hosting (deferred)

Do not start this phase until the user explicitly resumes website work. No
website repository, VPS or production changes are needed for the completed desktop
milestone. When this phase resumes, recheck the historical website findings in
`IMPLEMENTED.md` and validate a
same-origin service boundary, Host/Origin handling, scoped theme tokens and the
real hosted UI independently of the desktop bridge.

- [ ] Add the SvelteKit page and same-origin `/api/anagram/*` proxy in the website
      repository. Allowlist upstream paths/methods and sizes; never accept an
      arbitrary upstream URL. Keep upstream configuration server-only.
- [ ] Keep the Rust service private on the container network. Preserve admin host
      isolation and the site's same-origin CSP. Inspect actual Caddy/Tunnel config
      before integration; do not assume repository documentation proves live state.
- [ ] Start conservatively with one active hosted job, one CPU worker and a small
      bounded queue. Derive candidate/deep-ranking, input/word-count, elapsed-time,
      memory, result-size and queue limits from measurements under the VPS quota.
- [ ] Enforce limits in the Rust service regardless of UI values. Reject public
      unlimited generation, oversized requests and unsupported expensive options.
- [ ] Apply admission control/rate limits before expensive work, bounded result
      retention and cache eviction. Keep per-job ownership tokens, prevent result
      enumeration and avoid logging raw puzzle inputs by default.
- [ ] Cancel queued and running jobs safely; expire abandoned work. On restart,
      report expired/lost jobs rather than fabricate success. No durable queue
      infrastructure is needed for the initial single-instance deployment.
- [ ] Add container CPU/memory limits, non-root/read-only runtime where practical,
      pre-provisioned corpus data and health/readiness endpoints. Do not download
      corpora for each request or borrow the website's database credentials.
- [ ] Test overload, cancellation latency, memory pressure and website responsiveness
      alongside real host workloads. Tune admission limits before opening access.

Gate: measured acceptable impact on the VPS, known rollback and explicit approval
before deployment. If the host cannot afford even the minimum useful search,
publish the tool page with local-download guidance and hosted solving disabled.
Do not let a solver overload degrade the main site.

## Phase 14 — Broader distribution

This is broader distribution after the completed local desktop milestone.
Website/VPS-specific work stays deferred with phase 13.

- [ ] Build/test native Windows, Linux and macOS artifacts, including the actual VPS
      architecture. Cross-compilation alone is not runtime validation.
- [ ] Package the existing static UI + Rust runtime for each supported platform;
      choose per-user writable managed data/cache paths and verify offline reuse.
      Python/Node remain development dependencies only, never end-user requirements.
- [ ] Ship/provision one reviewed, versioned production data bundle containing the
      selected lexicon/frequency/language-model artifacts. Prefer bundling when size
      permits; otherwise perform one application-managed data update with hash
      validation, interruption recovery, progress and license notices. Do not present
      users with separate dictionary/model/n-gram shopping lists.
- [ ] Add release CI: Rust formatting/lints/tests, TajsBench quality gates, desktop
      UI type checks and integration tests. Add website build/output-leak checks only
      when website integration resumes.
- [ ] Benchmark optimized release builds at equal semantic workloads over multiple
      runs. Publish end-to-end latency, peak RSS, expanded/generated state counts and
      quality—not only hot-loop throughput or a single timing.
- [ ] Treat the native Rust pipeline as the production implementation. Keep archived
      Python only as historical/reference material where still useful; do not make
      an end-user Python fallback part of the distribution.

Optional later work: browser WebAssembly prototype if
corpus size, browser storage, startup and worker constraints prove acceptable.
It is not required for the desktop release. WASM needs its own data/threading and
browser validation plan; native Rust alone does not deliver browser-local solving.
