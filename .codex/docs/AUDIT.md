# TajsAnagrams audit: architecture, assumptions and validated evidence

Date: 2026-10-13. This is the single consolidated audit. Architecture findings
are revalidated below; the earlier quality experiments and initial investigation
retain their own dated evidence boundaries rather than becoming fresh claims.

## Phase E: product baseline design (2026-10-13)

The next work is Phase 5 of `PLAN.md`: establish the native **TajsBench** product
baseline, not create another TajsAnaBench implementation. The existing tools are
implemented; representative collection, broader actual judgments and a clean
freeze are not. The canonical design now lives in the existing
`tests/quality/README.md`, with completion tracked in `PLAN.md`.

| Stable finding | Disposition | Evidence and design response |
| --- | --- | --- |
| F-19: product plan treated one named answer as the sanity success condition | Corrected | The user reaffirmed both “Testing Anagram Solver” and “Several Migrants Tango” make sense and can be useful; showing both is desirable. PLAN now separates useful-answer delivery from reconstruction. These are still two standalone positives, not a new pairwise preference/tie judgment. |
| F-20: existing split tags could be mistaken for an untouched holdout | Qualified; new split/exposure protocol designed | All nineteen current cases have been inspected and measured. Their four `test`, one `train` and fourteen `validation` tags do not establish untouched generalization evidence. Existing files stay historical; all nineteen belong to development in the next pool. |
| F-21: the native pilot snapshot was not a complete promotion baseline | Qualified; clean/full freeze pending | `native-baseline.v1.json` explicitly records a dirty working tree, one trial per cache state and top-three-per-bucket projections. It cannot establish full top-ten usefulness, measurement stability or a clean release baseline. Preserve it; collect full raw output under a new baseline identity. |
| F-22: sparse useful-row counts were not whole-dataset useful-delivery metrics | New report design pending implementation | Current report v2 reports sparse standalone rows on judged bags. The new protocol requires attempt-denominated useful-success, unknown-label bounds/coverage, distinct useful answer counts and finite known-useful coverage, plus paired bag-clustered comparisons. No new metric is claimed implemented here. |

The proposed initial collection budget is 120 independent residual letter bags:
80 development, 20 validation and 20 locked pilot holdout, including at least
60 independently sourced/user-contributed bags. This is a practical starting
allocation, not statistical power or a release threshold. Source families,
constraint variants and shuffled inputs must not leak or inflate independent
case counts. Sentences, coherent titles/noun phrases and multi-useful-answer bags
all belong in the quality pool; correctness/error/stress cases remain a separate
engineering track. Authored/published targets are not automatically actual human
naturalness labels, and unknown alternatives are not negative examples.

The user has no additional personal puzzle examples yet and offered to dispatch
Echo for public real-use sourcing. A source-packet brief is included in the
existing benchmark README: links/context/constraints, exact-letter checks,
answer/endorsement provenance, deduplication and exposure. This review did **not**
dispatch Echo, perform an online collection or invent user examples/judgments.

This checkpoint changes documentation/design only, not solver scoring, corpora,
datasets, evaluator code or deployment. Current source/fixtures confirm the
nineteen-case count, two actual labels, empty preference set and pilot limitations;
local links and Markdown diff checks validate the documentation. Existing native
test/build/dogfood results remain dated Phase D evidence, not fresh checks from
this design turn. Phase 5 is still in progress; next implement the report gaps,
collect/source and judge actual outputs, then freeze and compare native revisions.

## Phase D: Rust authority and Python-free development (2026-10-13)

This phase supersedes Phase C's active-oracle policy. The user explicitly chose
TajsAnagrams as a successor with Rust authoritative, not perpetual reproduction
of AnagramSolver. Historical scores, orders, caches and experimental CLI formats
are evidence, not ordinary acceptance requirements. The source DOCX's suggestion
to retain useful Python validation was a proposal for its older snapshot, not an
instruction overriding this new product decision.

| Stable finding | Current disposition | Replacement and evidence |
| --- | --- | --- |
| F-15: historical Python behavior remained authoritative in ordinary gates | Retired | Python oracle/parity/legacy workflows no longer execute. Independent native letter/constraint verification, exhaustive small-input generation and intended lifecycle/cache/intake tests define correctness. Native score/order changes require interpretation, not automatic Python equality. |
| F-16: active development tools required a Python runtime | Replaced | Development-only `xtask` owns contract generation, TajsBench reporting, isolated native measurement, paired revision comparisons, stage diagnostics and ESDB/ARPA/TUBELEX research. Node/AJV remains an independently implemented schema consumer; PowerShell/Node still serve desktop/frontend tooling. |
| F-17: retirement could remove meaningful verification or stress coverage | Replaced before retirement | One-time evaluator/research comparisons matched existing tooling outputs; native tests cover metric and failure semantics. Five explicit stress requests survive at original 100,000 caps without Python result targets. Removed migration-only reference tests are not represented as equivalent native coverage of every old helper. |
| F-18: seven familiar targets were too narrow for quality research | Expanded, not a promotion gate | `cases.v2.json` adds twelve bag-disjoint authored title/clause targets with sorted input letters. These are curator relevance, not invented user judgments or untouched holdout samples. Actual usefulness labels and pairwise judgments remain separate. |
| F-07: stale App response publication | Still open | This tooling/authority change does not fix the previously reproduced main-component race. |

`cargo xtask check` runs formatting, core/CLI/tool tests, strict Clippy, contract
drift and independent AJV/TypeScript fixtures. It also rejects active Python
sources under tools/tests and Python runtime commands/setup in workflows.
The generator's independent test enumerates combinations using its own small
alphabet counts and hint logic, not production Inventory/subtraction/pruning.
It checks 1,000 seeded configurations across prefix/diverse strategies and
unbounded/1/3 caps, plus repeat runs for deterministic output. The separate
development evaluator verifies exact residual letters, subtraction, required
multiplicity, hints, exclusions/regex/characters, search repeats, word counts,
unique bags and chosen-order word identity without calling core validators.
These are independent implementations in Rust, not wrappers around the code
being checked, and do not claim exhaustive correctness for every possible input.

Contract output retains identical schemas/types; only the eight TypeScript
generator attribution headers change. TajsBench retains report schema v2,
desktop versus diagnostic ranking, failure denominators, cache/trial/snapshot
strata, partial delivery, unknown budget observations and separate actual
usefulness/preference labels. A result absent by a time budget is not assigned an
invented intermediate ranking. Stage interventions keep shortlist/order/display
losses distinct; required-bag controls are not ordinary search successes.
Paired native revisions report equality and speed ratios informationally rather
than imposing historical score/order parity. Empty or failed pairs remain unknown.

