# Wordle validation

`fixtures/tests.txt` and `fixtures/LICENSE` are vendored from
[yukosgiti/wordle-tests](https://github.com/yukosgiti/wordle-tests) at commit
`f67af9257e3d46867c381ceb174118d676d8d71c`. The 76,607 rows encode answer, guess,
and feedback (`c` green, `m` yellow, `w` gray). Preserve the MIT notice.
These five-letter fixtures supplement the independent variable-length oracle
and exhaustive small-policy comparisons in the Rust core tests.

Use `cargo xtask wordle-bench --help` for native evaluation. For measured runs:

```text
cargo run --release -p xtask -- wordle-bench --samples 20 --output .codex/temp/wordle/evaluation.json
cargo run --release -p xtask -- wordle-bench --split tuning --samples 10 --endgames 8,12,20 --batches 128,256 --cache-bytes 0,8388608 --output .codex/temp/wordle/tuning.json
cargo run --release -p xtask -- wordle-bench --samples 500 --skip 510 --strategies entropy,adaptive-no-exact,adaptive --paired-heuristics --output .codex/temp/wordle/paired.json
```

Reports identify
source bytes, effective vocabularies, answer workloads, strategy, guess policy,
budgets and tuning/evaluation split. The selected vocabulary is a model of a
game, not proof of the official Wordle answer distribution. Small sampled runs
are engineering checks, not representative quality or tail-latency evidence.
`--reference` accepts an independent answer list for coverage measurement; without
one, coverage is only self-coverage of the selected pool. Exact completion rate
counts actual multi-step endgame attempts, separately from singleton/final-turn
optimal results. Per-turn records retain candidate counts, latency and search
outcomes. The 100 ms soft target permits up to twice that time for deeper adaptive
endgames, including the no-exact ablation. The seed batch yields to exact work
within the first target; an unfinished exact attempt resumes ranking through
geometrically growing batches in the remaining envelope. Two-attempt search
widens directly without a seed cutoff. Deadlines are checked between every guess,
and only fully evaluated guesses enter rankings. `--full` scores the full guess
space before optional exact search and bypasses soft targets, while the hard
deadline and state budget still apply. `--timeout-ms 0 --full` enables offline full
evaluation.

For endgame attribution, compare `adaptive` with `adaptive-no-exact`: the latter
disables only multi-step exact search, preserving coverage ordering, the overall
endgame time envelope, and the two-attempt win-probability rule. Exact search uses
part of that envelope; its ablation can use all of it for ranking.
`--paired-heuristics` memoizes identical
candidate-set/horizon decisions and shares pre-endgame decisions whose algorithms
are identical. This prevents wall-clock shortlist drift from creating apparent
exact-search improvements before exact search was eligible. Multi-step exact
results are never shared with the ablation. This freezes the sampled policy for quality
comparison; latency percentiles contain only fresh computations and are therefore
not a path-weighted latency estimate. Per-turn records identify shared/local reuse.
The report checkpoints after each condition; `complete: true` identifies the final
report. Vocabulary coverage remains relative to the supplied reference/pool.
`--skip 10` excludes the first ten hash-ordered evaluation answers per length,
used in the initial pilot; `--skip 510` also excludes the first 500-answer study.
The versioned [initial benchmark](benchmark-summary.v1.json) and
[progressive widening validation](benchmark-summary.v2.json) retain aggregate
outcomes, paired effect histograms, confidence intervals, and source/workload
fingerprints. They use different samples and scheduling policies; their aggregate
percentages are not a paired before/after effect. Raw traces remain local.
