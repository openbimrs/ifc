# AGENTS.md — packages/openbim/

openBIM standards built **on** the IFC layer. Everything here is a *consumer*.

| Crate | Standard |
| --- | --- |
| `openbim` | Facade — features re-export the crates below, nothing more |
| `openbim-core` | Shared vocabulary: `Outcome`, `ElementRef`, `Detected` |
| `openbim-dt` | ISO 23387 data templates |
| `openbim-ids` | buildingSMART IDS |
| `openbim-bcf` | BCF (BIM Collaboration Format) — the XML container |
| `openbim-icdd` | ISO 21597 ICDD |
| `openbim-idm` | ISO 29481-3 idmXML |
| `openbim-loin` | ISO 7817-3 / EN 17412-3 LOIN |

Design rationale: `docs/adr/0015`.

## The one-way rule

`packages/ifc/` must never depend on anything here. If an IFC crate needs
something from `openbim/`, the abstraction is in the wrong place — move the
shared piece **down** into `packages/wire/` or `packages/ifc/`, never the
dependency up.

This is what stops the IFC core from accreting every standard that happens to
use it. It is also why `wire-xml` and `wire-zip` exist as a separate layer:
both `ifc-xml` and the openBIM standards need XML, and the shared piece could
not live here.

## One crate per standard — this is load-bearing

Not a stylistic choice. Cargo features are **additive across the whole
dependency graph**: if these were features of one crate, a single dependency
enabling `icdd` anywhere would make every consumer compile an RDF stack.

So: never merge two standards into one crate to "reduce boilerplate", and never
move a standard's dependency into `openbim-core` to share it. `scripts/gate.sh`
builds each crate in isolation precisely so that a violation fails the gate
rather than passing review.

## What belongs in `openbim-core`

Domain concepts used by **more than one** standard. Not XML, not ZIP — that is
`packages/wire/`.

Current members and why they are shared:

- `Outcome` — IDS *produces* results, BCF *consumes* them. One definition, or
  the two disagree.
- `ElementRef` — BCF viewpoint components and ICDD linkset endpoints are both
  "element X in document Y".
- `Detected<V>` — the one-namespace-many-versions trap, below.

If something is needed by exactly one standard, it goes in that standard's
crate. If `openbim-core` ever contains only re-exports of `wire-*`, delete it.

## 🚨 Version detection must never guess silently

Several of these standards reuse one XML namespace across incompatible
versions. IDS is the worst: **every** published version from 0.2 to 1.0
declares a byte-identical `targetNamespace`, and the differences are in
attribute *names* and cardinality rather than element names.

A reader that guesses wrong therefore **does not fail** — it silently produces a
different specification. That is why `Detected` carries how it knows and has an
explicit `Conflict` variant, and why `Detected::resolved()` returns `None` for a
conflict instead of picking one.

## Reporting discipline

An audit that treats *"data missing"* as *"check passed"* is worse than no
audit. Results must distinguish **applicable and passed**, **applicable and
failed**, and **not applicable** — that is what `Outcome` is for.

Taken from the sibling `../vendor/solibri` engine, whose rule layer makes the
same distinction explicit and whose notes record what happens when it does not.

## Tolerance is evidence-based, not a preference

Where a reader is deliberately lenient (BCF especially), the leniency is
justified by a measured corpus, and the measurement belongs in the crate docs.
`openbim-bcf` records the numbers: of 33 real third-party archives, 0 have
`project.bcfp` and 20 have no `bcf.version`. A spec-strict reader rejects every
one of them.

Do not relax a check without evidence, and do not tighten one without checking
the corpus first.

## No vendored ISO/CEN schemas

Types are written **from** the schemas; the schema files are referenced out of
tree and never committed — the same discipline `ifc-schema` applies to the
EXPRESS schemas.

Possessing a copy of an ISO schema does not establish the right to redistribute
it, and the public committee drafts of the LOIN schema carry no licence at all.
This applies to a published crate regardless of how the file was obtained.

## Status

All crates are **name reservations with no parsing**. `openbim-core` carries
real, tested types. Nothing here silently pretends to validate a model.

Working ISO 29481-3 and ISO 7817-3 codecs exist in the private `poing`
repository and are intended to move here (Phase 2). See `docs/ROADMAP.md`.
