# Native application boundaries

`anagram-core` owns puzzle rules, corpus identities, cancellation, resource policy,
generation, ranking and result-cache verification. Anagrams and Wordle have
separate domain implementations and independent correctness oracles.

`anagram-data` owns typed, cancellable dictionary/phrase preparation and ranker
data operations. It stages new outputs without overwriting existing files. The
CLI translates command arguments and owns stdin; the desktop calls the same
operations directly. CLI syntax, JSON results and error codes remain compatible.

The desktop `jobs::Inner` owns admission to one joined native worker. An active
operation has a globally increasing ID, a kind and cancellation state. Admission
and close use this owner; domain snapshots retain their own results. A stale
cancellation ID cannot stop a subsequent operation. Maintenance uses the same
worker lifecycle as puzzle work. Database review commands run on blocking tasks,
and indexed review-row/context lookups validate writes without loading all history.

`bridge.ts` owns frontend IPC. Wordle polling sends the last publication version;
unchanged polls return no snapshot or candidate array. Changed publications keep
the complete candidate pool searchable in the virtualized UI.

Corpus identities describe consumed contents, including absent optional WordNet
files and logical SQLite rows. Path, size or modification time alone never grants
a cache hit. WordNet source snapshots are identified before the ranked-cache
lookup; their parsing is deferred until a miss. Wordle reuses parsed vocabulary
by exact identity and Practice pins its vocabulary until a new game. Mutable
custom files continue to be read and hashed on analysis. Immutable managed-pack
activation is a possible later optimization, not an assumption of current code.

Play Along and Instant Solver persist versioned clue drafts in local WebView
storage. Recommendations and Practice secrets are not persisted. A restored
draft supplies its last vocabulary fingerprint; native analysis rejects changed
data until the user explicitly rechecks clues or starts a new puzzle. Selecting
a new custom source explicitly rechecks existing clues. Undo retains up to fifty
edits per mode for the current app run.

Build/install checks validate runtime artifacts. Research-view refresh is a
separate `cargo xtask review` command, or an explicit `-RefreshReview` build flag.
Research datasets and experiments are evidence, not runtime prerequisites or
automatic quality/promotion gates.
