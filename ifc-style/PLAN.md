# ifc-style implementation plan

Status: architecture scaffold; IfcPresentationAppearanceResource projections remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed presentation, layer, colour, material-appearance, and texture projections over representation items.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/assignment/styled_item.rs`: IfcStyledItem and style selects
- `src/assignment/layer.rs`: layer assignment links
- `src/colour/rgb.rs`: colour values
- `src/colour/select.rs`: colour-or-factor resolution
- `src/curve_style/style.rs`: widths/fonts/colours
- `src/surface_style/shading.rs`: shading values
- `src/surface_style/rendering.rs`: rendering/reflection values
- `src/surface_style/lighting.rs`: lighting/refraction data
- `src/texture/surface.rs`: texture descriptors
- `src/texture/coordinate.rs`: texture coordinate associations
- `src/layer/assignment.rs`: layer membership
- `src/layer/style.rs`: layer presentation

- `src/assignment/resolution.rs`: compiled private scaffold; implementation owned by `src/assignment/PLAN.md`
- `src/surface_style/refraction.rs`: compiled private scaffold; implementation owned by `src/surface_style/PLAN.md`
- `src/texture/image.rs`: compiled private scaffold; implementation owned by `src/texture/PLAN.md`
- `src/texture/map.rs`: compiled private scaffold; implementation owned by `src/texture/PLAN.md`

## Work queue

- [ ] `STYLE-ASSIGN` - implement styled-item and layer associations
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `STYLE-COLOUR` - implement colour/select semantics
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `STYLE-CURVE` - implement curve style/font/width views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `STYLE-SURFACE` - implement shading/rendering/lighting/refraction views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `STYLE-TEXTURE` - implement texture descriptors and coordinate associations
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `STYLE-CASCADE` - define deterministic occurrence/layer/style precedence
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `STYLE-CENSUS` - inventory all 70 appearance declarations and track support honestly
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