Benchmark capture owns project-local temporary caches, bounds stdout to 64 MiB
and stderr to 2 MiB, retains malformed output/nonzero exits/launch failures and
engine/subprocess deadlines, and samples native process-tree resident memory.
The sampler sleeps 20 ms between refreshes; wall time includes capture/sampling
overhead. Sampled RSS is not a guaranteed peak, and these timings must not be
directly equated with differently instrumented old reports. It is a native
CLI/probe harness, not a general daemon/process supervisor. Successful cold/warm
cache roundtrips must preserve exact buckets and report a cache hit.
ESDB opens read-only, ARPA retains queried contexts/backoff evidence, and TUBELEX
accepts plain/XZ TSV with actual expanded/read-byte limits. Nonfinite queried
ARPA entries, duplicate queried entries and negative TUBELEX integers now reject
instead of becoming misleading research values. No corpus/model is promoted.

### Phase D validation and evidence

- **141 core/CLI/xtask tests**, strict all-target Clippy, generated **16**
  contract artifacts and independent **19** AJV/TypeScript fixtures passed through
  `cargo xtask check` after retirement. **19** desktop host tests and **21**
  frontend/build-policy regressions also passed: **160 Rust tests** in total.
  Rust **1.90** checks the tooling successfully; the Windows advisory check passed.
- Before retirement, one-time comparisons reproduced **every report field** for
  the recorded 42-invocation desktop and 14-invocation optional-artifact captures,
  all **129** native stage summaries, and synthetic ESDB, ARPA and compressed
  TUBELEX research outputs (numeric tolerance 1e-12). This validates tool
  replacement, not continued solver equivalence. Those checks are not active CI.
- New real-child tests cover invalid JSON/shape, nonzero exit, native error,
  external timeout, oversized output and successful RSS/result capture. Native
  research tests cover ESDB filters/tiers/collisions/read-only behavior and XZ
  parsing/expansion limits. Existing native lifecycle/cache/intake/training and
  malformed corpus tests remain ordinary gates; no old solver is needed.
- All five explicit native stress workloads completed cold/warm, including both
  100,000-generated-candidate cases. Exact letters/constraints and successful cache
  roundtrips passed; historical Python scores/orders are deliberately not asserted.
- Two native generation cases over three randomized paired trials completed all
  six pairs and reported equal outputs. Speed ratios were about **1.00** and
  **0.99**: plumbing evidence, **not a performance improvement**. Binary hashes,
  identical requests and input hashes are recorded.
- The expanded 19-case run recorded **38 attempts**, **36 completions** and
  **two `actions_words` ten-second deadlines**, both retained in denominators.
  Desktop exact-target Recall@1/MRR was **1/19** per cache state. This is an
  expanded different dataset, not a regression/improvement relative to the old
  seven-case percentage and not a measure of all independently useful answers.
  `native-baseline.v1.json` preserves this explicitly hashed **dirty working-tree
  native snapshot** with top-three-per-bucket projections and failure evidence;
  it does not claim a clean release pin or freeze future scores as a gate.
- The new native stage runner completed admission, zero-frequency controls,
  generation, shortlist, full analysis, required-bag and ordering-pool captures
  on the sanity case. A separate native optional phrase/model smoke completed
  four cold/warm attempts with explicit artifact hashes. The earlier losing
  research model stays unpromoted; these are pipeline checks, not quality gains.
- Actionlint **1.7.12** validated all retained workflows (checksum-verified local
  tool, shellcheck/pyflakes integrations disabled). No hosted Actions execution
  or branch-protection change is claimed. Native CI no longer provisions Python
  or downloads WordNet solely for migration parity; ordinary tests use synthetic
  native fixtures. Real-data measurement jobs remain separate and informational.
- Release desktop build/type checks, transactional installation and restart
  passed. Installed/build executable SHA-256 is
  `6EEEDFEDAB91515F671508DA892D5F9C9DDF14F33CC8F599146C901BB45B51AE`,
  installed at `%LOCALAPPDATA%/Programs/TajemnikTV/TajsAnagrams/TajsAnagrams.exe`.
  PID 21864 was launched. Existing settings/data were preserved; no experimental
  tooling dependency is part of the installed application's dependency graph.
  This is build/install/launch evidence, not fresh rendered/user interaction.

Retired Python sources, unit/parity scripts, legacy runners and their workflow
are retained under ignored local-only `.codex/archive/python-retired-2026-10-13/`.
The initial archive manifest records **218 files** including ignored historical
caches; every recorded content hash was verified after moving. Later loose
tools/quality bytecode caches were also moved there. **93 tracked Python files**
are removed from active source; Git history at `4b2a827` remains another recovery
source. The ordinary fixture dictionary moved to `tests/fixtures/dictionary.txt`,
and its real CLI tests/build smoke caller were updated. Historical JSON/provenance
records and the original DOCX copy are preserved, not rewritten as new evidence.
No system Python installation was uninstalled. Future disposable research can
use another language if justified without restoring a required Python pipeline.

Raw replacement comparisons, captures and final checks are retained under
`.codex/temp/native-authority-validation/`. Corpus rights, parser/SQLite process
memory boundaries, F-07, public packaging/platform evidence and broader real
human-quality judgments remain unresolved; retiring migration parity does not
resolve them or require rewriting superseded algorithms again.

## Phase C: port gaps and active-pipeline organization (2026-10-13, superseded by Phase D)

This phase supersedes the compatibility/harness dispositions in the earlier
port review below. `IMPLEMENTED.md` confirms the accepted migration policy:
useful feature coverage and hard invariants, not perpetual byte-for-byte Python
equivalence. The pinned Python logic stays a test oracle, not a production fallback.

