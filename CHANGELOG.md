# Changelog

All notable changes to the OpenBIM.rs IFC family are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows Semantic Versioning.

## [Unreleased]

### Added

- `ifc-properties` implements six of its seven plan tasks: property sets with
  every value family, quantities, units, templates, occurrence/type precedence,
  and comparison against externally computed measurements. Property values keep
  their declared measure type, so a length stays distinguishable from a count.
  SI prefixes are carried as exact decimal exponents rather than rounded
  factors. `WR21`/`WR22` quantity breaches are reported -- neither is checked by
  `ifcopenshell.validate`. `PROP-EDIT` remains blocked on `MODEL-MUT`.

- `ifc-systems` completes its plan: flow roles and direction semantics, zones
  with their `WR1` membership rule, spatial containment vs referencing, and
  deterministic `upstream`/`downstream` queries oriented by port flow
  direction rather than by authoring order. Queries report when they crossed a
  port whose direction the file never stated, so an under-specified file
  cannot look authoritative.
- Explicit-knot polynomial/rational IFC B-spline curves and surfaces now lower
  into exact neutral Axiolid data. A synthetic IFC4 fixture proves parsed
  degrees, controls, compact knots, multiplicities, weights, and scalar-oracle
  evaluation.
- `ifc-systems` reads distribution ports and the connection network. Ports
  resolve through both `IfcRelNests` and the legacy
  `IfcRelConnectsPortToElement`, carrying flow direction and owning element.
  `ConnectionGraph` states what the file says; `NetworkGraph` adds
  through-element edges so a physical run is actually traversable. Both are
  cycle-safe, because ring mains are normal distribution topology.


- `ifc-systems` implements `SYS-ROOT`: `systems()` returns every `IfcSystem`
  and subtype with its members resolved from `IfcRelAssignsToGroup`. Systems
  are found by schema ancestry rather than `Model::ids_of_type`, which is an
  exact-type index and reports no systems at all for a file whose systems are
  all `IfcDistributionSystem`. `IfcZone` is included because IFC4 makes it an
  `IfcSystem` subtype. Memberships naming absent entities, and assignments to
  groups that are not systems, are reported as anomalies rather than dropped.
- `test/fixtures/synthetic-systems/synthetic_systems.ifc`, the first committed
  fixture stating systems at all, with its generator.

### Added

- `IfcCenterLineProfileDef` lowers to a centre-line profile: an open path plus
  the full `Thickness` across it, resolved into a constant-width boundary by
  the kernel's miter offsetting. It is read with an open-path reader rather
  than the closed-contour one, which would have invented a closing segment and
  turned a bent bar into a triangle. Corpus census 105 -> 106, and every
  concrete profile family in `IfcProfileResource` now lowers except
  `IfcArbitraryOpenProfileDef`, which encloses no area by definition.

### Fixed

- The published capability matrix is generated from the lowering source
  instead of maintained by hand, closing the ADR 0005 follow-up. It had
  drifted badly: eight shipped families were listed as planned, a warning
  claimed tessellated geometry was not lowered when both face sets are,
  `ifc-geometry` was understated by 5,472 lines, and `ifc-author` and
  `ifc-spatial` were missing from the census entirely.
- `UNLOWERED` in the profile lowerer listed twelve families that in fact have
  live dispatch arms, so the crate understated its own profile support. The
  coverage gate now rejects a family that is both dispatched and declared
  unlowered, rather than accepting either mention as coverage.

### Added

- `ifc-geometry` lowers ten profile families: the I, asymmetric I, L, T, U, C
  and Z steel sections, plus ellipse, trapezium, composite and derived
  profiles. `IfcMirroredProfileDef` lowers to a mirroring transform even
  though its `Operator` is a DERIVED attribute no file can carry. Corpus
  census 93 -> 105.
- Profile nesting is bounded: `IfcCompositeProfileDef` and
  `IfcDerivedProfileDef` reference other profiles, so a reference cycle is
  refused with a typed error instead of exhausting the stack.

### Fixed

- The schema coverage gate now enumerates `IfcProfileResource`, the fourth
  geometry schema. It previously covered three, so the 22 concrete profile
  families were never checked and coverage claims counted only what the
  fixture corpus happened to contain.
