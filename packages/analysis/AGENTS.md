# AGENTS.md — packages/analysis/

Analysis **capabilities** over IFC models. These are not openBIM standards, and
that distinction is why they live here rather than in `../openbim/`.

| Crate | Role |
| --- | --- |
| `clash` | Clash detection, on the injected geometry kernel |
| `diff` | GUID-matched semantic diff between two IFC revisions |

## Why they are not in `packages/openbim/`

`openbim/` is named for a family of published standards. Clash detection and
semantic diff are things this toolchain *does*; no standards body defines them.
Filing them under `openbim/` implies a status they do not have and erodes the
meaning of that directory.

They remain first-class: both are on `docs/ROADMAP.md`.

## Dependency direction

```
analysis → openbim-core   (to produce Outcome / ElementRef findings)
analysis → openbim-bcf    (to export findings)
```

Never the reverse. A standard must not depend on an analysis capability — BCF
is an issue format, and it neither knows nor cares that a clash engine produced
the issue.

## `clash` is the kernel boundary's stress test

It is the heaviest geometry consumer in the workspace and still takes
`axiolid-kernel` with `default-features = false`, receiving a backend by
injection. If clash can stay kernel-agnostic, everything can. Keep it that way:
if `clash` ever needs a concrete backend at compile time, the contract is
wrong, not the caller.

## Reporting discipline

Same rule as the openBIM standards: a result that treats *"could not
evaluate"* as *"passed"* is worse than no result. Use `openbim_core::Outcome`
and keep **applicable and passed**, **applicable and failed**, and **not
applicable** distinct.

## Status

Both are reserved — doc-only crates, no implementation. See `docs/ROADMAP.md`
Stages 5 and 6.