| Stable finding | Disposition | Implementation and acceptance evidence |
| --- | --- | --- |
| F-08: missing letter-subtraction surface | Implemented | Optional native `subtract` chunks remove letters without adding output words; required words consume the residual. CLI and desktop Advanced expose it. Tests cover normalization, unavailable/overlapping chunks and required multiplicity. |
| F-09: missing native import/stream/report/export conveniences | Native equivalents implemented; historical formats retained as compatibility tools | Repeated exclusion-file imports; during-enumeration accepted-bag JSONL streaming; native JSON/deep-result exports; scoped hint statistics and answer diagnostics. New-file-only publication, cap-probe exclusion and exact CLI outputs have tests. These do not silently widen returned ranked rows or claim compatibility with Python's old text export format. |
| F-10: active pipelines depended on archive contents | Reorganized | All 56 executable/config/test files moved together to `tests/oracle/`; active tools/workflows now point there. Three root-path adaptations retain existing corpus/registry locations. Archive contains historical documentation only; hiding it is an explicit pipeline acceptance test. |
| F-11: parity harnesses conflated intentional differences with port defects | Policy implemented | `tests/parity/compatibility.py` centralizes exactly three reviewed grammar corrections and constructs the exact expected native `display_phrase`. Shared fields, unknown fields, order and score comparisons stay strict. Negative-row rejection is tested separately, not deleted from coverage. |
| F-12: native trainer existed without differential fitted-weight proof | Differential harness added | `training.py` compares 12 fixed-seed fold/epoch/regularization configurations, fitted weights and pooled held-out metrics against Python. Inputs remain bag-grouped. Native validation/update budgets remain intentional stricter boundaries. |
| F-13: mutable policy-cache metadata treated as immutable corpus bytes | Corrected narrowly | All raw dictionary/count/WordNet inputs remain SHA-256 checked. Only regenerated `normal_user_v2.json` is independently validated against the exact frozen lexical policy, current source token/stamps and recomputed short-word list; unknown fields or changed policy still fail. Historical hashes are not rewritten. |
| F-14: current corpus/performance jobs still executed the Python engine | Native workflows selected | Existing Rust preparation/build/train/solve commands supersede the old engine paths. Thin Python capture/report tooling remains useful; obsolete Python internal caches/backends are not reimplemented in Rust. Legacy experiments remain optional reference commands. |
| F-07: stale App response publication | Open, outside port compatibility work | The previously reproduced component-script race is not fixed or reclassified by this phase. |

Native code changes are in `request.rs`, `generation.rs`, `cache.rs`, `solve.rs`
and CLI `compat.rs`/`main.rs`. Subtraction runs in linear inventory/target passes,
normalizes the **residual target** in results, participates in cache identity and
keeps required-word conflict/multiplicity checks. Cache-hit presentation uses the
same validated residual target rather than the original text. Original-input
deployment limits remain conservative; subtraction does not bypass them.

The generation observer sees unique accepted bags (including required words),
never the extra truncation-detection bag. It is opt-in at the API/CLI surface;
normal requests retain enumeration/stop semantics. Streaming does not eliminate
the engine's retained bag vector. A transcript begins `complete:false`; failures
or interrupted work cannot acquire a successful completion footer. Exports never
overwrite an existing file, but an I/O failure can leave a partial **new** file.
Native deep exports filter returned rows and explicitly retain original counters;
answer/hint diagnostics name returned-bag/returned-row scope and run after scoring.

No active script is lost to make archive appear clean. The Python compatibility
CLI, corpus builder, trainer, benchmark/ordering/refinement/matrix runners and
258 tests remain executable under `tests/oracle/`; their deployed-native
counterparts are documented in the existing `NATIVE_TOOLS.md`. Workflows preserve
reference capabilities, but current phrase/performance jobs now run native
preparation and solving; the phrase job also uses native grouped build/train.
Python only captures measurements/reports in those jobs. The obsolete Python
test/benchmark workflows are not treated as the current application pipeline:
reference validation is manual or selected by reference-file changes. Native CI
still uses valuable independent Python comparisons alongside Rust tests, with
selected ordering/ranking/storage/model/training comparisons added.
The current phrase job uses Wiktionary by default; expensive combined Wikipedia
data is an explicit manual input. Scheduled runs prepare/cache inputs but do not
claim benchmark execution. Legacy full matrices remain callable through the
oracle wrapper, not reconstructed as duplicate Rust pipelines.
Historical source manifests keep their recorded paths/hashes; lookup
accepts old names but resolves **only current oracle files**, not archive fallback.
Pinned identity checks permit only the three documented path relocations.

Strict-policy regressions demonstrate that a wrong display string, an unknown
extra field or changed score still fails. Presentation is independently expected,
not stripped from actual results. Grammar adaptations occur in memory before
ordering/ranking/solve tests, without rewriting the pinned Python source.
The preceding raw-failure logs remain historical evidence rather than being
relabeled passes; fresh policy-aligned runs are recorded separately.

### Phase C completed validation and dogfood

- **126 native/desktop tests**, strict all-target Clippy, **31 quality tests**,
  **21 UI/build-policy tests**, generated-contract checks and **19 independent
  shared schema fixtures** passed. Desktop type checking had zero errors/warnings.
- With the actual `archive/` directory temporarily moved out of the workspace
  root (and restored in `finally`), all **258 oracle tests with CLI integration**,
  frozen behavior verification, generation (500), scoring (200), real-corpus
  admission (24), WordNet (6,483), reviewed grammar (305), ordering (118), ranking
  (12), refinement (160), phrase/cohesion (400), storage, model inference (253),
  training (12) and full solve (10, including phrase cold/warm/rebuild) checks
  passed. These case counts are coverage, not independent human judgments.
- The full **five-workload native registry** passed cold/warm checks, including
  both 100,000-candidate workloads at original caps. The retained user-CLI smoke
  also passed all five cases via `tools/run_oracle.py`; retained refinement A/B
  ran successfully with UTF-8 output and no archive dependency. Initial attempts
  that invoked non-argparse runners with `--help` from the wrong cwd were not
  valid pipeline tests; the wrapper and real invocations provide the evidence.
- Integration regenerated the local derived dictionary-policy cache. Its old
  metadata SHA no longer matched the historical manifest, while **all other
  recorded corpus hashes still matched**. F-13 validates that one cache's exact
  derived meaning/identity rather than ignoring drift or manufacturing the old
  bytes. Bad short-word lists, invented source identities and unknown fields have
  rejection tests. Original dictionary/count content was not refreshed.
- Native preparation was exercised offline using existing dictionary/count/
  WordNet sources and six **project-authored synthetic titles**, producing a
  24-row phrase index. Native grouped dataset construction and training completed
  for 52 bags. Native pooled baseline metrics were R@1 **0.3654**, MRR
  **0.5405**; fitted held-out metrics were R@1 **0.3269**, MRR **0.5073**. This
  losing model is not promoted. The saved all-group fit is not generalization
  evidence, and synthetic titles are not a validated real Wikimedia corpus.
