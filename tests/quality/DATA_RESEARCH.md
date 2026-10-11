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
