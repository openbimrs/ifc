# ifc-style implementation plan

Status: implemented typed presentation/annotation domain; rendering remains out of scope.
Last updated: 2026-08-31

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed presentation, layer, colour, material-appearance, and texture projections over representation items.

## Planned file map

These paths originated as compiled private scaffold modules and now own the
implemented domain slices. Public symbols remain exposed only through
intentional parent re-exports.

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

- `src/assignment/resolution.rs`: deterministic direct-over-layer resolution
- `src/surface_style/refraction.rs`: refraction values
- `src/texture/image.rs`: image texture descriptors
- `src/texture/map.rs`: texture mappings

## Work queue

- [x] `STYLE-ASSIGN` - implement styled-item and layer associations
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-COLOUR` - implement colour/select semantics
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-CURVE` - implement curve style/font/width views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-SURFACE` - implement shading/rendering/lighting/refraction views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-TEXTURE` - implement texture descriptors and coordinate associations
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-CASCADE` - define deterministic occurrence/layer/style precedence
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-CENSUS` - inventory all 70 appearance declarations and track support honestly
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `STYLE-ANNOTATION` - implement annotation, text literal/extent, and fill-area views and writers
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.

- `STYLE-ASSIGN` - `style_resolution` and `style_authoring` pass - direct styles win over layers; IFC2x3 wrappers stay explicit.
- `STYLE-COLOUR` - normalized RGB and colour-or-factor tests pass - malformed factors are typed errors.
- `STYLE-CURVE` - three-schema named-slot suite passes - inherited slots are never hardcoded.
- `STYLE-SURFACE` - shading/rendering/lighting/refraction views plus core transactional authoring pass.
- `STYLE-TEXTURE` - IFC2x3/IFC4/IFC4X3 texture-mode drift tests pass - URLs and transforms remain data, not loaded resources.
- `STYLE-CASCADE` - direct/layer/ambiguity tests pass - lower-priority candidates remain observable.
- `STYLE-CENSUS` - canonical IFC4 resource census asserts 70 unique declarations with explicit support tiers.
- `STYLE-ANNOTATION` - annotation, text literal/extent, and fill-area views/writers pass real STEP round-trip and dangling-reference tests.