- A native full-solve capture with those explicit phrase/model artifacts recorded
  14 attempts over seven familiar fixtures: 12 completed, two `actions_words`
  deadlines at ten seconds. Their failures remain in TajsBench denominators.
  Optional artifacts are hashed in manifests and variant IDs. This verifies the
  pipeline/report plumbing, not a quality gain or a representative speed estimate.
- Both revised workflow YAML files parse and their native preparation/training/
  capture steps were exercised locally. No hosted Actions run, live corpus
  download, full Wikipedia matrix, branch-protection change or public release is
  claimed. Reports upload separately from raw corpora/trained model artifacts.
- The release desktop build, transactional installation and restart succeeded.
  Installed executable equals `build/release/TajsAnagrams.exe`, SHA-256
  `7DF10E4D81D64ACC5706104E8D38EBD4B05829933714D9CEC47DEE15CE80847F`,
  at `%LOCALAPPDATA%/Programs/TajemnikTV/TajsAnagrams/TajsAnagrams.exe`.
  PID 39464 was running/responding with title TajsAnagrams. Existing installation
  data/settings were preserved; the research model was not selected as a default.

Raw logs/captures are project-local in `.codex/temp/port-validation/`. The local
native preparation/model inputs remain isolated there. This is build/install/
launch and CLI/API evidence, not a rendered or physical interaction test of the
new desktop control. No fresh visual/control-interaction acceptance is claimed.
F-07 and the previously documented data
rights/process-memory boundaries remain open. No redundant legacy Python
implementation is ported merely to increase the Rust line count.

## Python-to-Rust port completeness review (2026-10-13 follow-up)

**Not a complete, behavior-identical port of everything in `archive/`, and not
proven correct for all inputs.** The supported native solver has implementations
of the main generation/ranking pipeline and substantial differential evidence.
There are explicit compatibility differences, unported legacy conveniences and
remaining validation gaps. The installed solver is independent of Python;
that does not mean every Python script or behavior was translated to Rust.

Review pin: `fa3c7d2` (initial worktree clean); native implementation unchanged
from the architecture validation above. `tests/reference/NATIVE_PHASE3_ACCEPTANCE.md`
records acceptance under a user policy prioritizing feature coverage, useful
results and speed over exhaustive one-to-one equivalence. It does not authorize
calling the port universally equivalent. The opening comment in core `lib.rs`
still refers to future full parity gates; that is not a machine-checked certificate
of completeness, nor proof that the desktop calls Python.

### Feature coverage and remaining compatibility boundaries

| Python responsibility | Current native coverage | Remaining distinction |
| --- | --- | --- |
| Normalization, letter inventories, required/hint/excluded words, word/length/frequency/short policies, repeats, prefix/diverse bounded generation | `lib`, `lexicon`, `generation`, `request` | Frozen English alphabet, not multilingual; native defaults/validation are not all legacy CLI defaults. |
| Lexical, pair, family and pre-ranking; WordNet morphology/POS/frames | `scoring`, `wordnet`, `ranking` | Requires comparable exact corpora/options; port parity is not dictionary equivalence. |
| Local grammar, noun phrases, clauses, auxiliaries, comparative/parallel structures, validity | `grammar`, `phrase`, `clause`, `auxiliary`, `comparative`, `structure`, `validity` | Three reviewed changes deliberately differ from frozen core Python helpers; raw ordering comparison fails until the comparison oracle accounts for them. |
| Exact/beam retained order pools, diversity, shortlist/family expansion, refinement, final buckets | `ordering`, `diversity`, `ranking`, `refinement`, `solve` | Tested pools are finite. Full-pipeline diagnostic control is needed for additive fields and reviewed grammar differences; see below. |
| Positive bigrams, SQLite phrase evidence, cohesion and alternative-order rescoring | `corpus_ranking`, `phrase_index`, `phrase`, `cohesion`, `phrase_evidence` | Rust rejects some malformed corpora Python accepts/skips; canonical SQLite table/key constraints differ. |
| Explicit 18-feature learned ranker and training | `learned`, `training`, native `ranker-build/train/rank` | Inference/model parity is tested; native build/train round-trip tests are not a proof of identical Python training trajectories for all configurations. |
| User dictionary/data preparation and title-index construction | Native CLI utilities/phrase builder and desktop maintenance | Native alternatives exist, not identical script interfaces or output-file/cache workflows. Exact derived dictionaries still require controlled comparison. |
| Python prepared/ranked caches and subprocess/worker orchestration | Bounded native SQLite caches, direct calls and Rust workers | Pickle/prepared cache import, Python subprocess/backends/batch controls and old cache formats are not ported; cache correctness is validated through native invariants, not file compatibility. |
| Legacy CLI convenience/exports | Structured JSON generation/solve plus desktop final TXT/JSON export | No request field for legacy `--subtract`, exclusion-file import, hint-stat report, generator stream/deep/full text exports or benchmark-answer console mode. Some can be emulated by callers, but those entry points are not implemented equivalents. |
| Benchmark/CI/reference/research scripts | Active Python tooling plus native probe binaries | Not all Python tooling is Rust, by design; the frozen Python oracle stays in `archive/`. No installed-app fallback is inferred. |

`archive/anagram_generate.py` explicitly subtracts `--subtract` letter chunks
without adding them as required output words, reads `--exclude-file`, and writes
`--stream-export` while generating. `GenerateRequest` exposes required/excluded
arrays and target text, but no corresponding subtraction/import/stream-output
contract. `Inventory::subtract` is an internal primitive, not an exposed
equivalent of those workflows. These are absent surface capabilities, not proof
that ordinary native exact-letter answers are incorrect.

Other deliberate differences already documented in `NATIVE_TOOLS.md` and the
native acceptance record: Rust regex does not support Python look-around or
backreferences; phrase sources require a real `ngrams` table and unique keys
rather than noncanonical views/duplicate-text modes; native solve ranks fully
required answers rather than retaining Python's zero-residual restriction.
Finite source/deployment/model limits, checked integer arithmetic, negative-row
rejection, typed errors and new status/display fields are not behavior-identical
to historical Python. These should remain named compatibility decisions rather
than silently weakening Rust checks to make malformed-input comparisons pass.

### Fresh differential evidence and failures

