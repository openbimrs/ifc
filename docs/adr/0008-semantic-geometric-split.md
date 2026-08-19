# 0008 -- Semantic/geometric split: partition by pipeline role, not by IFC resource

- **Status:** Accepted
- **Date:** 2026-08-19
- **Relates to:** 0001, 0005
## Context

IFC resources mix semantic and geometric concerns by design. IfcProfileResource
holds parameterized shapes (geometry input) plus naming metadata. The material
resource attaches profiles and layer offsets to materials. The quantity
resource stores numbers that only a geometry kernel can compute. We must
decide where each lives in the crate graph.
## Decision

Partition by ROLE IN THE PIPELINE, not by IFC resource name. Three roles:

1. Geometry INPUT (anything a kernel request is built from): profile shape
   definitions, placements, representation contexts' world CS. Lives in
   ifc-geometry (lower/ for translation, views for reading).
2. Domain SEMANTICS (what a thing means to a user): material identity, layer
   names, quantity values as authored, styles. Lives in the domain crates
   (ifc-material, ifc-properties, ifc-style, ...) as borrowed views.
3. Geometry OUTPUT consumers (numbers a kernel computes that flow back into
   semantic containers): quantity takeoff. The writer lives with the domain
   crate; the number comes from the kernel via the app layer. Neither crate
   depends on the other.
Concretely:

- No ifc-profile crate. Profile SHAPE evaluation stays in
  ifc-geometry::lower::profile (it exists only to feed kernel requests).
  Profile IDENTITY (name, type enum, external refs) is semantic and may later
  get a thin view in a domain crate if a consumer needs it. The shape math is
  not duplicated there.
- IfcMaterialProfile* (5 entities) split by attribute, not by entity: the
  Profile reference and offsets/cardinal-point geometry are read by
  ifc-geometry when building section solids; material identity, priority,
  category are ifc-material views. Same entity, two readers, each reading
  only its slots. This mirrors how IfcMaterialLayerSetUsage's
  OffsetFromReferenceLine is geometry input while layer names are semantics.
- IfcQuantityResource: quantities are containers of AUTHORED numbers.
  Reading/writing them is ifc-properties. COMPUTING them is app-layer:
  app calls kernel (volume of lowered solid), then writes through the
  ifc-properties view. ifc-properties never links the kernel; ifc-geometry
  never writes quantities.
- IfcRepresentationResource: split. IfcGeometricRepresentationContext's
  WorldCoordinateSystem, Precision, TrueNorth and the shape-representation
  selection ("which representation is the Body") are geometry input and go in
  ifc-geometry::context. IfcMapConversion/IfcProjectedCRS (geodetic datum
  shift) stay in ifc-georef; they transform ABOUT the model, not within it.
  Rule: if the matrix multiplies into kernel placements, it is ifc-geometry;
  if it georeferences the finished model, ifc-georef.
- IfcPresentationAppearanceResource (70 items): zero geometry input. Styles,
  colours, textures, curve fonts affect how geometry LOOKS, never what shape
  it has. Entirely ifc-style. The one bridge, IfcStyledItem.Item pointing at
  a representation item, is a semantic edge readable from ifc-style without
  touching ifc-geometry.
## Consequences

- Crate count stays flat. New capability lands as modules inside existing
  crates until a crate earns independent existence (own consumers, own
  release cadence, own dependency needs). ifc-geometry::lower::profile can
  be promoted later without breaking callers if a structural-analysis crate
  ever needs profile math without the rest of geometry; the module boundary
  is already clean.
- The dependency rule stays enforceable by the existing no_backend_dependency
  test: only the allowlist (ifc-geometry, ifc-alignment, ifc-georef) may see
  geom-kernel, and domain crates depend only on ifc-model.
- Entities read by two crates (IfcMaterialProfile) are not owned by either;
  ifc-model owns storage, each reader owns its projection. This is the
  borrowed-view pattern from ADR 0006 applied across resource boundaries.
- Cross-cutting flows (quantity population) are explicitly APP-layer
  orchestration: read via domain view, compute via kernel, write via domain
  view. No crate-to-crate coupling is introduced for them.

## Alternatives rejected

- One crate per IFC resource (mirror the spec): couples our architecture to
  the spec's own admitted mixing; IfcMaterialProfile would force
  ifc-material to depend on profile geometry.
- A shared ifc-profile crate consumed by both sides: creates a dependency
  diamond and an ownerless dumping ground; profile shape math has exactly
  one consumer today (lowering).
- Putting quantity computation in ifc-properties: would drag geom-kernel
  into a semantic crate and break the allowlist gate.
