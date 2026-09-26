# openbim-ifc instructions

Purpose: Facade for the IFC crates: pick codecs and domains as features. Lib target is named ifc.

Follow `../AGENTS.md`.

## Boundary

Aggregates the ifc-* crates. Must never depend on an openbim-* standard crate.
Holds no domain logic of its own: before adding code here, ask whether it
would fit in a single domain crate. If it would, it belongs there.

Cross-domain workflows live here and nowhere else. ADR 0003 forbids sibling
domain crates from depending on each other, so a check needing two domains --
`unreachable_products` needs containment from `ifc-spatial` and representation
contexts from `ifc-geometry` -- belongs in this layer. Gate such an item on
every domain it uses (`#[cfg(all(feature = "spatial", feature = "geometry-select"))]`),
and add that feature pair to the matrix in `scripts/gate.sh`: `--all-features`
cannot see a break that only appears in one combination.

## Status

Facade implemented.