Eleven existing harnesses were rerun serially using current native builds,
Python 3.14.6, provisioned local data and offline Cargo dependency resolution.
Outputs/logs are in `.codex/temp/python-rust-port-review-2026-10-13/`.
`reference_fixtures.verify_sources()` also passed: pinned Python source and
reference corpus hashes match the recorded manifest. Differences below are not
explained by a silently edited oracle or drifted reference data.
The previous architecture validation's 102 native tests and 200 scoring
comparisons still apply because production source has not changed.

| Unmodified harness | Fresh result | What it does and does not establish |
| --- | --- | --- |
| `generation.py` | Pass: 500 seeded cases | Ordered enumeration, clues/repetition/caps and stop semantics on its cases. |
| `wordnet.py` | Pass: 6,483 words | Features, morphology, frames and valency on the selected word set. |
| `grammar.py` | Pass: 305 cases | **Adjusted-reference parity**, not unchanged Python equivalence: it patches three reviewed helpers in memory. |
| `refinement.py` | Pass: 160 pools | Single/refined/augmented bounded pools, not unrestricted searches. |
| `corpus.py` | Pass: 24 cases | Real-corpus short-policy admission/prefix cases. |
| `phrase.py` | Pass: 400 cases | Synthetic phrase/cohesion calculations, not the combined Wikimedia corpus matrix. |
| `learned.py` | Pass: 253 cases | Feature/model/ranking behavior; not differential trainer equivalence. |
| `ordering.py` | Fail | Real structure/valency/objective and retained-order mismatch on `actions speak louder than words`. |
| `ranking.py` | Fail | Initially additive `display_phrase` schema mismatch; omitting only that field exposes the comparative semantic mismatch on a later case. |
| `corpus_storage.py` | Fail | Its negative-count corpus expects Python's skip behavior; Rust returns an error. |
| `solve.py` | Fail | The raw comparator rejects the additive `display_phrase` field; separate control below distinguishes shared pipeline values. |

For the exact five-word ordering case, Rust returns first-order structure
`0.957`, valency `0.78`, kind `comparative`, objective `0.7871370393229093`;
frozen Python returns `0.9397056`, `0.86592`, `comparative-clause`,
`0.7898379033229093`. Both choose the named phrase first in this case, but their
remaining retained pools differ. This is a genuine semantic divergence, not
floating-point tolerance or a missing corpus explanation.

`grammar.py` names the reviewed corrections: use graded comparative evidence
for comparative-like detection, remove the legacy "any adjective/adverb"
comparative-span fallback, and prevent function words from being treated as
participial noun-phrase starts. Current Rust `clause.rs`/`phrase.rs` implements
these changes; the frozen Python files are not edited. Consequently the existing
historical statement that 118 unchanged-reference order cases pass must not be
read as current evidence without those corrections.

### Discriminating controls (not raw parity passes)

The retained project-local `diagnostic_controls.py` changes only the comparison
conditions stated here, without editing production code or the archived oracle:

- Apply exactly the same three in-memory reference adjustments as `grammar.py`:
  all **118 exact/beam ordering cases pass**. This explains the observed ordering
  mismatch as the reviewed grammar divergence for these cases; it does not prove
  no other ordering differences exist.
- For ranking, omitting **only** additive `display_phrase` passes the common
  values in its first case but still fails at case 1 on `0.957` versus `0.9397056`.
  Adding the same three reviewed-reference adjustments then passes all **12
  complete ranking/corpus-rescoring pipeline cases** with all other fields and
  numerical tolerances unchanged. This is conditional equivalence, not a raw gate.
- For corpus storage, remove its one negative `c a` row from both loaders' input.
  The remaining SQLite batching/errors/immutability and nonnegative collocation
  comparison passes. `corpus_ranking.rs::Collocation::load` rejects negatives;
  Python's positive model skips them. This confirms intentional validation
  disagreement, not a defect in valid-input counts on this fixture.
- For full solve, omitting only additive `display_phrase` passes **10 real-corpus
  CLI cases** (five requests, both strategies), including synthetic optional
  SQLite and cold/warm/rebuild comparisons for both phrase-enabled variants.
  Every shared result field is compared with the original tolerance; no grammar
  adjustment was needed for these small-cap fixtures. This validates this subset,
  not every comparative bag or every retained pool from the ranking harness.

Reproduce the controls with
`python .codex/temp/python-rust-port-review-2026-10-13/diagnostic_controls.py ordering`,
`... ranking --reviewed-grammar`, `... corpus_storage`, and `... solve`.
Ignored local diagnostics are retained evidence, not committed permanent gates.
Raw harness failures remain failures; comparison adaptations do not erase them.

### Outstanding port evidence, not automatically implementation work

1. Align permanent parity harness expectations with an explicit versioned
   compatibility policy: frozen Python, reviewed semantic corrections, additive
   native presentation/status fields and deliberately stricter malformed input.
   Preserve tests of each divergence rather than blanket dropping unknown fields
   or relaxing score/order comparisons.
2. Decide whether the absent legacy CLI conveniences are actually required in
   the product. Do not call them ported because a caller could reconstruct them.
3. Native training exists, but inference tests and training round trips do not
   establish Python trainer equivalence. If equivalence is required, compare
   grouped folds, pool membership, labels, updates, fitted weights and held-out
   metrics on frozen inputs/options before claiming it.
4. The full five-workload normal-user registry and optional real combined-corpus
   matrix were not rerun in this follow-up. Their historical passes/unfinished
   matrices retain their dates and pins. This is not all option/corpus combinations
   or a formal proof; familiar fixtures also do not establish human usefulness.

No Python/Rust implementation is changed by this completeness review. The
actionable finding is **incomplete strict compatibility and stale differential
harness policy**, not evidence that the main Rust pipeline is missing or that
the app must return to Python. Known legacy Python cache/cleft heuristics are
not declared fixed merely because their code has a Rust analogue.

## Architecture/product audit consolidation and current validation

Source: **TajsAnagrams Architecture and Product Audit**, reviewed 2 October
2026, pinned to `94cd91ff4344bd7b3f6e73231c3d7c4396d10455` (21 September).
User-provided source: `D:/TajemnikTV/Downloads/tajsanagrams-architecture-product-audit.docx`.
SHA-256: `7d4394272c3c11d3f67f50e80c1ae8fd2f936eb79ea5f114e66dea819202c7e7`.
All document-body paragraphs and its architecture table were extracted in order;
there are no embedded images, tracked insertions/deletions or comments. Footer
text is just the title/page field. This is content consolidation, not Word layout QA.

