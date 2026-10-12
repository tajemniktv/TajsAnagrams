# Desktop dependency security scope

The shipped desktop target is `x86_64-pc-windows-msvc`. `deny.toml` checks that
dependency graph, including build dependencies, against current RustSec advisories
in CI. It contains **no ignored advisory IDs**. This is not a claim that every
platform represented in Cargo.lock is free of advisories.

The workspace now requires Rust 1.90, matching Tauri 2.12's minimum. The former
1.85 minimum, combined with Cargo resolver 3's MSRV-aware selection, prevented
automatic selection of patched `serde_with` and `time`. The earlier security
update selected `serde_with 3.21.0` and `time 0.3.47`, both compatible with the
owning Tauri/cookie/plist semver ranges.
The current lockfile refreshes these packages within those ranges.
The desktop CI job builds and tests on the declared minimum toolchain.

The frontend uses Vite 8 and its matching Svelte plugin 7. TypeScript is kept on
6.0.3, the latest release supported by svelte-check's normal mode. TypeScript 7
currently requires a second TypeScript installation and opt-in native checking;
the separate contract checks already use TypeScript 7.

Reqwest 0.13 uses the renamed `rustls` feature and platform certificate validation
instead of the former `rustls-tls` feature. Default features remain disabled.

The initial advisory run also found quick-xml's duplicate-attribute CPU exhaustion
advisory (RUSTSEC-2026-0194). Updating its parent `plist` to 1.10.1 selects
`quick-xml 0.42.0`, beyond the patched 0.41.0 boundary.

Run `cargo deny --locked check advisories --warn unmaintained`. Vulnerability and
unsoundness findings remain errors. Maintenance-only advisories remain visible
warnings. Tauri 2.12 upgrades `urlpattern` and removes the five unmaintained
`unic-*` crates previously present in the Windows graph; no advisory exemptions
were added.

- [serde_with advisory](https://github.com/jonasbb/serde_with/security/advisories/GHSA-7gcf-g7xr-8hxj): the reported panic concerns KeyValueMap serialization.
- [time advisory](https://rustsec.org/advisories/RUSTSEC-2026-0009.html): the reported stack exhaustion concerns RFC2822 parsing.
- [glib advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html): the reported unsound iterator remains in the **unshipped Linux GTK graph**.

`glib 0.18.5` is constrained by GTK 0.18 and WebKit2GTK 2.0, reached through
Tauri/Wry/Tao/Muda. Adding glib 0.20 directly does not upgrade these consumers.
`cargo tree --locked --target x86_64-pc-windows-msvc -i glib` has no entries;
`cargo tree --locked --target all -i glib` shows the upstream constraint.
This remains an upstream blocker for Linux desktop support. Before enabling Linux
packaging, expand the advisory target set and resolve the GTK dependency branch;
do not just suppress the advisory. Dependabot may continue to flag the all-platform
lock graph until upstream resolves it.

The inspected Tauri use was `skip_serializing_none`, not KeyValueMap; cookie used
fixed date formats and plist RFC3339, not RFC2822. Those observations distinguish
affected dependency versions from a demonstrated application exploit, but are not
used to exempt the now-updated packages from the advisory gate.
