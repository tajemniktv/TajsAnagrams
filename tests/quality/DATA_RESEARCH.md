# Phase 6–7 source and safety record (2026-09-27)

Research inputs live only in project `.codex/temp`; no corpus or model is
bundled by this patch. The installed solver is unchanged.

| Source | Local research identity | Packaging review |
|---|---|---|
| [ESDB](https://github.com/en-wl/wordlist) | Git HEAD `1e5b7d3a72f47a71da5d28686c1dd4b397178485`; generated `scowl.db` SHA-256 `2be9a8cd6a94c40141642e8598df08d35e1b1bfc29b4b34eccaaf36716b8b7c3` | [Copyright](https://github.com/en-wl/wordlist/blob/main/Copyright) permits redistribution with notices; review tier/dialect choices and any extra notices before shipping a derived wordlist. |
| [OEWN 2025/2025+ WNDB](https://en-word.net/downloads) | downloaded official WNDB ZIPs; not packaged | [Downloads page](https://en-word.net/downloads) describes CC BY 4.0 and 2025+ proper-name distinction. Record exact ZIP hashes and full notices if selected. |
| [TUBELEX English frequency list](https://github.com/naist-nlp/tubelex/blob/main/frequencies/tubelex-en.tsv.xz) | SHA-256 `274af30d327a360e66739ccb7a1715ce4f626c3edb554308c5478d59072461f1` | Repository advertises [BSD-3-Clause](https://github.com/naist-nlp/tubelex/blob/main/LICENSE); verify the exact frequency artifact and retain notices before shipping. Subtitle text itself is not published. |
| [TUBELEX KenLM order-5 ARPA](https://huggingface.co/naist-nlp/tubelex-kenlm/tree/main) | **Not retained or used** | The host flags the English and Spanish `.arpa.xz` files **unsafe**. A download was made before this was noticed, then removed without opening it. The ARPA research scorer only accepts a separately reviewed plain-text local input; no production role is approved. |

The ESDB `core`/`extended`/`obscure` mapping currently means sizes <=60,
61–70 and 71–80 respectively. This is an experiment, not an assertion that
every word in a tier is suitable for the solver. The comparison excludes open
and hyphenated compounds, so canonical multi-token lexeme handling still needs
design and tests. A wider benchmark is needed to quantify false positives.

OEWN parser smoke data: `testing`, `anagram`, `solver`, `shakira`, `voldemort`,
`books`, and `running` had the same sampled recognized/POS/frame outcomes across
Princeton 3.1, OEWN 2025 and 2025+. This only proves the legacy parser can
load those WNDB exports, not corpus/ranking parity or name handling.

## Phase 6–7 follow-up (2026-10-15)

The pinned ESDB snapshot now supplies a fourth answer-independent treatment,
`lexical60`: names80 plus literal US/A non-abbreviation source words at size <=60.
`tests/quality/lexicon-experiment.v2.json` records 43 development bags and three
timing repeats on twelve selected follow-up bags. It closes admission gaps but
does not consistently improve intended-answer delivery: source coverage,
ordering and final ranking are separate stages. No derived corpus is packaged.

The stage evidence prioritizes a real order-language-model experiment rather
than another target-specific dictionary patch. The existing Rust ARPA research
scorer remains usable with a separately reviewed plain-text model; no historically
flagged TUBELEX archive was restored or executed.

One inspected candidate is [UD English EWT](https://universaldependencies.org/treebanks/en_ewt/),
whose web/email/review genres are relevant to informal English. Its
[publisher README](https://github.com/UniversalDependencies/UD_English-EWT/blob/master/README.md)
licenses annotations/database rights under CC BY-SA 4.0 but separately identifies
underlying text copyright holders. Consequently its headline license is not
treated here as blanket redistribution clearance for the raw texts or a derived
shipped model. It was considered, not downloaded, trained or approved for
packaging. This source/footprint review remains distinct from testing the scorer
or demonstrating an anagram-quality improvement.

## Phase 7 general-text pilot (2026-10-15)

No flagged TUBELEX archive was restored. A separate offline five-gram model was
trained by `cargo xtask research train-ngram` from four Standard Ebooks repositories:

- [Pride and Prejudice](https://github.com/standardebooks/jane-austen_pride-and-prejudice)
- [The Adventures of Sherlock Holmes](https://github.com/standardebooks/arthur-conan-doyle_the-adventures-of-sherlock-holmes)
- [The War of the Worlds](https://github.com/standardebooks/h-g-wells_the-war-of-the-worlds)
- [Alice's Adventures in Wonderland](https://github.com/standardebooks/lewis-carroll_alices-adventures-in-wonderland)

Their inspected `LICENSE.md` files describe the original texts/artwork as believed
US public domain, with contributors' work dedicated worldwide under CC0. This
records the publisher's terms, not a blanket international redistribution claim.
Original clones, licenses, exact commits, per-file hashes, paragraph extraction
and tokenization manifest remain under `.codex/temp/phase7-2026-10-15`; condensed
identities and results are versioned in `phase7-experiment.v1.json`. Only body
paragraph text was used, not cover art or upstream executable tooling. No target
file, intended answer, user note or model judgment was used for training.

The training method follows interpolated modified Kneser-Ney's continuation
counts and count-dependent discounts; see [Chen and Goodman's empirical
study](https://www.microsoft.com/en-us/research/publication/an-empirical-study-of-smoothing-techniques-for-language-modeling/).
It is a research implementation, not KenLM binary compatibility or an assertion
of equal corpus quality. The 35.96 MB ARPA remains **unbundled**: the pilot's
mixed/OOV-confounded evidence does not pass the quality/domain/footprint gate.
No model path or provider selector has been added to ordinary Settings, no
production loading/caching contract was changed, and phase/corpus attestation
ablation and managed packaging remain future work.