Source reread checkpoint (2026-10-13, `7902d5a`): the original DOCX is retained
byte-for-byte at `.codex/temp/tajsanagrams-architecture-product-audit.docx`, with
the same SHA-256 above. Its full body, architecture table and phased backlog
were reread against this consolidated audit; no substantive finding was missing.
The source explicitly preserves useful Python validation/parity tooling while
keeping the installed native product independent of Python. This is not a
requirement to retain every superseded legacy runner or port it again to Rust.
The finding dispositions below are the earlier `91aafc7` checkpoint; Phase C
supersedes its F-05 coverage assessment with additional continuous
ordering/ranking/storage/model/training comparisons. This reread performed no
new tests, build, runtime acceptance or external license adjudication.

Current validation pin: `91aafc7` with an initially clean worktree. The attached
audit's suggested actions and phased backlog are proposals, not instructions
executed by this review. The requested change is consolidation and validation;
no production code, corpus, deployment or CI policy is changed here.

### Finding disposition at the architecture-validation pin

| Source ID | Current disposition | Evidence-supported conclusion |
| --- | --- | --- |
| F-01, P1 distribution gate | Still open as a repository evidence gap | The corpus ledger does not establish redistribution permission for dictionary/frequency/real phrase/model artifacts. It is not a finding that private local use is unlawful. |
| F-02, P2 input resilience | Original unbounded-source finding resolved; memory claim narrowed | Finite per-role/aggregate intake budgets exist and enforce actual reads. They do not guarantee bounded parser heap or SQLite disk/page usage. |
| F-03, P2 stale ranked cache | Resolved for core source/build identity | Ranked cache includes a deterministic core content fingerprint, not just package/ranking version strings. |
| F-04, P2 negative bigram rows | Resolved | Each parsed negative integer is rejected before normalization, vocabulary filtering or duplicate aggregation. |
| F-05, P2 differential coverage | Partly addressed, still a maintenance gap | Native CI now includes scoring parity and quality tests, but most available ranking/ordering/full-solve harnesses remain outside that workflow. |
| F-06, P3 missing guide | Resolved | `NATIVE_TOOLS.md` exists; all six local file links across both READMEs resolve. |
| F-07, UI polling probe | Reproduced in actual component script with controlled IPC | An old result or error can publish after a new job starts. This is not an installed-GUI observation. |
| Multilingual boundary | Confirmed intentional contract, not an accepted feature request | English A-Z normalization is frozen; loading another dictionary does not establish language support. |

### F-01: provenance is not distribution clearance

`tests/reference/CORPORA.md` explicitly leaves the Feldman dictionary and
Norvig/Google-derived count-data permissions unverified, requires review of
real Wikimedia-derived indexes and trained models, and records WordNet notice
preservation. `PLAN.md` keeps redistribution review and wider distribution
deferred. This validates the **state of repository evidence**, not the legal
terms themselves; no new legal adjudication or external license review occurred.

The source recommendation remains a release dependency: before bundling or
mirroring datasets, enumerate exact sources/releases/hashes, reviewed terms,
attribution and notices, and check actual package contents for unapproved data.
WordNet-bearing packages need their notices preserved. No package-content
clearance test was run or inferred from local parity. The ledger is historical
and distinguishes original from derived vocabulary; it is not a substitute for
an inventory of the exact future release. Local user-provisioned use remains a
separate boundary.

### F-02: source budgets exist, but are not heap limits

`crates/anagram-core/src/policy.rs::DeploymentLimits::default` sets 2 GiB
per role and 3 GiB aggregate intake defaults; explicit null limits can disable
them for intentional local workloads. `provenance.rs::Snapshot::load_with_budget`
uses `CorpusReadBudget` readers before retaining bytes, with an overflow-probe
byte rather than trusting potentially stale metadata. Ranked solve shares one
budget across model, unigram, dictionary, bigram, WordNet and phrase intake
(`solve.rs::run`); generation has its own intake budget in `request.rs`.
Preparation paths also have budget regression tests in
`crates/anagram-cli/tests/corpus_budget.rs`.

Fresh native tests cover exact-limit/limit+1, cumulative role and aggregate
limits, growing and sparse files, cancellation during EOF/overflow, deferred
buffered errors, model format versus intake limits, WordNet/model aggregation,
canonical phrase bytes and decompressed title inputs. These are bounded synthetic
tests, not deliberate real OOM tests. `request.rs` maps intake failures to
`corpus_too_large`; the passing UI settings regression verifies that clearing a
limit saves explicit null rather than accidentally reinstating defaults.

Important residual: snapshots still materialize bytes, parsed maps can coexist,
and a multi-GiB source ceiling is not a measured safe memory envelope. Phrase
indexes budget **canonical logical JSON bytes**, not raw SQLite file size;
decompressed titles are charged by their expanded bytes. Budgets are per intake
phase, not a global process/concurrent-job heap cap. Peak parsed-state memory,
SQLite resources and suitable machine-specific defaults remain unmeasured.
The attached proposal's metadata-preflight step is not required to prove actual
read enforcement; it could be a separate optimization, not an absent correctness
guard.

### F-03: build-aware cache invalidation is implemented

`crates/anagram-core/build.rs::fingerprint` hashes logical filenames and bytes
for Rust core sources, build script and package/workspace manifests and locks
when present. It includes dirty source edits without relying on a Git SHA;
location, modification times and unrelated files are intentionally excluded.
`cache.rs::ranked_key` includes `ANAGRAM_CORE_BUILD_ID` alongside format,
engine/ranking versions, role identities and canonical request settings.

Fresh tests verify same identity/key, different identity/key, compiled versus
source fingerprint, dirty source/manifests/locks, added/deleted modules,
path/mtime reproducibility and normalized package builds. Existing cache tests
also pass input/request invalidation and bounded storage/corrupt-payload recovery.
This disproves the old same-version **core-source** mechanism. The fingerprint
is not a complete toolchain/environment/software supply-chain attestation;
unchanged source compiled with different toolchains is not claimed to invalidate
automatically. Cache format identity remains separate from algorithm identity.

### F-04: cancelling negative duplicates are rejected

