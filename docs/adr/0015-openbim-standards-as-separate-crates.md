# 0015 — openBIM standards as separate crates behind a facade

- **Status:** Accepted
- **Date:** 2026-08-24
- **Deciders:** Friedrich, nehirde
- **Amended:** 2026-08-24 — flat `packages/`, `openbim-*` publish names, `openbim-codec-*` substrate
- **Supersedes:** —

## Context

`packages/` held four doc-only crates: `ids`, `bcf`, `clash`, `diff`.
Two problems.

**1. Two of them are not openBIM standards.** Clash detection and semantic diff
are *capabilities*. Keeping them in a directory named for a standards family
implies a status they do not have, and invites the same directory to accrete
anything IFC-adjacent.

**2. The remaining standards share a substrate that cannot live in `openbim/`.**
BCF, IDS, IDM, LOIN and ICDD are all XML; BCF, ICDD and IFCZIP are all ZIP. But
`packages/AGENTS.md` states the one-way rule: `packages/` must never
depend on `packages/`. Since `ifc-xml` needs XML handling and a future
IFCZIP needs ZIP handling, a shared substrate inside `openbim/` would force
exactly the dependency the rule forbids.

A third force appeared during design: several of these standards reuse a single
XML namespace across incompatible versions. IDS is the extreme case — every
published version from 0.2 to 1.0 declares a byte-identical `targetNamespace`,
and the differences are in attribute *names* and cardinality rather than element
names. A reader that guesses wrong does not fail; it silently produces a
different specification.

Prior art was available and inspected: `../vendor/solibri/crates/codec` (82k
LOC) solves the same problem with `container/{zip,bare,xml}` plus one module per
format, detecting containers by magic bytes rather than file extension.

## Decision

We will structure openBIM support as **one crate per standard**, plus a thin
facade, over a substrate layer that sits below both `ifc/` and `openbim/`.

```
packages/          encoding substrate, no domain knowledge
  openbim-codec-xml/             XML recognition (BOM, sniffing)
  openbim-codec-zip/             ZIP framing recognition
packages/
  openbim/              facade; features are pure re-exports
  openbim-core/         shared DOMAIN vocabulary (not XML, not ZIP)
  openbim-{dt,ids,bcf,icdd,idm,loin}/
packages/      clash, diff — capabilities, not standards
packages/         icdd, idmxml, loin — `pub use` aliases
```

Key points:

- **Separate packages, not features of one crate.** Cargo features are additive
  across the entire dependency graph. In a single crate, any dependency
  anywhere enabling `icdd` would make every consumer compile an RDF stack —
  including one that only reads `.ids` files. Separate packages make that
  structurally impossible.
- **The facade defaults to no standards.** Depending on `openbim` costs only
  `openbim-core`.
- **`loin` implies `dt`**, because the ISO 7817-3 schema imports the ISO 23387
  namespace. That is a property of the standards, not a design choice.
- **`openbim-core` holds domain vocabulary only** — `Outcome`, `ElementRef`,
  `Detected`. If it ever holds only re-exports of `wire-*`, it should be
  deleted; that would prove there was no shared domain.
- **No ISO/CEN schema is vendored.** Types are written from the schemas, which
  are referenced out of tree — the same discipline `ifc-schema` applies to the
  EXPRESS schemas.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One `openbim` crate, one feature per standard | Feature unification is graph-wide: an `icdd` feature enabled by any dependency imposes RDF on every consumer. This is the decisive argument. |
| Shared XML/ZIP inside `packages/` | Would force `packages/` to depend on `openbim/`, violating the one-way rule that keeps the IFC core from accreting every standard. |
| Put RDF in `wire-rdf` alongside `openbim-codec-xml`/`openbim-codec-zip` | ICDD is the only RDF consumer. A `wire-rdf` crate created now would be a one-consumer abstraction; defer it until ICDD is implemented. |
| Keep `clash`/`diff` under `openbim/` | They are not openBIM standards. Misfiling them is how the directory loses its meaning. |
| Delete `clash`/`diff` | Both are on the roadmap, and `clash` is the stress test for kernel-agnosticism. Moved, not deleted. |
| Short crate names (`ids`, `bcf`, `dt`) | All taken on crates.io by unrelated projects. Verified 2026-08-24. |

## Consequences

**Positive**

- A consumer needing only IDS compiles only IDS. Provable with `cargo tree`,
  and gated in `scripts/gate.sh` rather than asserted in prose.
- `packages/` can use `openbim-codec-xml` without any path to `openbim/`.
- The version-detection trap is encoded once, in `openbim_core::Detected`,
  with an explicit `Conflict` variant instead of a silent guess.
- Adding a standard is additive: a new leaf crate plus one facade feature.

**Negative / costs**

- Twelve new crates where there were four. More manifests to maintain, more
  publish steps.
- The alias crates (`icdd`, `idmxml`, `loin`) must stay pure `pub use`. If one
  ever defines a type, a graph holding both it and its canonical crate carries
  two structurally identical but non-unifiable types. They pin with `=` for the
  same reason.
- `openbim-core` risks becoming a dumping ground. The rule — used by more than
  one standard, or it belongs in the standard's own crate — must be enforced in
  review.

**Follow-ups / risks to watch**

- The LOIN namespace is **not final**: the draft schema says so in a comment,
  and an earlier draft used a different one. Namespace migration must stay a
  first-class concern in `openbim-loin`.
- `ifc-zip` (an `IFCZIP` decorator generic over `Codec`) is deferred; when it
  lands it must reuse `openbim-codec-zip` rather than reimplementing framing.
- Working ISO 29481-3 and ISO 7817-3 codecs exist in the private `poing`
  repository. Porting them here is Phase 2 and is deliberately not part of the
  first release.

## Relation to existing code

- `Cargo.toml` — workspace members gain `packages/{wire,analysis,alias}/*`.
- `packages/{ids,bcf}` → `packages/openbim-{ids,bcf}`.
- `packages/{clash,diff}` → `packages/{clash,diff}`.
- `scripts/gate.sh` — adds the openbim feature matrix and per-crate isolated
  builds that make the isolation claim executable.
- Follows the boundary discipline of `../vendor/solibri/crates/codec`, whose
  `container`/`formats` split addresses the same problem in one crate.

## Amendment, 2026-08-24 — repository split and publish names

The workspace became the `openbim` infrastructure repository
(`github.com/openbimrs/openbim`), freeing the name `nehirde` for the
application that consumes these crates. Three consequences:

**`packages/` is flat.** Grouping directories (`ifc/`, `openbim/`, `wire/`,
`analysis/`, `alias/`) are gone; every crate sits directly under `packages/`.
The layer a crate belongs to is carried by its NAME, and the architecture tests
now select on the name. This is the one genuinely load-bearing detail of the
change: three existing gates filtered crates by parent directory and would have
silently matched **zero** crates after the move — passing vacuously rather than
failing. Each was rewritten to select by name, and each was re-verified with a
mutation probe (introduce a violation, confirm the gate fails, restore).
A gate that cannot fail is worse than no gate.

**Publish names are `openbim-*`.** `ifc`, `bcf`, `ids`, `idm`, `dt`, `codec`
and `cde` are all taken on crates.io by unrelated crates. Only `icdd` and
`loin` were free, and those two are published as alias crates. The IFC facade
is published as `openbim-ifc` but keeps `ifc` as its **lib target name**, so
consumer code still reads `use ifc::…` — the ergonomic name survives even
though the registry name could not.

**`wire-*` became `openbim-codec-*`.** Same crates, same boundary, a name that
says what they are. They remain below both the IFC layer and the standards.
