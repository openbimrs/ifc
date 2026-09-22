# ifc-style instructions

Purpose: Borrowed presentation, layer, colour, material-appearance, and texture projections over representation items.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata; no geometry crate, renderer, image decoder, or GPU API.
- `create_planar_extent` stages `IfcPlanarExtent`, or `IfcPlanarBox` when a
  placement is given. `IfcAxis2Placement` is a SELECT, not a supertype, so
  `validate_ref`'s `is_a` check rejects both members; each concrete form is
  named instead.
- `IfcCurveStyleFontAndScaling` slot 1 is `CurveFont` in IFC4 and
  `CurveStyleFont` in IFC4X3. `schema_attribute` resolves the spelling from
  the target schema; a hardcoded name makes the writer refuse its own output
  under the other schema. (`IfcFillAreaStyle.ModelorDraughting` differs only
  in case, which attribute lookup already ignores.)

## Module ownership

- `assignment.rs`: styled-item and presentation-layer associations
- `annotation.rs`: annotation, text-literal, extent, and fill-area views
- `authoring.rs`: transaction-staged style and annotation construction
- `colour.rs`: RGB/factor/select values
- `coverage.rs`: canonical 70-declaration support census
- `curve_style.rs`: curve fonts, widths, colours
- `fill_style.rs`: fill, hatch, and tile presentation views
- `surface_style.rs`: shading/rendering/lighting/refraction data
- `text_style.rs`: text presentation style views
- `texture.rs`: surface textures and coordinate mappings
- `layer.rs`: presentation layer assignment/style
- `light.rs`: light-source and photometric light-distribution views
- `view.rs`: schema-resolved borrowed projection entry point
- `error.rs`: invalid/ambiguous presentation data

## Invariants

- Style changes appearance, never geometry shape.
- Light sources are `IfcGeometricRepresentationItem` subtypes but carry no
  shape: `ifc-geometry` classifies them `non-shape` and owns t...[truncated]
- Texture/image loading and renderer material compilation are adapter/application concerns.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