- `select::subtype` gained the 22 profile rows it was missing, so
  `is_a(.., "IFCPROFILEDEF")` resolves profile families instead of
  answering false for all of them.

### Added

- `lower/profile.rs` declares each unlowered profile family with its reason,
  and `tests/schema_coverage.rs` fails if a concrete family is neither
  lowered nor declared. 13 families are currently declared unlowered.

### Added

- `ifc-geometry` lowers `IfcExtrudedAreaSolidTapered`,
  `IfcRevolvedAreaSolidTapered`, `IfcFixedReferenceSweptAreaSolid`,
  `IfcSectionedSpine` and `IfcSweptDiskSolidPolygonal`. The polygonal disk
  carries its `FilletRadius`, which the kernel's `SweptDisk` now models.

### Fixed

- Trim parameters on an `IfcPolyline` or `IfcCompositeCurve` directrix are
  segment indices, not lengths, and are no longer scaled by the length unit.
  In a millimetre file a parameter of `2.0` became `0.002`, collapsing the
  trim onto the curve's start. This affected `IfcSweptDiskSolid` and
  `IfcTrimmedCurve`.

### Added

- `ifc-geometry` lowers `IfcRectangularPyramid`, `IfcBoundingBox`,
  `IfcGeometricSet`, `IfcGeometricCurveSet`, `IfcShellBasedSurfaceModel` and
  `IfcFaceBasedSurfaceModel`. Corpus census 82 -> 86.
- `lower/bbox.rs` recomputes the world AABB from all eight transformed
  corners. `IfcBoundingBox` is aligned to its own representation's axes, which
  are routinely rotated; passing corner and extents straight into a
  world-aligned `Aabb` claims a box the file never described.
- `lower/collection.rs` routes collection members per family. Curves and
  surfaces stay non-dispatchable as top-level items -- a bare curve must not
  stand in for a body -- but are the payload inside an `IfcGeometricSet`, so
  they route there and only there, via the generated `is_a` supertype table.
- Two corpus inventory tests: one asserts every `IMPLEMENTED` family really
  lowers, the other walks the corpus by entity type and asserts anything that
  lowers is claimed. Previously the census only visited families already named
  in the lists, so deleting a name hid its instances instead of failing.

### Fixed

- `IfcRectangularPyramid` reads `XLength, YLength, Height`. It follows
  `IfcBlock`, not `IfcRightCircularCone`, which puts `Height` first; the cone
  ordering yields a pyramid with height and width swapped that still builds.


- `ifc-geometry` lowers `IfcAdvancedBrep` and `IfcAdvancedBrepWithVoids`.
  `IfcAdvancedFace` attaches its support surface to `Face::surface`,
  `IfcEdgeCurve` attaches its support curve to `Edge::curve`, and
  `IfcEdgeLoop`/`IfcOrientedEdge` preserve edge sharing: the fixture's eight
  oriented-edge uses collapse onto four shared edges, so the manifold
  survives instead of becoming disconnected facets.
- `resource/topology.rs` gains `VertexPoint`, `EdgeCurve`, `OrientedEdge`,
  `EdgeLoop` and `FaceSurface` views. `IfcOrientedEdge.EdgeElement` is slot 2,
  after the two inherited `IfcEdge` attributes STEP writes as `*`.
- `test/fixtures/synthetic-surfaces/synthetic_advanced_brep.ifc`, generated by
  `tools/gen_surface_fixtures.py`: a half-cylinder plug with one curved
  lateral face and two planar caps.

### Fixed

- Both B-rep sense flags now compose. `IfcEdgeCurve.SameSense` sets the stored
  edge's intrinsic sense and each `IfcOrientedEdge.Orientation` flips its use
  of that edge. Applying only one leaves a curved solid whose face normals and
  edge directions disagree, which renders correctly and breaks booleans.

### Added

- `ifc-geometry` lowers the remaining exact surface families:
  `IfcCylindricalSurface`, `IfcSphericalSurface`, `IfcToroidalSurface`,
  `IfcSurfaceOfRevolution`, `IfcRectangularTrimmedSurface`,
  `IfcBSplineSurfaceWithKnots` and `IfcCurveBoundedPlane`. Radii and axis
  origins convert to metres while frame axes stay unit length, so a placement
  keeps the U/V parameterisation that trims are taken against.

