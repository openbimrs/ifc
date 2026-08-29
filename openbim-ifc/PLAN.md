# openbim-ifc implementation plan

Status: name reserved; implementation not started.
Last updated: 2026-08-24

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Aggregates the ifc-* crates. Must never depend on an openbim-* standard crate.

## Open work

See `docs/ROADMAP.md` Stage 5 for sequencing. Nothing is claimed here yet.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- (none claimed yet)

## Work queue

- [ ] `IFF-FEAT` - keep the facade a pure aggregator; no domain logic here.
  Cross-domain orchestration is not domain logic: a check needing two sibling
  domains cannot live in either, per ADR 0003, so it belongs here. The test is
  whether the code would fit in one domain crate -- if it would, it does not
  belong in the facade.
- [x] `IFF-UNREACH` - report products no viewer will draw (issue #5).

## Completion log

Record the proof command and its result here when an item above is checked off.

### `IFF-UNREACH` - unreachable product lint

`unreachable_products()` in `src/unreachable.rs`, gated on `spatial` +
`geometry-select`. Closes issue #5 and `apps/open-signs/FINDINGS.md` F-09.

Placed in the facade rather than `ifc-author` as the issue proposed: the check
needs containment (`ifc-spatial`) and representation contexts (`ifc-geometry`),
which are siblings, and `ifc-author` depends on neither. ADR 0003 puts
cross-domain work in an orchestration layer.

Evidence:

- `cargo test -p openbim-ifc --features step,spatial,geometry-select --lib` --
  11 unit tests, covering each reported cause and each deliberate exclusion.
- `cargo test -p openbim-ifc --features step,spatial,geometry-select --test
  unreachable_corpus` -- 5 tests over the committed fixture corpus.
- Measured on `AC20-FZK-Haus.ifc`: 127 products, 20 outside the containment
  tree (17 openings via `IfcRelVoidsElement`, 3 representationless virtual
  elements), **0 findings**. A lint reporting 20 problems on a good reference
  model is one nobody runs twice.
- Differential: stripping every `IfcRelContainedInSpatialStructure` from a
  fixture turns silence into findings, and re-adding one clears it again.
- 6/6 mutation probes caught: openings no longer excused; representationless
  products flagged; plan-only geometry accepted; containers flagged; findings
  returned unordered; `geometric_products` unexported.

The corpus test initially resolved a path outside the repository and returned
`Option`, so it skipped every assertion and still reported `ok`. It now loads
committed fixtures and panics when one is missing.
