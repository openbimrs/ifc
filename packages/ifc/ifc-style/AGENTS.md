# ifc-style instructions

Purpose: Borrowed presentation, layer, colour, material-appearance, and texture projections over representation items.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata; no geometry crate, renderer, image decoder, or GPU API.

## Module ownership

- `assignment.rs`: styled-item and presentation-layer associations
- `colour.rs`: RGB/factor/select values
- `curve_style.rs`: curve fonts, widths, colours
- `surface_style.rs`: shading/rendering/lighting/refraction data
- `texture.rs`: surface textures and coordinate mappings
- `layer.rs`: presentation layer assignment/style
- `query.rs`: style cascade/lookup
- `error.rs`: invalid/ambiguous presentation data

## Invariants

- Style changes appearance, never geometry shape.
- Representation items are referenced by EntityId; do not import geometry node types.
- Texture/image loading and renderer material compilation are adapter/application concerns.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
