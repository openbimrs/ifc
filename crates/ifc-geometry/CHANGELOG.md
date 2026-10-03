# Changelog -- ifc-geometry

All notable changes to the `ifc-geometry` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- IFC4X3 `IfcOpenCrossProfileDef` lowers exactly through
  `lower_open_profile_node` to an open polyline whose vertices are the
  closed-form sums of its widths and slopes (horizontal or along-slope
  widths, `OffsetPoint` honoured, slopes measured from +X towards +Y as the
  IFC4.3 figure states). `describe_profile` reads it as the new
  `ProfileParameters::OpenCross`, in metres and radians, refusing a
  `Widths`/`Slopes` or `Tags` count mismatch, a negative width and a vertical
  slope with horizontal widths. It bounds no area, so the area-profile path
  refuses it, as for `IfcArbitraryOpenProfileDef` (#243).
- IFC4X3 `IfcDirectrixDerivedReferenceSweptAreaSolid` lowers to the same
  exact `FixedReferenceSweep` as its supertype when the directrix defines only
  a tangent, which IFC4.3 says is the identical behaviour. A directrix that
  defines a tangent plane (built from `IfcCurveSegment`s, such as
  `IfcGradientCurve` and `IfcSegmentedReferenceCurve`, or lying on a surface)
  is refused as `Unsupported` naming the missing neutral primitive, the
  public `lower::swept::DIRECTRIX_DERIVED_TANGENT_PLANE` (#243).
- IFC4X3 `IfcSectionedSolidHorizontal` and `IfcSectionedSurface` are refused
  as `Unsupported` with a named reason instead of the generic "not lowered
  yet": their sections stand at `IfcAxis2PlacementLinear` stations along the
  directrix, and Axiolid has no sectioned sweep over curve-measure stations
  nor any sectioned-surface relation (#243).
- `section_slot::OC_*` slot constants for `IfcOpenCrossProfileDef`.

## [0.6.0] - 2026-10-02

### Changed (breaking)

- Requires `ifc-alignment` 0.5. `derive_placement_transform` takes an
  `ifc_alignment::PointByDistance`, so the alignment version is part of
  this crate's public API; mixing it with `ifc-alignment` 0.4 types no
  longer compiles. No behaviour changes in this crate.

## [0.5.0] - 2026-09-29

### Changed

- `MaterialProfileSetUsageGeometry::new` accepts
  `IfcMaterialProfileSetUsageTapering`, the schema subtype of
  `IfcMaterialProfileSetUsage`, whose inherited slots it reads unchanged;
  the new `MaterialProfileSetUsageGeometry::tapering()` returns its
  `MaterialProfileSetUsageTaperingGeometry` (end profile set and end
  cardinal point), or `None` for a plain usage (#136).

### Changed (breaking)

- `ViolationKind`, `Support` and `FunctionStatus` are `#[non_exhaustive]`: a
  match needs a wildcard arm.
- `RuleViolation`, `LoweredGeometry` and `MappedInstance` are
  `#[non_exhaustive]`; they can no longer be built with a struct literal
  outside the crate.
- `authoring::surface_curve_swept_area_solid`,
  `authoring::fixed_reference_swept_area_solid` and
  `authoring::swept_disk_solid` are removed (#210). Without the model they
  could not write every release correctly: the two directrix sweeps wrote
  the IFC4X3 `IFCPARAMETERVALUE(..)` trim into IFC4 files, and the swept
  disk wrote `$` for the trim IFC2X3 requires. Use
  `surface_curve_swept_area_solid_in`, `fixed_reference_swept_area_solid_in`
  and the new `swept_disk_solid_in`, which take `&Model` after the
  transaction and otherwise the same arguments.
- `SurfaceCurveSweptAreaSolid::start_param`/`end_param` and
  `FixedReferenceSweptAreaSolid::start_param`/`end_param` return
  `GeometryResult<Option<TrimMeasure>>` instead of `Option<f64>` (#210). In
  IFC4X3 the trim is an `IfcCurveMeasureSelect`, and an
  `IFCLENGTHMEASURE(..)` trim is a distance along the directrix, not a curve
  parameter; the reader now says which (`TrimMeasure::Parameter` or
  `TrimMeasure::Length`) instead of returning both as a parameter. A bare
  number is a parameter, as IFC2X3 and IFC4 declare. A typed value that is
  not one of the SELECT's members is refused with `WrongValueKind` instead
  of being unwrapped.
- Lowering an `IfcSurfaceCurveSweptAreaSolid` or
  `IfcFixedReferenceSweptAreaSolid` whose trim is an `IfcLengthMeasure`
  fails with `Unsupported` (#210). It used to pass the length on as a curve
  parameter, which on a conic directrix reads metres as radians; converting
  a length into the directrix's parameter needs arc-length evaluation,
  which lowering does not do.

### Added

- `authoring::swept_disk_solid_in` (#210): an `IfcSweptDiskSolid` in the
  model's declared release. The trim is written bare in every release, as
  before; in IFC2X3, which declares `StartParam` and `EndParam` required,
  an unset one is refused with `InvalidAuthoredValue` and nothing is
  staged.
- `solid::swept::TrimMeasure` (re-exported from `solid`), the kind and
  value of a directrix sweep's trim, and `TrimMeasure::parameter`.

### Fixed

- `authoring::curve_segment` writes a `CurveMeasure::Length` as
  `IFCLENGTHMEASURE(..)` (#210). It wrote `IFCNONNEGATIVELENGTHMEASURE(..)`,
  which is not a member of IFC4X3 `IfcCurveMeasureSelect =
  SELECT (IfcLengthMeasure, IfcParameterValue)` and is an `ifc-validate`
  error. Because `IfcLengthMeasure` is signed and neither slot is bounded,
  a negative length is now written as given instead of refused.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.4.4] - 2026-09-28

### Added

- `authoring::surface_curve_swept_area_solid_in` and
  `authoring::fixed_reference_swept_area_solid_in` (#200). They take the
  model and write `StartParam`/`EndParam` in the form its declared release
  requires: bare in IFC2X3 and IFC4, where the slot is `IfcParameterValue`,
  and `IFCPARAMETERVALUE(..)` in IFC4X3, where it is the SELECT
  `IfcCurveMeasureSelect`. The release binds from `FILE_SCHEMA` as the
  other authoring crates bind it (none binds IFC4). An attribute the release
  requires left unset (IFC2X3 `Position`, `StartParam`, `EndParam`) is
  refused with `InvalidAuthoredValue`.
- `GeometryError::AuthoringSchemaUnbound` (an unknown or ambiguous
  `FILE_SCHEMA`) and `GeometryError::AuthoringEntityNotInSchema` (the
  release does not declare the entity, such as the fixed-reference sweep in
  IFC2X3), for those writers. `GeometryError` is `#[non_exhaustive]`, so
  this is not breaking.
- `authoring::grid_with_owner_history` (#202): an `IfcGrid` in the model's
  declared release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
  requires on every `IfcRoot`. It binds the release as the `_in` writers
  above do, and lays the record out by attribute name from its table, so
  an IFC2X3 grid has its 10 attributes, not IFC4's 11. A `predefined_type`
  in IFC2X3, which declares none, or outside the release's
  `IfcGridTypeEnum`, is refused with `InvalidAuthoredValue`. The owner
  history must be in the model or staged on the transaction and be an
  `IfcOwnerHistory` (`InvalidAuthoredValue` on `OwnerHistory` otherwise);
  none is ever invented. IFC4 and IFC4X3 records are `grid`'s with the
  owner history in its optional slot. No error variant is added.

### Fixed

- `IfcParameterValue` slots are written bare (#200):
  `rectangular_trimmed_surface` (`U1`, `V1`, `U2`, `V2`), `point_on_curve`,
  `point_on_surface`, `reparametrised_composite_curve_segment`
  (`ParamLength`), and the `StartParam`/`EndParam` of `swept_disk_solid` and
  `swept_disk_solid_polygonal`. Each is declared with the defined type
  `IfcParameterValue`, not a SELECT, in every release that declares it, and
  ISO 10303-21 writes a typed parameter only for a SELECT. The readers
  accept both forms, as before.

### Changed

- `surface_curve_swept_area_solid` and `fixed_reference_swept_area_solid`
  still write `IFCPARAMETERVALUE(..)`, which is correct in IFC4X3 only. They
  cannot see the release; their docs now say so and point IFC4 (and IFC2X3)
  callers to the `_in` writers.
- `authoring::grid` is unchanged and documents its limitation (#202): it
  takes no model, so it writes the IFC4 layout (11 attributes,
  `OwnerHistory` `$`), which is never valid IFC2X3. It moved from
  `authoring/transform.rs` to `authoring/grid.rs`; the public path is the
  same.

## [0.4.3] - 2026-09-28

### Added

- `BodyItem::item_world`: the frame each described item is placed in, the
  context and product placement composed with every `MappingTarget o
  MappingOrigin` it was reached through (#185). It is reported for every item
  kind, including mapped B-reps and tessellations, and is not required to be
  rigid, so a mapping that mirrors or scales shows. `BodyItem::is_mirrored()`
  answers whether that frame reverses handedness, and
  `Transform::determinant()` gives its signed volume scale. Additive:
  `BodyItem` is `#[non_exhaustive]`.

## [0.4.2] - 2026-09-27

### Changed

- `data/ifc4-where-rules.tsv` lists each geometry WHERE rule by entity,
  label and support state only. Its `expression` column held the rule bodies
  verbatim, which are CC BY-ND schema text and are no longer shipped.
  `data/NOTICE.md` covers all five data files and no longer claims the
  directory holds no rule bodies while it did.

## [0.4.1] - 2026-09-27

### Fixed

- An `IfcCurveBoundedPlane` lowered under a non-identity frame no longer
  moves its boundaries twice (#163). `OuterBoundary` and `InnerBoundaries`
  are in the basis plane's parameter space, which is also how the neutral
  curve-bounded relation reads them, but the frame was applied to them as
  well as to the plane: they landed elsewhere or were refused as off the
  plane. Only the basis plane takes the frame now, so a framed plane meshes
  to the identity mesh moved by the frame. Connection surfaces lowered under
  a space's frame are the case this breaks.

### Added

- `product_representation_frame(model, units, product, purpose)`: the frame
  a product's representation of that purpose is placed in, the context's
  `WorldCoordinateSystem` composed above the placement chain (#164). Lowering
  and `body_description` now take their frame from it, so geometry lowered
  outside the body, such as a space boundary's connection surface in the
  relating space's coordinates, is placed exactly as the body is.
  `Ok(None)` when the product has no such representation; kernel-free.
- `profile_outline(model, units, profile)` and `ProfileOutline` (#166): an
  `IfcArbitraryClosedProfileDef`'s or `IfcArbitraryProfileDefWithVoids`'s
  boundaries as rings of vertices in metres, in profile coordinates, for
  `IfcPolyline` and line-only `IfcIndexedPolyCurve` boundaries. Each ring
  is in authored order without its closing vertex, as profile lowering
  reads it. An `IfcArcIndex` segment or any other curve family is
  `Unsupported` naming the curve, never chorded; a 3D point, fewer than
  three distinct vertices and non-consecutive segments are `Degenerate`.
  Kernel-free.

### Changed

- `lower_product_representation` selects the representation before it
  resolves the placement, as `body_description` already did: a product with
  no representation of the purpose is `Ok(None)` even when its placement is
  broken, where it used to be the placement error.

## [0.4.0] - 2026-09-27

### Fixed

- A face surface whose `FaceSurface` is dangling now reports the face as the
  referrer, and one naming a non-surface is `WrongEntityType` naming the
  target (#155). The first reported the missing id against itself; the
  second was an `Unsupported` "curved and B-spline surfaces", which reads as
  valid IFC this bridge declines rather than a broken reference. Applies to
  faces inside advanced B-reps too.
- Compiled `IfcBlock` meshes were offset by half their extents. `IfcBlock`
  has a corner at its `Position` (IFC4 ADD2 TC1), but the neutral
  `Primitive::Block` is tessellated centred on its origin by
  `axiolid-reference`, so every compiled block sat half its size away from
  where the file placed it, along its own axes. Lowering now puts the
  half-extent shift on the block's `Instance`. This changes compiled
  geometry for every `IfcBlock`; the lowered `Instance` translation now
  names the block's centre. The other CSG primitives already agreed.
- `Plane`, `CylindricalSurface`, `SphericalSurface` and `ToroidalSurface`
  `::position(&model)` now type-check their target through
  `resource::resolve` (#135). A `Position` naming anything other than an
  `IfcAxis2Placement3D` is `WrongEntityType` naming the target; it used to
  be wrapped as a 3D placement and misread.
- A derived linear placement no longer reports an evaluator's degenerate
  curve as an undefined roll. The refusal now distinguishes an unsupported
  curve family, a rejected measure (off the curve, or roll undefined
  because the tangent is parallel to the up reference) and a degenerate
  curve, matching how `axiolid-evaluate` 0.3 reports them.

- An opening that a file makes void two hosts is subtracted from the first
  only (#59). `IfcFeatureElementSubtraction.VoidsElements` is a
  single-valued inverse in IFC2X3 and IFC4, but every `IfcRelVoidsElement`
  was applied, so the second host's net body was cut by an opening that
  belongs to another element. The relation with the lower id now wins in
  `openings_of` and in net compilation. Output changes only for files that
  violate the schema.
- `IfcParameterizedProfileDef.Position` now reaches the kernel for every
  parameterised family (#147). It was applied to rectangles and circles
  only, so an I, L, T, U, C or Z section, an ellipse or a trapezium with an
  offset or rotated `Position` lowered at the profile origin. Output changes
  only for files that author a non-identity `Position` on those families.
- The translation of an `IfcDerivedProfileDef` operator is converted to
  metres (#147). It was passed through in file units, so a derived profile
  offset by 50 mm in a millimetre file lowered 50 m away.
- An `IfcAsymmetricIShapeProfileDef` in an IFC2X3 file no longer reads
  `CentreOfGravityInY` (slot 11 in that schema) as `BottomFlangeEdgeRadius`
  (#147). IFC2X3 declares the entity as an `IfcIShapeProfileDef` subtype
  with a different tail; the declared schema now selects the layout, and
  the IFC4-only edge radii and slopes are absent in IFC2X3.

### Added

- `IfcFaceSurface` and `IfcAdvancedFace` lower as representation items
  (#155). Both are members of `IfcSurfaceOrFaceSurface`, the type of a
  connection surface, but the dispatcher refused them. The new
  `lower_face_surface_node` builds ONE face through the B-rep face path --
  exact carrier surface, `SameSense` as the face's orientation relative to
  that carrier, each bound's `Orientation` -- inside one open shell with no
  solid, so a single face never acquires a volume. They move from the
  nested-only disposition ledger to `IMPLEMENTED`, and `BodyKind::Face`
  describes them. Committed evidence:
  `test/fixtures/synthetic-surfaces/synthetic_space_boundary_face_surface.ifc`.
- `lower_connection_surface(session, connection, frame)` and
  `lower_related_connection_surface` lower an
  `IfcConnectionSurfaceGeometry`'s `SurfaceOnRelatingElement` and optional
  `SurfaceOnRelatedElement` (#155), dispatching `IfcSurface`,
  `IfcFaceSurface`/`IfcAdvancedFace` and `IfcFaceBasedSurfaceModel`. Each
  end is authored in its own element's coordinate system, so each takes its
  own frame. Point, eccentric point, curve, volume and (IFC2X3) port
  connections are typed `Unsupported` refusals naming the connection; any
  other entity is `WrongEntityType`.
- `body_description(model, units, product)` reports how a product's Body
  representation is modelled (#147). It returns one `BodyItem` per
  geometric item in authored order, with mapped items resolved (`mapped_by`
  names the chain), each carrying a `BodyKind` (extrusion, tapered
  extrusion, revolution, tapered revolution, directrix sweep, swept disk,
  sectioned spine, B-rep, CSG, CSG primitive, half space, bounding box,
  tessellated, surface model, geometric set, curve, surface, point) and, for
  the swept-area families, a `SweptSolid`: the profile description, the
  end profile of a tapered sweep, the solid's placement in world
  coordinates and a `SweepPath` (extrusion direction as a world unit vector
  plus depth in metres, revolution axis and angle in radians, or the
  directrix curve). Mapped geometry describes identically to the same
  geometry authored in place. Anything that cannot be stated exactly is a
  typed error for the whole body, never a partial list: unsupported item or
  profile families, dangling references, mapping cycles, open profiles
  swept as areas, and a mapping that scales or mirrors a swept solid. It is
  kernel-free and reachable with `--no-default-features`.
- `describe_profile(model, units, profile)` reads any concrete
  `IfcProfileDef` into a `ProfileDescription`: type, `ProfileName`,
  `Position`, and `ProfileParameters` for every family in metres and
  radians (rectangle, rounded and hollow rectangle, circle and hollow
  circle, ellipse, I, asymmetric I, L, T, U, C, Z, trapezium, arbitrary
  closed, with voids and open by curve reference, centre line, composite,
  derived with its operator, mirrored). A bare `IfcProfileDef` and unknown
  families are refused; a composite or derived chain that references itself
  is `CyclicChain`.
- `derive_placement_transform` derives a linear placement on an
  `IfcPolyline` or a line-only `IfcIndexedPolyCurve` basis curve (#96), not
  only on an alignment. The curve lowers to the neutral polyline, whose arc
  length is an exact finite sum, so a distance converts to a parameter
  exactly; a native parameter follows the IFC polyline parameterisation
  (one per segment). Refused by name: a zero-length segment, fewer than two
  points, non-consecutive `Segments`, an `IfcArcIndex`, a parameter on a
  multi-point `IfcLineIndex` (IFC does not state its split), and ellipse
  and B-spline bases as before.
- `product_bounds` bounds linear extrusions of straight-edged profiles
  (rectangles, polyline contours, and placed/derived forms of them) and
  blocks exactly from their vertices, without tessellating (#98). The
  result reports `BoundsSource::Exact`. Curved profiles, rounded
  rectangles and booleans still go through the compiled mesh; a
  difference only shrinks its operand, so its operand's box is never used.
- `voiding_conflicts(model)` and `VoidingConflict { opening, kept_host,
  rejected_host, relation }` report such openings (#59). It is kernel-free,
  like `openings_of`. Restating the same host is not a conflict.
- Resolving typed accessors beside the raw `*_ref` getters (#97), following
  `Plane::position(&model)`: `Line::point`, `Polyline::points`,
  `IndexedPolyCurve::points`, `BSplineCurve::control_points`,
  `OffsetCurve3D::ref_direction`, `Circle::position`, `Ellipse::position`,
  `Trim::cartesian_point`, `BSplineSurface::control_point_views`,
  `SurfaceOfLinearExtrusion::{position, extruded_direction}`,
  `SurfaceOfRevolution::{position, axis_position}`,
  `BoundingBox::corner_point` and `TessellatedFaceSet::coordinate_list`.
  Each type-checks its target: a dangling reference is `MissingEntity`
  naming the referrer, a wrong type is `WrongEntityType` naming the target.
  All are kernel-free.
- `resource::resolve`, the shared type-checked resolvers behind them, and
  two select views: `Axis2Placement` (2D or 3D, for `IfcConic.Position`) and
  `CartesianPointList` (2D or 3D, for `IfcIndexedPolyCurve.Points`).
  Profiles stay references; `describe_profile` owns `IfcProfileDef` reading.

- `compile::product_bounds` / `product_bounds_with` return a product's
  world-space axis-aligned bounding box (#36). The body is resolved and placed
  the same way as for `compile_product_mesh`, and the result is `Ok(None)` in
  the same case (no body).
  - When every leaf of the lowered graph is a mesh or an authored bounding
    box, the box is read off the exact graph without tessellating
    (`BoundsSource::Exact`).
  - Otherwise it comes from the compiled mesh (`BoundsSource::Tessellated`).
    That box is exact for planar geometry and can fall short of a curved
    surface by up to the tolerance.
  - A body with no finite extent is `GeometryError::Degenerate`, never an
    empty box.
- `examples/product_bvh.rs` indexes every product of a file in
  `axiolid_spatial::Bvh` and prints the broad-phase overlaps.
  `axiolid-spatial` is a dev-dependency only: the index stays Axiolid's, and
  this crate only produces the boxes.

### Changed

- `lower::profile` builds every profile from `describe_profile` instead of
  reading slots itself (#147), so lowering and body description cannot
  disagree about a slot, a unit or a default. A profile nesting chain that
  exceeds its budget is now `ChainTooDeep` rather than `Unsupported`, and a
  self-referencing chain is `CyclicChain`; a dangling boundary curve of an
  arbitrary profile is reported when the profile is read.
- Requires `axiolid-mesh-compile` 0.3.4, `axiolid-construct` 0.3.3 and
  `axiolid-evaluate` 0.3.1. Compiled output changes where the kernel's did:
  a B-rep's void shells are tessellated facing into the cavity instead of
  being dropped, so an authored cavity is no longer filled
  (axiolid/kernel#120); every solid of a multi-solid B-rep is meshed, not
  only the first (axiolid/kernel#111); and a curve-bounded plane, the usual
  space-boundary connection surface, compiles to a planar surface mesh
  instead of being refused (axiolid/kernel#192).
- Requires `axiolid-mesh-compile` 0.3.3 and `axiolid-contracts` 0.3.1.
  Closed `IfcPolygonalFaceSet` bodies whose face corners lie on a straight
  run (collinear notch and window heads) now mesh closed and report `Solid`
  (axiolid/kernel#170); before, the triangulation left T-junction cracks and
  they came back `Surface`. Surface models with a zero-area bowtie face, as
  Nova MEP exports write pipe-fitting end caps, now compile instead of being
  refused (axiolid/kernel#171). On 12 real models (66,659 products) this
  moves 1,999 products to `Solid` and failures from 2,291 to 755, together
  with the kernel#168 and #169 fixes already required.
- `tests/meshing_coverage.rs` pins both: a real ArchiCAD lining at its
  exact coordinates meshes to its divergence volume as a solid, and a
  surface model with a bowtie cap keeps its area. A third test checks that
  an explicit chord budget (`ExecutionOptions::with_chord_error`,
  axiolid/kernel#165) brings the composite-curve D within 1e-5 of its exact
  volume.

## [0.3.1] - 2026-09-25

### Added

- `compile::compile_product_mesh_reported` (and `_with`) return a
  `CompiledMesh`: the triangles plus the kernel's `MeshClosure`, i.e. whether
  they bound a solid. `CompiledMesh::solid_mesh(product)` returns the mesh
  only for `Solid` and otherwise refuses with the new
  `GeometryError::NotASolid`, naming the product. An
  `IfcShellBasedSurfaceModel` now compiles (axiolid/kernel#161) but is a
  `Surface`: without this a caller summing the divergence of its triangles
  gets a volume the file never claimed, finite and plausible for a closed
  shell. A backend that does not report closure gives `Unknown`, which is
  refused as well. `NetMesh::closure` carries the flag for net bodies.
  `compile_product_mesh` is unchanged and still returns the bare mesh.
- `tests/meshing_coverage.rs` and the generated public fixture
  `test/fixtures/synthetic-coverage/meshing_coverage.ifc` (#47): one product
  for each product-meshing failure kind #47 measured on real models, compiled
  through `compile_product_mesh`. The four kinds fixed here (#43 to #46) pin
  exact volumes, cross-checked with IfcOpenShell 0.8.5. The two fixed in the
  Axiolid reference compiler (`axiolid-mesh-compile` 0.3.1) pin their answer
  too: polygonal faces with more than 3 corners (axiolid/kernel#160) mesh the
  2 m quad cube to 8 m3, and the open-shell surface model
  (axiolid/kernel#161) meshes its 0.12 m2 and refuses any volume.
- `DegenerateFacePolicy` (#46), set per session with
  `LoweringSession::with_face_policy`. The default, `Refuse`, is unchanged:
  an `IfcPolyLoop` with fewer than three distinct edges refuses the brep,
  naming the loop. `DropAndReport` leaves out a face whose outer (or only)
  bound collapses, since it covers no area, and lists it in
  `ProvenanceMap::dropped_faces`. A collapsed hole in a face with real area
  is still refused, and a shell whose every face collapses is refused as
  `Degenerate`. The policy drops exactly what the default refuses.
- On OfficeBuilding.ifc all 16 `IfcWindow`s refused this way (two shared
  loops of the form `(A, A, B, B)`) compile under `DropAndReport`, each
  reporting its one dropped face; their volume, 0.1707752 m3, matches
  IfcOpenShell 0.8.5. No other product in eight real models changes.
- `IfcArbitraryClosedProfileDef` and `IfcArbitraryProfileDefWithVoids` now
  lower an `IfcCompositeCurve` outer or inner boundary (#43). Segments may be
  `IfcPolyline`, `IfcTrimmedCurve` over `IfcCircle` or `IfcLine`, or a nested
  `IfcCompositeCurve`. Arcs stay exact `Circle2` segments. `SameSense`,
  `SenseAgreement` and `MasterRepresentation` are honoured, and a trim may be a
  parameter (in the project's plane-angle unit) or a cartesian point.
- On the two real models that carried them, all 128 products refused for a
  composite profile boundary now compile, and no other product changed.
- Net geometry (#44, ADR 0014): `compile::compile_product_mesh_net` (and
  `_with` for your own backend) returns a product's Body with every
  `IfcRelVoidsElement` opening subtracted, plus the ids of the openings
  removed. `compile_product_mesh` is unchanged and stays gross: quantity
  takeoff wants gross, clearance and ratio checks want net. The graph-level
  entry point is `lower::lower_product_net`; the kernel-free relation reader is
  `openings_of`.
- An opening that cannot be removed -- its Body does not lower, it has none,
  it is not a solid, or the backend refuses it or its cut -- is
  `GeometryError::OpeningNotSubtracted` naming the host and the opening. The
  gross body is never returned in its place. A host whose own Body the backend
  refuses stays `CompilationRefused` on the host.
- Multi-item hosts and openings (a Body with several items, or a mapped item)
  are cut item by item: every host part minus every opening part. A mapped
  opening is flattened through its instance transforms, composed outer after
  inner.

### Changed

- The workspace requires `axiolid-mesh-compile` 0.3.2 and
  `axiolid-mesh-compile-contract` 0.3.1. With 0.3.0 an `IfcPolygonalFaceSet`
  with any face of more than 3 corners, or with voids
  (`IfcIndexedPolygonalFaceWithVoids`), was refused as
  `Unsupported(Tessellation)`
  (axiolid/kernel#160), and every `IfcShellBasedSurfaceModel` as "brep has no
  solid" (axiolid/kernel#161). 0.3.1 triangulates such faces in their own
  plane and refuses a non-planar one by face index. With 0.3.1 a bend
  trimmed from a circle across its seam, as Revit writes a bar's bends
  (`270 -> 45` or `270 -> 15` degrees), was sampled around the wrong side of
  the circle, so the bend missed the next leg and the bar was refused as
  `composite directrix has a N unit gap` (axiolid/kernel#168). 0.3.2 runs the
  trim from its first end the way its sense says, wrapping past the seam. On
  the Revit rebar model below that compiles the last 1,494 refused bars: 40,990
  of 41,019 products compile, and no local model refuses a directrix gap.
- The workspace requires `axiolid-construct` 0.3.2 (reached through
  `axiolid-mesh-compile`, pinned only as a floor behind
  `compile-reference-backend`). With 0.3.0 two compiled results were wrong or
  refused, though lowering was right: an `IfcPolygonalBoundedHalfSpace` whose
  `Position` is translated within the clip plane cut the wrong region with no
  error (axiolid/kernel#164), and an opening body extruded downward, as
  Solibri and Revit hang windows from the lintel, was wound inside-out, so
  its subtraction was refused (axiolid/kernel#166; 78 of 423 real hosts
  refused with 0.3.0, 14 with 0.3.1). Both tests that pinned these now run.
  With 0.3.1 a swept disk was oriented by one fixed axis seeded from its
  first segment: a bent bar whose later leg ran along that axis was refused,
  and a leg nearly along it twisted the tube so its volume came out low with
  no error (axiolid/kernel#169). 0.3.2 carries the frame along the path. On
  the 41,019-product Revit rebar model below, the 878 refused bars compile
  and swept-disk bars within 0.5 % of their closed-form volume go from
  30,199 to 34,003 of 39,215; none is more than 1 % off.
- A gap between consecutive composite segments, or between the last and the
  first, wider than 1e-5 m is refused as `Degenerate`, naming the segment
  and the gap. It is never bridged with an edge the file did not author.
- Any other segment parent, a reparametrised segment, and a conic placed with
  a 3D placement stay typed `Unsupported`, naming the entity.

### Known limits

- The compiled mesh of a curved profile is only as close to the exact area as
  the kernel's chord budget allows. Up to `axiolid-mesh-compile` 0.3.2 that
  budget equals the linear tolerance, which is coarse for small radii: a real
  gutter profile meshes 1.9 % over its exact area and a slot 0.7 % under
  (axiolid/kernel#165). Lowering is exact; the arcs reach the kernel as arcs.
- On the local real-model corpus (423 hosts with openings whose gross Body
  compiles, six models) 14 hosts are still refused: the kernel boolean
  refuses a non-manifold operand, and each refusal names the opening. No
  host that already netted changed volume between kernels.
- Measured against IfcOpenShell 0.8.5 on 138 sampled hosts across four real
  models: 130 agree within 0.1 % (median difference about 1e-10). The other
  8 are not subtraction errors. On 7, IfcOpenShell closes a gap in the host's
  composite-curve profile with a segment the file never authored; our gross
  matches an independent exact integration of the profile to 1e-4 on the 4
  checked, IfcOpenShell's is off by up to 7 %. On 1, IfcOpenShell's own
  boolean fails and it returns the host uncut.

### Fixed

- `IfcPolygonalBoundedHalfSpace` clips now compile (#45). `PolygonalBoundary`
  was lowered through the 3D curve path as a `Curve3`, but Axiolid's
  `BoundedHalfSpace` contract and reference compiler require a `Curve2`, so
  every such `IfcBooleanClippingResult` was refused with `half-space boundary
  .. is not a Curve2 node` although lowering succeeded. The boundary now
  lowers as a `Curve2` polyline in `Position`'s XY plane, lengths converted
  to metres. A 3D boundary point is accepted only with `z = 0`; any other `z`
  violates `BoundaryDim` and is refused as `Degenerate`, naming the point,
  instead of being projected.
- `IfcSweptDiskSolid`, `IfcSweptDiskSolidPolygonal`,
  `IfcFixedReferenceSweptAreaSolid` and `IfcSurfaceCurveSweptAreaSolid` read
  `StartParam`/`EndParam` in their directrix's own parameterisation. On an
  `IfcCompositeCurve` that is not a length: ISO 10303-42 accumulates each
  segment's parametric length, 1 per `IfcPolyline` edge and a trimmed
  segment's own trim span, so an arc contributes its ANGLE in the file's
  plane-angle unit (IFC4 `IfcCompositeCurve`, figure 389: a line plus a 90
  degree arc is 91). It was converted as a length and handed to the kernel,
  which measures arc length, so a Revit rebar authored `(0, 365)` over five
  1-unit legs and four 90 degree bends was read as 365 m and refused, and a
  range that happened to fit silently cut the bar short. A full range now
  keeps the authored directrix; a partial range is cut exactly at the
  composite's parameters before the kernel sees it, honouring `SameSense`,
  `SenseAgreement`, `ParamLength` and arcs across the circle's seam. A range
  past the composite's parametric length is `Degenerate`, naming the sweep and
  the length; a range over a segment trimmed only by points is `Unsupported`,
  since its parametric length is not stated. A trimmed-curve directrix now
  converts by its basis, like any trim, instead of always as an angle.
- On the local real-model corpus (eight models) this changes one model: in a
  41,019-product Revit rebar model, compiled products go from 19,832 to
  38,618. Checked against the closed-form volume (inscribed disk polygon
  times exact path length) of all 39,215 swept-disk rebars: before, 25 were
  within 0.5 % and 18,077 compiled wrong, up to 96 % short; with this fix
  alone (`axiolid-construct` 0.3.1) 30,199 are within 0.5 %, and 34,003 with
  the kernel floors above. No product in any other model changed volume.

## [0.3.0] - 2026-09-23

### Added

- Lowering an `IfcTriangulatedFaceSet` now attaches texture coordinates from
  its `IfcIndexedTriangleTextureMap` as a corner-indexed attribute channel
  named `uv` (`lower::tessellated::UV_CHANNEL`), one `(s, t)` per triangle
  corner (#30). Positions stay shared, so the mesh stays closed. A shorter
  `TexCoordIndex` leaves trailing triangles `UNMAPPED`; an omitted one adds
  no channel; a longer one, or an index outside the texture vertices, is an
  error. A second map on the same face set becomes `uv1`, and so on.

### Known limits

- `IfcIndexedPolygonalTextureMap` (IFC4X3) is not lowered.

### Changed

- **Breaking:** requires Axiolid 0.3 (`axiolid-mesh` 0.3 adds
  `AttributeChannel::corner_indices`).

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-geometry-v0.6.0...HEAD
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.5.0
[0.4.4]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.4
[0.4.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.3
[0.4.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.2
[0.4.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.1
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
