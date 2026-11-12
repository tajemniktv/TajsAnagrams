# Current priorities

The application uses a native Rust engine, a Tauri host and Svelte interface.
Development requires Rust, Node/pnpm and PowerShell; installed applications do
not need those development tools or Python. See [architecture](docs/ARCHITECTURE.md),
[desktop usage](apps/desktop/README.md), and the
[historical roadmap](docs/history/roadmap-2026-11-06.md).

## Dependable everyday solving

Keep search effort independent of lexical constraints. Preserve board keyboard
ownership, accessible focus, source-aware recommendations, and one visible native
operation owner. Play/Instant drafts resume with vocabulary identity checks;
Practice secrets remain native. Ordinary Settings expose appearance and data
setup, with expert and research controls behind Developer tools.

Anagram results keep word-count groups, emphasize phrases, and support copying,
pinning and word constraints. Final results remain the publication contract.
Provisional rows retained after cancellation are future work requiring a separate
result/ranking contract; they are not part of the current audit-fix scope.

## Performance and evidence

Retain content-based cache invalidation and independent correctness oracles.
Measure whole interactions, source intake, serialization and resident memory.
Defer unnecessary parsing on cache hits and reuse identified vocabulary. Do not
assume custom files are immutable. Consider managed immutable packs only after
measurements justify the extra activation/versioning machinery.

Use explicit `smoke` and `development` evaluation profiles, keeping old manifests
unchanged. The [v2 development baseline](tests/quality/native-baseline.v2.md)
freezes complete outputs and failures from a clean build, repeated empty/reuse
timings, and matched prefix/diverse treatments. Its exposed cases support
regression diagnosis, not a representative product
quality gate. The broader collection and contextual human review described in
[the baseline protocol](tests/quality/README.md#next-native-baseline-protocol)
remain necessary before claims about general usefulness or production promotion.

Evaluate existing generation alternatives under matched desktop settings before
adding another search implementation. Diagnose vocabulary, generation, shortlist,
ordering and ranking failures separately. Preserve uncertainty and keep intended
answer recovery separate from standalone usefulness.

The Phase 7/8 experimental LM and ordered-search results did not establish a
production improvement. They remain research tooling; no experimental corpus,
model or search policy is automatically installed. See the frozen evidence in
`tests/quality/phase7-experiment.v1.json` and `phase8-experiment.v1.json`.
