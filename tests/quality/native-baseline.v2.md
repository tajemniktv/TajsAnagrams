# Native development baseline — 12 November 2026

[Frozen summary and identities](native-baseline.v2.json).

The clean `c49c3b6` build was evaluated on all 50 exposed v4 development bags,
using default desktop-equivalent requests, a 10,000-candidate budget, 120-second
deadline, default local corpora and no optional phrase database/model. Prefix and
diverse treatments, cases and two trials were jointly shuffled. Each treatment
had a separate empty/reuse result cache. All 400 attempts and complete outputs,
including failures, are retained locally under
`.codex/docs/evidence/audit-2026-11-12`; the versioned summary records their hashes.

| Outcome per trial | Prefix | Diverse |
|---|---:|---:|
| Bags with a displayed phrase | 50/50 | 49/50 |
| Intended reference within ten of its word-count bucket | 12/50 | 12/50 |
| Bags delivering an existing owner-approved useful answer | 12/50 | 13/50 |
| Bags with unresolved usefulness | 38/50 | 36/50 |

Both trials gave identical successful outputs. Diverse's additional known-useful
bag was `multigrams-02`. `small_birds` timed out under diverse in both trials and
both cache states: four failed attempts. A failed initial computation publishes
no cache entry, so its reuse attempt recomputed rather than hitting a cache.
Prefix completed all 200 attempts. Every one of the 396 successful captures
passed the independent output verifier; all 198 successful cold/reuse pairs had
identical result buckets and verified cache hits.

| Whole CLI-process timing, including failures | Median | 95th percentile |
|---|---:|---:|
| Prefix, empty result cache | 1.695 s | 12.520 s |
| Prefix, reuse attempt | 0.202 s | 0.268 s |
| Diverse, empty result cache | 3.252 s | 27.920 s |
| Diverse, reuse attempt | 0.207 s | 0.544 s |

The mean of per-run sampled peak resident memory for completed prefix runs was
79.7 MiB with an empty result cache and 28.3 MiB on reuse; diverse was 87.0 and
29.5 MiB. Sampling occurred every 20 ms and can miss peaks. These are CLI process
measurements, not whole installed-application or WebView/IPC measurements. OS
filesystem cache and other workstation activity were uncontrolled. Cold/reuse
differences include generation/ranking reuse; they do not isolate the effect of
deferring WordNet parsing or establish a causal before/after speedup.

**Decision:** retain prefix as the production default. Diverse offers a measured
tradeoff on these exposed cases, not a demonstrated general-quality improvement.
No corpus, model, search policy or labels were promoted or changed by this run.

These 50 bags are not independent source families or untouched holdouts. Repeats
and cache states are not independent quality samples. Existing permissive owner
judgments were matched using the native evaluation tools; uncertain/conflicting
judgments abstain. Unjudged output is unresolved, not useless. Broader independent
collection and contextual judging remain necessary for the product-quality gate
described in [the baseline protocol](README.md#next-native-baseline-protocol).
