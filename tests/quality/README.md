# TajsBench v1

`cases.v1.json` is the curated/gold set, grouped by exact normalized letter bag.
Splits are bag-disjoint. The first cases deliberately start with the product
sanity phrase and existing project registry phrases. The registry is migration
evidence, not itself a human-quality label. New external phrases require a
source/version/license note in each case's `provenance`.

Run `python -m unittest discover -s tests/quality` to validate the fixtures and
metric semantics. Evaluate recorded solver snapshots with:

```text
python tools/tajsbench.py --runs .codex/temp/quality-runs.jsonl --output .codex/temp/quality-report.json
```

One JSONL record is `{case_id, variant, warm, wall_ms, peak_rss_bytes,
result}` where `result` is the native `solve` JSON. An explicit `rows` array
can be used for scorer ablations or small independent experiments. Each row
needs `phrase` or `display_phrase`, or a `best_order` array. The evaluator rejects
non-anagrams. Optional `generated`, `expanded`, `deep_analyzed`,
`orders_evaluated` and `persistent_bytes` are reported, never estimated. Supply
separate records per cold/warm run and per observed latency snapshot. A result
observed at 5 seconds is **not** retroactively counted at 50 ms.

The report includes Recall@1/5/10, MRR, NDCG@5/10, cold/warm p50/p95,
available state/work/size counters, word-count and puzzle-length breakdowns,
and quality at 50/200/1000/5000 ms. Metrics with no observations are null or
absent, not zero. Compare variants over the **same case IDs and budgets**; a
bare higher average over different coverage is not a win.

`python tools/run_quality_baseline.py` records one cold solve and one disk-cache
warm solve per case with a fresh per-case cache under `.codex/temp`. Build the
native CLI release binary first (`cargo build -p anagram-cli --release`). Its
`warm` means **CLI disk-cache reuse**, not in-process desktop warm state.
Failures (including deadlines) are retained as failures and excluded from
ranking averages; report coverage alongside every average.

`preferences.v1.json` holds human pairwise judgments separately from curated
relevance. Present pairs with `make_pair(case, a, b, seed)`, record the returned
left/right and `presentation_seed`, then save the chosen side. The file is empty
until actual judgments are collected; fabricated human-preference accuracy is
not a benchmark result. Weakly supervised pairs belong in a separate dataset.