- A trim parameter is scaled by the **basis surface's** quantity kind: angle on
  a revolved or conic direction, length on a planar one. A file in degrees with
  a length factor applied turns a 90-degree patch into roughly 0.0016 of one,
  and nothing downstream reports it because the surface is still valid.

- `test/fixtures/synthetic-surfaces/` — 4 generated IFC4 fixtures, plus the
  `tools/gen_surface_fixtures.py` script that produces them. No licensed public
  corpus carries curved or B-spline surfaces: `ifc-lite` (MPL-2.0) has none,
  the buildingSMART sets (CC-BY-4.0) hold only faceted breps and extrusions,
  and the one repo that does have them publishes no licence at all. Generating
  our own is the only licence-clean route, and committing the generator keeps
  the fixtures reproducible instead of opaque. All four pass
  `ifcopenshell.validate` with zero issues.

### Fixed

- `IfcSurfaceCurveSweptAreaSolid` now lowers end to end. Its `SweptCurve`
  arrives wrapped in an `IfcArbitraryOpenProfileDef`; treating that wrapper as
  a profile demanded a closed contour it cannot supply. As a swept surface's
  generatrix it is a curve, not an area, so it is unwrapped to the curve it
  names. Used as an actual profile the entity is still refused with a stated
  reason -- closing it would fabricate a face the file never described.
  Corpus census rose 80 -> 81.

### Added

- `ifc-geometry` lowers `IfcPlane` and `IfcSurfaceOfLinearExtrusion` into
  `GeometryNode::Surface` and `SurfaceRelation::LinearExtrusion`, with a new
  `Transform::to_geom_frame` that carries a placement's own U/V axes into the
  kernel frame. A surface's `x`/`y` fix its parameterisation: rebuilding them
  from the normal picks an arbitrary rotation about it, so the surface still
  renders in the same place while every trim and pcurve taken against it lands
  somewhere else. `Depth` on a linear extrusion is deliberately dropped -- the
  surface is unbounded in the extrusion parameter and `Depth` is a drawing
  hint, so folding it into the direction would silently reparameterise the
  surface.

### Changed

