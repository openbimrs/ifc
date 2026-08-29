# openbim-ifc instructions

Purpose: Facade for the IFC crates: pick codecs and domains as features. Lib target is named ifc.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Aggregates the ifc-* crates. Must never depend on an openbim-* standard crate.

Cross-domain workflows live here and nowhere else. ADR 0003 forbids sibling
domain crates from depending on each other, so a check needing two domains --
`unreachable_products` needs containment from `ifc-spatial` and representation
contexts from `ifc-geometry` -- belongs in this layer. Gate such an item on
every domain it uses (`#[cfg(all(feature = "spatial", feature = "geometry-select"))]`),
and add that feature pair to the matrix in `scripts/gate.sh`: `--all-features`
cannot see a break that only appears in one combination.

## Status

Facade implemented.
