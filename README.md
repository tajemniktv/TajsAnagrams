# TajsAnagrams

Desktop anagram solver powered by the native Rust engine, with a shared Svelte UI.
The installed application does not require Python or a separate CLI executable.

- [Desktop app and local builds](apps/desktop/README.md)
- [Native CLI and data/model tools](NATIVE_TOOLS.md)
- [Implementation plan and progress](PLAN.md)
- [Active Python oracle and compatibility tools](tests/oracle/README.md)

## Repository layout

- `crates/`: native engine, CLI and shared data/model tooling.
- `apps/`: desktop application and shared UI.
- `contracts/`: generated request/result schemas and types.
- `tools/` and `tests/parity/`: active build, contract and reference-validation tools.
- `tests/oracle/`: pinned Python reference, compatibility/benchmark tools and their unit tests.
- `archive/`: historical material only; no active pipeline imports or runs from it.

Python validation utilities remain active; the Python engine is a test oracle,
not an installed-app fallback. Existing corpora stay in `.anagram_data/`;
installed data/settings are unaffected. Removing `archive/` does not remove any
current tooling or pipeline. Native production tools are documented in `NATIVE_TOOLS.md`.

Run the oracle unit suite from `tests/oracle/` with
`python -m unittest discover -s tests -v`. Native parity scripts still run from
the repository root and explicitly import the test oracle.