- `IfcArbitraryOpenProfileDef` now reports a stated reason ("the neutral
  profile model represents closed contours only") instead of a generic
  "profile subtype is not lowered yet". An open profile bounds no area;
  closing it would fabricate a face the file never described. This is the real
  blocker for `IfcSurfaceCurveSweptAreaSolid`, whose lowering is implemented
  and waiting on it.

### Fixed

- Removed duplicate direction normalization in surface lowering and
  `to_geom_frame`. `resource::direction::resolve_unit` already normalizes at
  the IFC boundary, which is where the crate's contract says it happens
  exactly once, so the second pass was unreachable code.

### Added

- `ifc-geometry` lowers exact curves: `IfcPolyline`, `IfcLine`, `IfcCircle`,
  `IfcTrimmedCurve` and `IfcCompositeCurve`. **A trim parameter is not always a
  length.** `IfcTrimmedCurve` carries values in the basis curve's own
  parameterisation, which is a length along an `IfcLine` but an *angle* on an
  `IfcCircle`. Applying the length factor to both silently rescales every arc:
  in `swept_disk_composite_arc_crankbar.ifc` the 0.082 rad arcs would become
  8.2e-5 rad on a millimetre file, and the result still renders. An
  `IfcVector`'s magnitude is likewise preserved rather than normalized away,
  because it scales the line's parameter rather than describing orientation.
- `IfcCsgSolid`, `IfcBlock`, `IfcSphere`, `IfcRightCircularCylinder`,
  `IfcRightCircularCone` and `IfcSweptDiskSolid` lower. A CSG solid is a
  wrapper and resolves to whatever its `TreeRootExpression` resolves to. CSG
  primitives are local by kernel contract, so their `Position` rides on an
  `Instance` node instead of being folded into the extents, which would
  discard the origin offset and break any rotation. A swept disk keeps its
  `InnerRadius` -- dropping it turns every pipe into a solid bar -- and refuses
  a half-open parameter range rather than guessing the missing end.
- With these families the committed corpus reaches **80 lowered items and an
  empty unsupported set**: every representation item in every committed
  fixture now lowers into the neutral DAG.

- `IfcHalfSpaceSolid`, `IfcBoxedHalfSpace`, and `IfcPolygonalBoundedHalfSpace`
  lower into `GeometryNode::HalfSpace`. A half space is the infinite cutting
  tool IFC uses to spell "clip this solid with a plane", so lowering it is what
  makes the enclosing boolean resolvable: `IFCBOOLEANCLIPPINGRESULT` left the
  unsupported set as a side effect, and the corpus census rose 67 -> 72 lowered
  items. **The `AgreementFlag` is inverted on the way through**: IFC `.T.`
  selects the side the base surface normal points *away from*, while the
  neutral `HalfSpace.agreement` selects the normal side. Passing the flag
  straight through keeps exactly the half that should have been removed, and
  nothing downstream reports it -- the boolean still evaluates and the mesh is
  still watertight, the wall simply has the wrong end missing. Curved base
  surfaces are reported as unsupported rather than flattened to a tangent
  plane, which would cut along the wrong shape. The two bounded subtypes lower
  to their underlying half space: their bounds are clipping hints, and building
  a prism from an unlowered 2D boundary curve would invent geometry.

- `ifc-geometry` lowers `IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` into
  `GeometryNode::TriMesh` and `GeometryNode::PolygonMesh`. Corpus census rose
  64 -> 67 lowered items and `IFCTRIANGULATEDFACESET` left the unsupported set.
  Authored n-gons and their voids survive verbatim: triangulating at read time
  would pick a fill rule and a tolerance on the kernel's behalf, and a face
  with its holes flattened into the outer loop tessellates into a solid slab,
  so a window silently becomes a wall. Face sets lower to meshes rather than
  `BRep` because a face set carries no adjacency -- recovering topology means
  inferring shared edges by comparing floats, which invents information the
  file never had. Two indexing traps are covered by tests because both produce
  meshes that still render: `CoordIndex` is 1-based in the file and 0-based in
  the mesh, and a `PnIndex` -- at set level or on a face -- is an extra hop
  that permutes vertices when skipped. Normals take the frame's linear part
  only; sending them through the full affine transform adds the translation
  and breaks lighting on every product away from the origin.

- `unreachable_products()` on the facade, behind `spatial` + `geometry-select`:
  reports products a viewer will never draw, with the reason. Closes #5. A file
  can pass `IfcOpenShell.validate` with zero errors and still open blank --
  validation asks whether the file is legal IFC, this asks whether the geometry
  is reachable. Three causes are detected: no
  `IfcRelContainedInSpatialStructure` (the spatial tree is how viewers reach
  geometry at all), a body authored only into a non-model context such as
  `PlanView`, and a representation whose context does not resolve. Openings,
  aggregated parts, spatial containers and representationless products are
  deliberately never reported: on `AC20-FZK-Haus.ifc` 20 of 127 products sit
  outside the containment tree and every one of them is legitimate, so the lint
  finds nothing there. It lives in the facade because containment and
  representation contexts are sibling domain crates that ADR 0003 forbids from
  depending on each other.
- `ifc-schema::ifc4()`: the IFC4 ADD2 TC1 schema (776 entities, 397 types)
  bundled as a compiled binary artifact and cached in a `OnceLock`, on by
  default via the new `ifc4` feature. Closes #4: consumers no longer source
  `IFC4.exp` themselves, hit the Latin-1 decode trap (`Schema::from_express_bytes`
  already fixed the decode half), or reparse 372 KB of EXPRESS on every process
  start -- the bundled artifact is a compiled 120 KB structural table with no
  normative source text or prose in it. `ifc-schema-generate` (the `generation`
  feature) regenerates the committed artifact from a user-supplied `IFC4.exp`;
  the normative file itself is never vendored into the crate or its published
  archive. `Schema::from_express`/`from_express_bytes` remain the path for
  schemas this crate does not bundle (IFC2x3, IFC4x3, custom).
- `product_world_transform` and `products_world_transforms` are re-exported at
  the `ifc-geometry` crate root and from the facade under `geometry-select`.
  Resolving an `IfcLocalPlacement` chain is the most-reused operation in any
  IFC consumer and the one most often reimplemented incorrectly -- composition
  order and unit scaling are both easy to invert. The batch form shares one
  placement cache, which matters because products in a storey share their
  whole ancestor chain.

- `scripts/check-leakage.py` runs in `scripts/gate.sh`. It has existed since the
  documentation work but ran only by hand, which is how a tracked
  `ifc-geometry/references/` directory survived undetected. 3/3 mutation probes
  confirm it rejects a `references/` path, XSD bytes under an innocuous
  filename, and a PDF payload.

- `ifc-geometry` splits into two build sizes. The new default-on `lowering`
  feature carries the six `axiolid-*` dependencies; turning it off leaves
  representation contexts, plan/body selection, profiles, curves, surfaces,
  solids, units and placements, which read `ifc-model` slots and link no
  geometry code. Measured on the crate's own dependency graph: 26 crates with
  lowering, 17 without -- all eight `axiolid-*` crates and `glam` drop out.
  The facade exposes the same split as `geometry-select` (selection only)
  versus `geometry` (selection plus lowering). `ifc-geometry/tests/
  kernel_free_build.rs` and two new cases in `openbim-ifc/tests/thin_build.rs`
  assert it against the resolved dependency graph, so a stray unconditional
  `use axiolid_*` fails the gate instead of silently relinking the kernel.
  Existing consumers are unaffected: `lowering` and `geometry` stay on by
  default. Closes #2.

- Opt-in recovery from damaged exports. `StepCodec::lenient()` returns a
  `StepReader` that skips unreadable data records instead of failing the file,
  and `Model::diagnostics()` reports each dropped range, so a viewer can show
  "loaded, 1 record skipped" rather than silently losing data or refusing a
  2.5 MB model over one truncated record. `StepCodec` stays strict: an
  authoring tool that drops entities corrupts the file it edits. Header
  structure and the physical-file marker remain fatal under both policies.
- `ifc_model::Diagnostic`: codec-neutral non-fatal findings carried on the
  model, with an optional source byte range.
- Advanced `openbim-step` to `0.3.2` for the recovery API.

### Changed

- `geometric_products` moved from `ifc-geometry`'s `lower` module to `input`
  and is now re-exported at the crate root. Asking which entities carry a shape
  is a slot read, so it no longer disappears with `--no-default-features`; the
  old `lower::context::geometric_products` path still resolves.
- Placement resolution moved from `lower::context` to `constraint::placement`.
  It was previously reachable only through the deep `lower` path, so it was
  undiscoverable, and after the `lowering` feature split it did not compile at
  all for kernel-free consumers -- exactly the 2D consumers that need world
  coordinates without a solid modeller. `lower::context` re-exports it, so the
  old path still resolves.

- Committed schema-derived artifacts moved from `ifc-geometry/references/` to
  `ifc-geometry/data/`, matching `ifc-template-catalog/data/`. The name
  `references/` is reserved for the local, unredistributable schema checkout,
  so a published crate must never use it. The files themselves are unchanged
  and were never a licensing problem -- they carry structural facts (slot
  indices, declaration names) and this repository's own ownership mapping, with
  no EXPRESS source text -- but the directory name defeated the detector that
  exists to catch real leaks. `data/NOTICE.md` records the reasoning. The local
  `references/` tree is now gitignored so the detector sees only the
  publishable tree.

- **Behavior change.** `select_plan_representation` now requires a drawable
  identifier *and* a plan context, instead of letting the context win outright.
  ArchiCAD authors `Box`/`BoundingBox` shape representations inside a
  `PLAN_VIEW` sub-context, so the old context-first rule returned a bounding
  box and never consulted `PLAN_IDENTIFIERS`. On `AC20-FZK-Haus.ifc` that was
  107 of 253 shape representations, and every plan lookup came back a box.
  Authorial intent now selects *between* drawable candidates rather than making
  a box drawable.

  This returns fewer answers, not just better ones: on that file, products
  resolving a plan representation drop from 121 to 34 (14 `Annotation`, 13
  `Axis`, 7 `FootPrint`, and no non-drawable picks). The 87 products that lose
  an answer genuinely have only a bounding box, and `None` is the documented
  contract for "no drawable plan geometry" -- but a consumer that was drawing
  those boxes will now draw nothing for them.

- `ifc-geometry` representation contexts: `RepresentationContext` reads
  `IfcGeometricRepresentationContext` and `IfcGeometricRepresentationSubContext`
  -- identifier, type, parent, target scale, and a typed `TargetView` that
  preserves unknown enumeration constants instead of flattening them.
  `plan_contexts` finds the sub-contexts a drawing is authored into.
- DERIVED attribute inheritance: a sub-context redeclares six inherited
  attributes, which real files write as `*` meaning "read this from my parent".
  `precision`, `world_coordinate_system`, `coordinate_space_dimension` and
  `true_north` resolve the parent chain; reading the slot directly yields the
  marker and silently loses the project's precision and placement. See ADR 0009.
- `select_plan_representation`: the inverse of `select_shape_representation`.
  Prefers an explicit `PLAN_VIEW` context, then `Plan`/`Annotation`/`FootPrint`/
  `Axis`, and returns `None` for a solid-only product rather than offering a
  body to draw flat.

- `ifc-spatial`: containment and objectified relationship traversal. Builds the
  project/site/building/storey/element tree from `IfcRelAggregates`,
  `IfcRelContainedInSpatialStructure` and `IfcRelNests`, answering "which
  elements are on this storey" and its inverse. Tolerates real exports --
  omitted levels, elements on the building, duplicate storeys, dangling
  references and containment cycles -- reporting defects through `orphans()`
  and `dangling()`. Reached through the facade's new `spatial` feature.
  See ADR 0008.
- `ifc-model::ReverseIndex`: on-demand target-to-referrer index recording the
  attribute slot each reference sits in, which is what distinguishes the two
  ends of an objectified relationship.
- `ifc-model` bounded traversal: `depth_first`, `breadth_first` and
  `find_cycle` with explicit `Budget`/`Stop` reporting, so a malformed file
  truncates with a diagnosis instead of hanging the caller.

- `ifc-author`: schema-checked entity construction. Build an entity by naming
  its attributes and let `ifc-schema` resolve STEP slot positions, instead of
  hand-placing positional values. Refuses unknown entities and attributes,
  duplicate sets, missing required attributes, declared-type and aggregate
  mismatches, and malformed GlobalIds. Reached through the facade's new
  `author` feature. See ADR 0007.

- A documentation site (VitePress) published to GitHub Pages, covering
  architecture, a conservative capability matrix, ADRs, a roadmap, and
  end-to-end use-case guides.
- Six architecture decision records covering the domain/codec-free entity
  graph, the codec trait, borrowed domain views, the Axiolid geometry
  boundary, scaffold-module semantics, and thin facade defaults.
- `openbim-ifc/tests/docs_examples.rs`, which compiles and runs every Rust
  example shown in the documentation so published code cannot drift.
- `scripts/sync-changelog.py`, generating the documentation changelog page
  from this file; `scripts/gate.sh` fails on drift.
- `scripts/check-leakage.py`, rejecting standards material (XSD, PDF,
  `references/`) from the published site.

### Fixed

- `Model::insert` no longer corrupts the type index when it replaces an
  existing entity: the id was appended unconditionally, so re-inserting listed
  it twice under the same type, and replacing an entity with one of a different
  type left it listed under both. `ids_of_type` and `type_histogram` reported
  those duplicates.

### Changed

- Advanced `openbim-step` to `0.2.1` for strict mandatory-header validation,
  line/print-control handling, low-line keywords, and lossless string escapes.
- Delegated generic ISO 10303-21 STEP syntax and ISO 10303-11 EXPRESS parsing
  to `openbim-step`; IFC retains thin model, schema-version, and validation
  adapters.
- Extracted the IFC family from `openbimrs/openbim` into its canonical standalone
  repository while preserving relevant source history.
- Added an independent Cargo workspace, CI workflow, verification gate, project
  documentation, and self-contained regression fixtures.
- Made release-critical package metadata explicit across the nested-workspace
  boundary.

[Unreleased]: https://github.com/openbimrs/ifc/commits/main
