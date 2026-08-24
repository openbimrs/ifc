# IFC geometry plan (superseded)

Status: superseded on 2026-08-19 by progressive `ifc-geometry` plans.

The original plan was useful while the IFC-local primitive contract was being
built. The geometry package now owns a format-neutral exact `GeometryGraph`, and
`ifc-geometry` lowers into that graph rather than owning a kernel/request model.
Several stages recorded here as pending were also completed, so retaining the
old queue would misdirect future agents.

Use:

- `packages/ifc-geometry/AGENTS.md` for the stable bridge contract;
- `packages/ifc-geometry/PLAN.md` for crate-wide task order;
- paired plans under `src/input`, `src/lower`, `src/resource`, `src/curve`,
  `src/surface`, `src/solid`, `src/constraint`, `src/select`, and `src/rules` for
  bounded implementation work;
- `packages/ifc-geometry/references/ifc4-add2-tc1-geometry-declarations.tsv`
  plus its coverage test for the authoritative declaration inventory.

Do not add implementation progress here. Update the nearest owning `PLAN.md`.
