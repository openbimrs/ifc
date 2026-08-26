# 0006 — Facade features default to thin

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

`openbim-ifc` is a facade over nineteen crates. Its default feature set decides
what a casual `cargo add openbim-ifc` compiles.

The tempting default is generous — enable the domains, the schema, geometry —
so that the crate "just works" on first use. The cost is borne invisibly by
every downstream build: a web viewer that only needs to parse and draw would
compile domain interpreters and a geometry lowering stack it never calls.

Because domain crates are optional views over a graph that round-trips
regardless ([ADR 0003](/adr/0003-domain-crates-as-borrowed-views)), a thin build
is not a degraded build. It reads and writes every entity in the file. It simply
does not *interpret* the ones it was not asked about.

## Decision

We will default to `step` only. Every domain, the schema metadata, both
alternative codecs, and the geometry bridge are opt-in features. Convenience
bundles (`codecs`, `domains`, `full`) exist for consumers who genuinely want
breadth.

We will enforce this with a test rather than a convention, because a default
feature set drifts the moment someone adds a dependency for convenience.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Enable all domains by default | Every consumer pays compile time and binary size for interpreters they never call |
| No default features at all | `cargo add openbim-ifc` then fails to do the single most common thing — read a `.ifc` file |
| Split into separate published facades | Multiplies published crates; users must choose before understanding the axes |

## Consequences

**Positive**

- A thin consumer compiles the model plus the STEP codec and nothing else.
- Feature choice maps directly onto the architecture, so the dependency graph is
  predictable from the feature list.
- Round-tripping is unaffected by feature selection, so thin builds are safe to
  use on files containing data they do not interpret.

**Negative / costs**

- Users must discover features to reach functionality; a missing feature
  presents as a missing symbol rather than a runtime error.
- Feature combinations multiply, so CI must build several explicitly rather than
  relying on `--all-features` alone.

**Follow-ups / risks to watch**

- Any future dependency added unconditionally to the facade silently fattens
  every downstream build. The thin-build test is the guard; keep it meaningful.

## Relation to existing code

- `openbim-ifc/Cargo.toml` — `default = ["step"]` with a comment recording this
  decision
- `openbim-ifc/tests/thin_build.rs` — enforcement
- `scripts/gate.sh` — builds `--no-default-features`, `--features step`,
  `--features ifcxml`, and `--all-features` with clippy denials on each