`crates/anagram-core/src/scoring.rs::Bigrams::load` checks `count < 0`
immediately after successful `i64` parsing. Fresh tests in `tests/scoring.rs`
reject `10` followed by `-1`, negative out-of-vocabulary pairs and `i64::MIN`;
nonnegative duplicates aggregate and checked integer overflow still fails.
Unparseable/noninteger/NaN/infinite/out-of-range values retain the explicit
skip policy; rejecting every malformed text row was not the original acceptance
criterion and is not silently inferred. The old aggregate-only acceptance path
is gone. This run did not separately exercise Data -> Validate in the installed
UI, so its displayed error wording is not fresh runtime evidence.

### F-05: workflow coverage is improved, not comprehensive

`.github/workflows/native.yml` runs native core/CLI tests, contract generation
and parity, scoring parity, quality tests, generation, refinement and grammar
parity on Windows and Ubuntu. `.github/workflows/desktop.yml` separately covers
desktop build/checks, UI async regressions, installation rollback and host tests.
Available `tests/parity/{ordering,ranking,wordnet,corpus,corpus_storage,learned,solve,registry,phrase}.py`
are not called by native CI. Passing recorded/manual suites are not continuous
coverage, nor is a successful historical Actions run proof of current branch
protection. Remote runs and required-check enforcement were not inspected here.

Next decision remains to select a reliable, fixed-seed low-cost subset around
order/tie semantics, full solve, cache and optional evidence, with a deliberate
mutation showing each new gate detects its intended regression. Expensive
real-corpus/performance matrices can remain explicit manual/scheduled evidence.
No new required merge gate is established by this document.

### F-06: README guidance is restored

Both README links to `NATIVE_TOOLS.md` resolve. The guide explains the native
CLI and its optional data/model tools, states that desktop never launches the
CLI or needs Python, and documents root build caches. A fresh filesystem check
of all six local Markdown file links in root/desktop README found no missing
targets. It did not validate external URLs or Markdown section anchors.
The old missing-file remedy is complete, although no permanent link-check CI
step is inferred from this one-off check.

### F-07: UI probe; controlled reproduction upgrades the evidence

`apps/desktop/src/App.svelte::refresh` assigns an inactive status, then awaits
`bridge.result(job.id)` without a request revision/current-job guard. While
that promise is held, `start` can see the old inactive job, clear the result/error,
start a new job and refresh to its active status. Resolving the old promise then
publishes its result; rejecting it publishes its error from the old catch path.
`jobs.rs` permits replacement of completed snapshots; one joined worker does
not prevent this frontend response-order race.

Reproduction: `.codex/temp/audit-consolidation-2026-10-13/polling-probe.cjs`
extracts/transpiles the actual component script, stubs Svelte runtime primitives
and IPC/timers, exposes `refresh/start`, holds an old inactive result promise,
starts an active new job, then separately resolves/rejects the old promise.
Both assertions reproduced stale publication with the new job still active.
Run `node .codex/temp/audit-consolidation-2026-10-13/polling-probe.cjs` against
the retained local artifact. No copied refresh implementation or solver mock
result is treated as production-quality evidence. The harness has no DOM and
does not measure incidence in installed Tauri/Svelte scheduling.

The existing nine `tools/test-desktop-ui.cjs` tests pass: they exercise
Experimental/Management races and corpus-limit settings, **not this App path**.
Remedy proposed for a subsequent scoped fix: guard both success and failure
publication with request revision/current job, invalidate old polls on start
and disposal, and add held-response tests for old status/result/error. These
tests must demonstrate both prevention and continued new-job progress.

### Architecture, product boundaries and retained recommendations

Current `bridge.ts` invokes Tauri commands; `jobs.rs` runs/join-shuts down the
native worker. Keep the Rust core, Tauri host, Svelte/TypeScript UI, versioned
contracts and local selected data; the installed solver is not a Python wrapper,
HTTP solver or account/service client. Python remains active validation/parity
tooling and its archived solver is the comparison oracle. The native core can
still intentionally prepare/download data through separately selected tooling;
local solver independence does not mean the whole repository never uses network.

`lib.rs::normalize_letters` and its passing compatibility test retain
NFKD -> ASCII(ignore) -> lowercase -> A-Z, without transliterating Polish l-stroke
or German sharp-s. English grammar/WordNet remains the contract. Multilingual
support would require an explicitly accepted versioned alphabet, surface form,
tokenization, constraints, frequency/morphology/grammar profile and fixtures;
it is not implemented simply by changing dictionaries.

`tauri.conf.json` still disables bundling. Public signed packaging, WebView2
prerequisites/update-channel decisions, exact artifact provenance and wider
upgrade/rollback acceptance are separate release work. Linux/macOS desktop
acceptance and website hosting are not established by cross-platform core CI.
Website hosting remains deferred and would need its own isolation, admission,
rate/memory limits and operational proof. No remotely reachable critical exploit
or defect in merge ordering, cancellation ownership, cache fail-open behavior,
IPC exposure, rollback or frozen letter arithmetic was established here.

The source audit's backlog is retained by dependency rather than replayed:
input/cache/guide remedies are already implemented; regression coverage selection
and the reproduced UI publication race remain actionable; parser-memory/resource
measurement is distinct from input caps; multilingual/platform/website scope
requires separate product decisions; data rights/notices and package-content
proof remain release gates. Do not replace foundations or broaden product scope
to satisfy an outdated recommendation.

### Fresh checks and historical evidence boundaries

- `cargo test -p anagram-core -p anagram-cli --offline --locked`: **102 tests
  passed**, zero failed/ignored, including the relevant intake/cache/parser suites.
- `python tests/parity/scoring.py`: **200 seeded comparisons passed**.
- `python -m unittest discover -s tests/quality -q`: **27 tests passed**.
- `node --test tools/test-desktop-ui.cjs`: **9 tests passed**.
- `python tools/generate_contracts.py --check`: **16 artifacts match**.
- Both controlled App stale-publication variants reproduced; all six local
  README file links resolve. Raw native log, extracted DOCX and probe live in
  `.codex/temp/audit-consolidation-2026-10-13/` (ignored, project-local).

