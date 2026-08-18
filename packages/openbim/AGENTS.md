# AGENTS.md — packages/openbim/

openBIM standards built **on** the IFC layer. Everything here is a *consumer*.

| Crate | Role |
| --- | --- |
| `ids` | buildingSMART IDS: audit a model against an information spec |
| `bcf` | BCF issue read/write — how findings leave this toolchain |
| `clash` | Clash detection, on the shared geometry kernel |
| `diff` | Semantic diff between two IFC revisions |

## The one-way rule

`packages/ifc/` must never depend on anything here. If an IFC crate needs
something from `openbim/`, the abstraction is in the wrong place — move the
shared piece down into `packages/ifc/`, not the dependency up.

This is what stops the IFC core from accreting every standard that happens to
use it.

## `clash` is the boundary's stress test

It is the heaviest geometry consumer in the workspace and still takes
`geom-kernel` with `default-features = false`, receiving a backend by injection.
If clash can stay kernel-agnostic, everything can. Keep it that way.

## Reporting discipline

Applies to `ids`, `clash`, and `diff` alike: an audit that treats *"data
missing"* as *"check passed"* is worse than no audit. Results must distinguish
**applicable and passed**, **applicable and failed**, and **not applicable**.
Taken from the sibling `../vendor/solibri` engine, whose rule layer makes the
same distinction explicit and whose notes record what happens when it does not.

## Status

All four are reserved — doc-only crates, no implementation. `references/ifclite`
carries a buildingSMART IDS test corpus usable as an oracle for `ids`. See
`docs/ROADMAP.md` Stage 5.