The attached audit performed no builds/tests and cites historical exact-SHA
Actions and acceptance records: 51 Rust tests, differential/52-case quality and
12-pipeline cases, five CLI workloads, Windows interactions, cold/warm timings
and a failed Python stress baseline (~10.2 GB sampled RSS). Those remain claims
about their recorded binaries, data and machine, not fresh current-pin results.
The October monthly phrase refresh with skipped benchmark steps is not performance
evidence. Older plan phase-number summaries also predate the current roadmap;
use current `PLAN.md` instead of treating historical checklist text as scope.
The separately retained quality/performance sections below identify their own
binary hashes, experiments, checks and installation proof. No new desktop build,
installation/restart, visual acceptance, remote CI success, broad quality gain
or license clearance is claimed by this Markdown-only consolidation.

## Current outcome: a validated, installed performance improvement

We found a useful improvement without changing ranking, admission, dictionaries
or generated answers: reject impossible final-word branches **before** cloning
the chosen-word vector and clue state and pushing a search frame. The existing
signature index proves that no remaining candidate at the allowed monotone
cursor can consume the residual inventory. Repeated-word and hint semantics,
candidate order, cap/exhaustion reporting and cancellation remain intact.

The implementation is in `crates/anagram-core/src/generation.rs`. This is not
quality-directed search or a new heuristic: it skips work that cannot yield an
answer under the current exact constraints.

### Paired experiment and retained limits

A preserved pre-change CLI was paired with the prototype, with randomized
case/trial order and randomized before/after order inside each pair. Requests
used desktop generation defaults, a 500-candidate cap, no solver-result cache,
and a ten-second limit. Each condition had three paired trials. OS page caching
was uncontrolled; timings include process startup and corpus loading.

| Case / strategy | Before median | After median | Median-time reduction |
| --- | ---: | ---: | ---: |
| actions_words / prefix | 5,016 ms | 2,999 ms | 40% |
| actions_words / diverse | 2,508 ms | 1,692 ms | 33% |
| quiet_machine / prefix | 1,182 ms | 873 ms | 26% |
| quiet_machine / diverse | 855 ms | 684 ms | 20% |
| small_birds / prefix | 722 ms | 578 ms | 20% |
| testing_anagram_solver / prefix | 502 ms | 469 ms | 7% |
| testing_anagram_solver / diverse | 557 ms | 464 ms | 17% |

All **21 completed pairs** had identical full semantic generation output,
including exact bag order, admission counters and exhaustion/truncation.
`small_birds` diverse timed out in all three pairs on both binaries: output
equality and completion speedup are unknown there, not vacuously successful.
The initial one-trial screen had mixed timings, so it was not used to declare
a win. These medians are observed on four cases, not a universal speed guarantee.

Raw screen/paired runs and hashes are in
`.codex/temp/quality-improvements-2026-10-13/`. Use
`two-slot-paired-verified.summary.json`, which rechecks complete output semantics
and marks uncompleted comparisons null, rather than the preliminary summary.
Baseline CLI SHA-256: `67bf25d36c04ab9a0407c0d093a36fb3879fbaec838d7cafea5a4c29cf43bed5`;
measured prototype: `553d60a3b055482ad38388bfc0710e4427939270f40026688b2517856466cc19`.
The final source was formatted and rebuilt after the prototype; its outputs were
verified independently. The committed reusable recorder is
`tools/benchmark_generation.py`; reproduce the selected experiment with:

```text
python tools/benchmark_generation.py --before .codex/temp/quality-improvements-2026-10-13/baseline-cli.exe --case-ids testing_anagram_solver actions_words small_birds quiet_machine --repeats 3 --output .codex/temp/generation-paired.jsonl
```

### Validation and local dogfood

- Full native core/CLI tests and strict all-target Clippy passed.
- Ordered generation differential checks passed **500 seeded cases** against the
  unchanged Python reference, including bounded/exhaustive, repeat/no-repeat,
  clue modes, initial clues and both strategies.
- All **19 desktop-host tests** and **27 quality tests** passed. A focused new
  regression covers same-inventory sibling words, monotone cursors, repeated
  words and exactly-one-clue semantics.
- A final desktop-profile CLI run completed six of seven cases in both cache
  states; all **12 completed full-solve result buckets** were identical to all
  three original baseline trials. `actions_words` still hit the ten-second
  experiment deadline. Exact-target Recall@1 remains 1/7, not a quality gain.
- Desktop type checking had zero errors/warnings; the frontend build reused its
  validated cache. Release build, transactional installation and restart passed.
  The installed executable matches `build/release/TajsAnagrams.exe` by SHA-256
  `BE518A713D693227A77A24AAD7211820E79F0F2F422E94B1E785C6D558DB731D`.
  One matching process was running and responding with window title TajsAnagrams.
  Installation is `%LOCALAPPDATA%/Programs/TajemnikTV/TajsAnagrams`.
- This proves build/install/launch and CLI behavior, not a measured desktop UI
  latency improvement or user visual acceptance. Generation-only timing changes
  must not be advertised as identical percentage gains in total solve latency.

### Human feedback challenges the quality assumption itself

The user explicitly confirmed that fluent sentences and coherent titles/noun
phrases can both be useful, judged individually. On the actual sanity-bag
outputs, the user judged **both “Several Migrants Tango” and “Testing Anagram
Solver” useful on their own**. These are direct, unblinded usefulness judgments,
not a relative preference and not a representative human-quality dataset.

`tests/quality/usefulness.v1.json` records these two positive labels separately.
TajsBench's opt-in `--usefulness` report counts useful/not-useful/unreviewed rows;
unreviewed answers are unknown, never invented negatives. The frozen seven-case
gold set and empty pairwise preference file remain unchanged.

Consequently, the historical 14.3% exact-target recovery does **not** mean that
only 14.3% of results are useful. The current native first result on the sanity
bag is explicitly accepted as useful. The Pareto-dominance finding below remains
valid for strictly preferring the named target, but by itself does not establish
that the solver's alternative is a usability failure or that a new language
model is needed. Further quality work should collect broader real judgments and
independent cases before promoting new features or data.

## Phase A: initial measurement corrections (historical checkpoint)

Implemented the measurement corrections and ran the stage-isolation experiments.
The evidence supports several distinct losses in gold-phrase recovery, plus an
ordering-feature representation limitation for the product sanity phrase. It
does not support switching dictionaries, adopting a language model, removing
grammar, or blindly raising search budgets as a complete remedy.

At this earlier checkpoint, production Rust/desktop behavior and corpora were
unchanged; only research tooling, tests, evaluator semantics and the roadmap
checkpoint changed. No actual human judgments had yet been collected. The seven
familiar cases were not a promotion gate or an untouched held-out set. Subsequent
production validation and direct judgments are recorded in the current section.

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
