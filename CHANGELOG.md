# Changelog

All notable changes to the OpenBIM.rs IFC family are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows Semantic Versioning.

## [Unreleased]

### Added
- `ifc-style` authors the four `IfcLightSource` forms, plus
  `IfcSurfaceStyleLighting` and `IfcSurfaceStyleRefraction`. Normalised
  ratios are bounded to 0..=1, radii and cone angles must be positive and
  finite, an all-zero attenuation triple is refused because it emits
  nothing at every distance, and a refraction index below the vacuum
  index of 1.0 is refused.

- `ifc-systems` authors distribution element occurrences: the nine
  concrete `IfcDistributionElement` classes the flow reader traverses,
  plus `IfcZone` and `IfcSpatialZone`. A closed enum keeps the class set
  to what the reader understands, and `USERDEFINED` without a naming
  `ObjectType` is refused.
- `ifc-style` authors textures: `IfcImageTexture`, `IfcTextureMap` and
  `IfcTextureCoordinateGenerator`. Cardinality floors and reference
  types are enforced; the vertex-to-face-loop correspondence is left to
  the consumer that owns the geometry, since this crate never inspects
  a representation item's shape.
- `ifc-georef` authors coordinate operations: `IfcGeographicCRS`,
  `IfcMapConversion`, `IfcMapConversionScaled` and `IfcRigidOperation`.
  `TargetCRSOnlyProjected` is enforced at construction, and a
  zero-length X axis vector is refused rather than written as a
  rotation that does not exist.
- `ifc-spatial` authors the remaining objectified relationships:
  coverings, declarations, object definition, system service, flow
  control, the three actor/product/process assignments, group-by-factor,
  element connection with and without realizing elements, and
  interference. Positions resolve through the existing `RelSlots`
  constants, so `IfcRelDefinesByObject`'s reversed slot order cannot be
  written as an aggregate. Self-connection and empty member sets are
  refused; `ImpliedOrder` writes UNKNOWN rather than `$` when no
  precedence was decided.

- Placement, connection geometry, grids and geometric sets in
  `ifc-geometry`: `IfcLocalPlacement`, `IfcGridPlacement`, `IfcGridAxis`,
  `IfcVirtualGridIntersection`, all five connection geometry forms,
  `IfcPointOnCurve`, `IfcPointOnSurface`, `IfcGeometricSet` and its curve
  form, `IfcPath`, `IfcBooleanClippingResult` and `IfcSectionedSpine`. A
  clipping result is always a difference, and a spine needs a position
  for every cross section.
- Curves on surfaces in `ifc-geometry`: `IfcPcurve`, `IfcSurfaceCurve`
  and its intersection and seam forms, `IfcCompositeCurveOnSurface`,
  both boundary curves, `IfcCurveSegment`,
  `IfcReparametrisedCompositeCurveSegment` and `IfcGradientCurve`. A
  boundary curve states `ClosedCurve` true because `IsClosed` requires
  it, and the seam and intersection forms demand two pcurves.
- Transformation operators and mapping in `ifc-geometry`: all four
  `IfcCartesianTransformationOperator` forms, `IfcRepresentationMap`,
  `IfcMappedItem` and `IfcTopologyRepresentation`. `Scale2` is written
  at slot 4 in the 2D branch and slot 5 in the 3D one, where `Axis3`
  comes between, and every scale is held to `ScaleGreaterZero` while an
  absent scale still derives to one.
- The remaining profile forms in `ifc-geometry`:
  `IfcArbitraryOpenProfileDef`, `IfcArbitraryProfileDefWithVoids`,
  `IfcCenterLineProfileDef`, `IfcCompositeProfileDef`,
  `IfcDerivedProfileDef`, `IfcMirroredProfileDef` and
  `IfcRoundedRectangleProfileDef`. The mirrored form writes its derived
  `Operator` as `*`, and the rounding radius is bounded by half of each
  rectangle dimension.
- Surface authoring in `ifc-geometry`: `IfcSphericalSurface`,
  `IfcToroidalSurface`, `IfcCurveBoundedPlane`, `IfcCurveBoundedSurface`,
  `IfcRectangularTrimmedSurface` and both B-spline surface forms. The
  B-spline writers enforce a knot identity per direction and reject a
  ragged control grid; the trim derives its senses from its parameters,
  so `Usense`/`Vsense` cannot contradict them.
- B-rep topology authoring in `ifc-geometry`: vertices, edges, oriented
  edges, loops, faces, shells and the four `IfcManifoldSolidBrep` forms,
  plus both surface models. `IfcOrientedEdge` writes its derived vertices
  as `*`, and `IfcPolyLoop` enforces the UNIQUE polygon rule.
- `ifc-geometry` authors the remaining swept solids and surfaces:
  the tapered extrusion and revolution, both directrix-driven
  sweeps, both swept disk solids, `IfcSurfaceOfLinearExtrusion`,
  `IfcSurfaceOfRevolution`, `IfcPlane`, `IfcCylindricalSurface` and
  `IfcAxis1Placement`. A swept disk solid refuses an inner radius
  that is not smaller than its outer one.
- `ifc-geometry` authors the curve families: `IfcLine` and `IfcVector`,
  `IfcCircle`, `IfcEllipse`, `IfcTrimmedCurve`, `IfcCompositeCurve`
  and its segments, `IfcIndexedPolyCurve`, the 2D and 3D offset
  curves, and both B-spline-with-knots forms. The B-spline writers
  enforce `sum(KnotMultiplicities) = Degree + |ControlPoints| + 1`,
  and trim parameters keep their `IfcParameterValue` wrapper.
- `ifc-geometry` authors the CSG primitives and half spaces:
  `IfcBlock`, `IfcSphere`, `IfcRightCircularCone`,
  `IfcRightCircularCylinder`, `IfcRectangularPyramid`, `IfcCsgSolid`,
  the three `IfcHalfSpaceSolid` forms and `IfcBoundingBox`. Every
  dimension is `IfcPositiveLengthMeasure`, so zero and non-finite
  values are refused.
- Tessellated geometry authoring in `ifc-geometry`: `IfcCartesianPointList2D`
  and `3D`, `IfcTriangulatedFaceSet`, `IfcPolygonalFaceSet`,
  `IfcIndexedPolygonalFace` and the with-voids variant. Callers pass 0-based
  indices; the writer converts to the schema's 1-based `IfcPositiveInteger`
  and refuses indices past the end of the point list.
- Standard section authoring in `ifc-geometry`: I, L, T, U, Z and C
  shapes, the asymmetric I, ellipse, trapezium, and the rectangular and
  circular hollow profiles. Every EXPRESS WHERE rule on these entities is
  enforced by the writer, which the workspace validator does not implement
  for profiles. Kernel-free: no axiolid type appears in any signature.
- `authoring_conformance` in `openbim-ifc`: every domain crate authors
  through its own API and the result is validated against the schema,
  covering the six entity types written by more than one crate.
- `create_quantity_with` and `QuantityExtras` in `ifc-properties`, reaching
  the `Description`, `Unit` and `Formula` attributes of a simple quantity.
- Referent authoring in `ifc-alignment`: `referent`, `linear_placement`,
  `axis2_placement_linear`, `point_by_distance`, `cartesian_point` and
  `stationing`, which writes the `Pset_Stationing` set the station reader
  looks for by name.
- `ResourceEditor::create_resource_type` in `ifc-resource`, authoring all
  six `IfcTypeResource` subtypes. `PredefinedType` is required on a type
  where it is optional on an occurrence, and `Usage` is refused because it
  belongs to the occurrence.
- Georeferencing authoring in `ifc-georef`: `create_projected_crs`,
  `create_representation_context`, `create_representation_subcontext` and
  `create_direction`. The subcontext writes its four inherited geometry
  attributes as `*`, which the schema derives from the parent context.
- `create_monetary_unit` in `ifc-cost`, the currency every cost value in a
  file is denominated by.
- Template and type authoring in `ifc-properties`:
  `add_property_set_template`, `attach_template`, `attach_type`,
  `add_measure_with_unit` and `add_context_dependent_unit`.
- Systems authoring in `ifc-systems`: `create_system`, `create_port`,
  `assign_to_group`, `nest_ports`, `connect_port_to_element` and
  `connect_ports`, plus `contain_in_spatial_structure` and
  `reference_in_spatial_structure`. The crate could trace flow through a network it could
  not build. Authoring resolves the readers' own slot constants.
- Spatial structure authoring in `ifc-spatial`: `create_project`,
  `create_spatial_element` for site, building, storey and space, and the
  `aggregate` and `contain` relationships. The crate could walk a spatial
  tree it had no way to build. Authoring resolves the same slot constants
  the reader uses, so the inverted layouts of `IfcRelAggregates` and
  `IfcRelContainedInSpatialStructure` cannot drift apart.
- Every property value type is authorable in `ifc-properties`:
  `add_property_enumerated_value`, `add_property_bounded_value`,
  `add_property_list_value`, `add_property_table_value`,
  `add_property_reference_value` and `add_complex_property`, alongside
  `add_element_quantity` and `add_physical_complex_quantity` for quantity
  takeoff. The schema WHERE rules are enforced at authoring time: bounded
  values must share a measure, table columns must pair and stay
  homogeneous, and a complex property cannot contain itself.
- Material relationship authoring in `ifc-material`:
  `create_material_relationship`, `create_material_properties`,
  `create_material_classification_relationship`,
  `create_material_definition_representation` and
  `create_profile_with_offsets`. Every instantiable material entity the
  crate can read is now authorable; the remaining four are abstract
  SELECT or supertype declarations.
- Property set authoring in `ifc-properties`: `add_property_single_value`,
  `add_property_set` and `attach_property_set`. The crate could resolve
  property sets and author none. Enforces the schema ExistsName,
  UniquePropertyNames and NoRelatedTypeObject rules.
- Event, lag and recurrence authoring in `ifc-schedule`: `create_event`,
  `create_event_time`, `create_lag_time` and `create_recurrence_pattern`
  close the last four readable-but-not-authorable entity types. Slot
  order was verified against the bundled IFC4X3_ADD2 EXPRESS schema, not
  recalled; the schema's two `USERDEFINED` WHERE rules are enforced at
  authoring time.
- Schedule authoring for work plans, work schedules, calendars and the
  relations that bind them: `create_work_control`, `create_work_calendar`,
  `create_work_time`, `assign_tasks_to_control` and `nest_tasks`. A
  programme can now be authored, not only read.

- `VIENNESEBEND` transition spirals lower exactly. IFC 4.3 defines the
  family as a 7th order polynomial spiral whose curvature depends on the
  superelevation swing and the gravity centre line height, not on the
  endpoint radii alone; expanded in arc length that is a degree-7
  polynomial, which `CurvatureLaw::Polynomial` holds without truncation.

- `constraint::placement::derive` resolves an `IfcLinearPlacement` that states
  only an `IfcPointByDistanceExpression`, by evaluating the basis curve through
  an injected `CurveEvaluator`. Behind the non-default `compile` feature: the
  bridge names the capability and links no implementation.
- `ifc_alignment::gradient_curve3` returns the composed centreline as a
  `Curve3`, so a caller can evaluate it without going through a graph.
- `lower_gradient_curve` composes an alignment horizontal layout with its
  vertical profile as an exact `Curve3::Elevated`, so a clothoid plan and a
  parabolic profile both survive unapproximated.
- `elevation_law` and `profile_law` map IFC vertical segments to exact
  `ElevationLaw`s, covering CONSTANTGRADIENT and PARABOLICARC.

### Changed

- `spiral_curve`, `is_exactly_lowerable`, `lower_horizontal_layout` and
  `lower_horizontal_layout_partial` take the cant layout. A horizontal
  segment alone does not determine a Viennese bend, so the signature now
  states that rather than implying otherwise. Callers with no cant pass
  `None` and behave as before.
- `lower_horizontal_segment` refuses a Viennese bend by name: a single
  segment has no chain context, so no true distance along exists for it.

- Axiolid kernel crates move to `0.2.1`, which publishes
  `axiolid-curve-evaluate-contract`.

- Axiolid kernel crates are consumed from crates.io at `0.2.0` instead of an
  exact git tag. The tag pin existed only while the kernel was unpublished.

### Removed

- `docs/local-kernel.md`: the local-checkout workaround it described is
  obsolete now that every kernel crate is published.

### Added
- `product_world_transform` resolves products placed by `IfcLinearPlacement`, using the cached `CartesianPosition`. Deriving a frame from the basis curve is a typed refusal, not an approximation.
- `ifc-alignment` can author alignment records: `IfcAlignment`, the
  horizontal, vertical and cant layouts, `IfcAlignmentSegment`, and the
  three parameter-segment kinds. Readers and authoring share one set of
  slot constants (ADR 0011).
- `ifc-geometry` can now author geometry as well as read it (ADR 0011):
  points, directions, axis placements, polylines, rectangle/circle/arbitrary
  profiles, extruded and revolved area solids, and unevaluated boolean
  results. The authoring API links no geometry kernel -- it takes plain
  `f64`, `[f64; 3]` and index buffers -- so vertices computed with CGAL,
  OCCT or Axiolid can all be authored through one path.
- `ifc-schedule` can now author programmes, not only read them:
  `create_task`, `create_task_time` and `create_sequence`. Constructors
  index the slot constants the existing readers own, so `IfcTask`'s seven
  inherited slots are reserved rather than hand-counted, and an authored
  chain drives `execution_order` in the right direction. Self-loops,
  out-of-range `Priority` and `Completion`, malformed `GlobalId`s and
  unknown `DurationType` values are refused. ISO 8601 durations and
  timestamps are stored exactly as authored.
- `ifc-structural` can now author boundary conditions with
  `stage_boundary_condition`, covering node, node-warping, edge and face
  families. The kind selects the entity, its attribute names and its
  measure wrapper together, so authored values read back through the
  existing schema-driven accessors. Stiffness values the family does not
  declare -- rotational on a face, warping outside
  `IfcBoundaryNodeConditionWarping` -- are refused rather than dropped.
- `ifc-cost` can now author the quantities a cost item measures:
  `create_quantity` for length, area, volume, count, weight and time, and
  `assign_cost_quantities` to bind them to an `IfcCostItem`. Each kind
  writes its matching typed measure, and the measured value lands in the
  slot the existing reader scans for, after the Name/Description/Unit
  attributes inherited from `IfcPhysicalQuantity` and
  `IfcPhysicalSimpleQuantity`. Negative measures, fractional counts,
  empty or duplicated attachments, and entities that are not physical
  quantities are refused.
- `ifc-material` can now author the whole `IfcMaterialSelect` family, not
  just layers: `create_constituent`, `create_constituent_set`,
  `create_profile`, `create_profile_set`, `create_material_list`,
  `create_layer_set_usage`, `create_profile_set_usage`, and
  `create_layer_with_offsets`. The offsets subtype writes its seven
  inherited `IfcMaterialLayer` slots before its own two, so it reads back
  through the existing accessors. Refusals cover a constituent `Fraction`
  outside `0..=1`, a `CardinalPoint` outside `1..=9`, empty compositions,
  and a usage bound to the wrong kind of set. `DirectionSense` and
  `LayerSetDirection` gained `as_token`, the inverse of `parse`.
- `ifc-author` can now author ownership: `IfcPerson`, `IfcOrganization`,
  `IfcPersonAndOrganization`, `IfcApplication` and `IfcOwnerHistory`. Every
  `IfcRoot` subtype carries `OwnerHistory`, which the domain crates write as
  null; callers that want provenance can now supply a real record. Ownership
  is never attached automatically, so an omitted history stays honestly
  absent rather than being filled with an invented actor. A `ChangeAction`
  outside `IfcChangeActionEnum`, a `LastModifiedDate` earlier than
  `CreationDate`, and a person naming nobody are all refused before staging.
- `ifc-properties` can now author the project unit context, not only read it.
  Eight `Transaction` helpers stage `IfcSIUnit`, `IfcMonetaryUnit`,
  `IfcConversionBasedUnit`, `IfcDerivedUnit`, `IfcDerivedUnitElement`,
  `IfcDimensionalExponents` and the `IfcUnitAssignment` that binds them to the
  project. An SI prefix outside `IfcSIPrefix` is refused by reusing the
  existing `prefix_exponent` table rather than a second list, and an empty
  `IfcUnitAssignment` is refused outright: both mistakes produce a file that
  parses and validates while silently mis-scaling every measure in it.
- `ifc-style` now reads the IFC presentation entities that carry no shape, so
  every concrete `IfcRepresentationItem` subtype in IFC4 ADD2 TC1 is finally
  named somewhere in the workspace:
  - All five light sources (`IfcLightSourceAmbient`, `…Directional`,
    `…Goniometric`, `…Positional`, `…Spot`) via `StyleView::light_source*`,
    plus the photometric `IfcLightIntensityDistribution` and
    `IfcLightDistributionData`. `LightSource::kind` resolves the concrete
    subtype most-specific-first, so a spot never reports as merely positional.
    `LightDistributionData::samples` pairs `SecondaryPlaneAngle` with
    `LuminousIntensity` and returns a typed error when the two lists are
    index-misaligned, rather than `zip`-truncating a luminaire's photometry.
  - `IfcPlanarExtent` and `IfcPlanarBox` via `StyleView::planar_extent` /
    `planar_box`.
  These were already classified `non-shape` with `ifc-style` as owner in
  `ifc-geometry/data/ifc4-representation-item-dispositions.tsv`; the ledger
  named an owner that had never implemented them. Tests cover IFC2x3, IFC4
  ADD2 TC1, and IFC4X3 ADD2 plus a real STEP round-trip, so inherited slots are
  resolved by name rather than by a guessed subtype offset.

- `ifc-geometry` feature `compile` (**off by default**): hands the lowered
  neutral DAG to an Axiolid mesh provider and returns triangles.
  `compile_product_mesh(&model, product, tolerance)` yields `Ok(None)` for a
  product with no body representation, or the new
  `GeometryError::CompilationRefused` carrying the provider's own reason.
  Amends [ADR 0004](/adr/0004-geometry-bridge-not-kernel), which
  previously excluded execution providers from the workspace entirely; the
  bridge still implements no geometry. Enforced, not just documented: the
  default and `--no-default-features` columns link zero provider crates, and
  `package_architecture.rs` walks the feature graph from `default` so a
  provider cannot arrive through a default-enabled feature edge.
- `ifc-geometry/tests/compile_pairing.rs`: pins lowering and compilation
  together over a fixture corpus — every product must reach a typed answer,
  triangles or an attributed refusal. Includes
  `union_over_halfspace_unbounded.ifc`, authored to be refused, because every
  other fixture compiles cleanly and the refusal branch would otherwise never
  execute.

- Workspace lints: `missing_docs = "deny"`. Thirteen crates enforce it; the
  nine still carrying debt are capped by `scripts/check-missing-docs.py`, now
  part of the gate, so the count can only shrink.
- `ifc-geometry` documents every public item and joined the enforced set
  (32 items documented, mostly the MaterialResource geometry views).
- `ifc-step`: `Index` reads a file without decoding it. `Index::scan` keeps
  only record boundaries and type names, so a 529 MB export with 9,000,008
  records is indexed in 0.66 s holding 206 MB against 18.2 s and 2366 MB for
  a full parse. `Index::entity` decodes one record on request and
  `materialize_closure` builds a self-contained `Model` from a subset.
- `benchmarks/`: parse cost measured against ifc-lite and ifcopenshell
  on identical 1 MB to 513 MB files, all three agreeing on entity count.
  At 513 MB / 9M entities: 18.16 s and 2366 MB here, 17.14 s and 2750 MB
  for ifc-lite decode, 54.34 s and 6444 MB for ifcopenshell. ifc-lite
  index-only scan answers the same file in 2.05 s holding nothing, which
  is the shape this parser has no answer for.

- `docs/capabilities.md`: a generated section listing concrete
  `IfcRepresentationItem` subtypes that appear nowhere in the
  repository. The existing tables are derived from the dispatch code, so
  every row in them is Implemented or Refused by construction and the
  matrix could never report a gap. Walking down from the IFC4 schema
  instead finds 8 of 111 concrete geometry items unaddressed (the five
  light sources, `IfcPath`, `IfcPlanarBox`, `IfcVertexLoop`).

- `ifc-alignment`: HELMERTCURVE and SINECURVE horizontal transition segments
  lower exactly as `Curve2::Intrinsic`. Helmert is one curve carrying a
  `CurvatureLaw::Piecewise` with a seam at half length and two quadratic
  pieces, each written in its own rebased arc length; splitting it into two
  curves is impossible without the seam position, a non-elementary integral.
  Sine uses the kernel's `sine_corrected_transition`, matching IFC's published
  law exactly. VIENNESEBEND is now the only refused horizontal family: its law
  needs the cant swing from the separate `IfcAlignmentCant` layout. Still no
  quadrature, series, or sampling in the crate.
- `ifc-step`: measured parse cost at model scale and documented it in
  `docs/api/rust.md`. A 115 MB / 2M-entity export parses in 3.6 s using
  1518 MB peak RSS (13.2x the file size); ifcopenshell 0.8.5 takes 8.2 s
  and 2087 MB on the same file. `tests/scale.rs` pins the ratio so a
  regression fails the gate.
- `rust-toolchain.toml` pins 1.88.0 so the local gate and CI check the
  same thing; previously `gate.sh` used whatever toolchain was installed.
- `ifc-validate`: WHERE rules for the relationship families `ifc-spatial`
  reads. `IfcRelSpaceBoundary.CorrectPhysOrVirt` ties declared physicality
  to the bounding element across all three concrete subtypes,
  `NoSelfReference` covers the four `IfcRelAssigns` subtypes, each naming
  its own relating attribute, and `IfcRelConnectsPathElements` priorities
  are bounded to 0..=100 with an empty list treated as conformant per the
  rule's own OR clause.

- `ifc-alignment`: CLOTHOID, BLOSSCURVE and COSINECURVE transition spirals
  lower exactly as `Curve2::Intrinsic`, the natural-equation curve added in
  axiolid-curve v0.12.0. A spiral has no elementary parametric form, but its
  curvature *is* an elementary function of arc length, and a plane curve is
  fixed up to rigid motion by that law; anchoring it to a start frame fixes it
  absolutely. Storing the law is therefore lossless -- the crate still
  performs no quadrature, series expansion, or sampling anywhere.
  HELMERTCURVE, SINECURVE and VIENNESEBEND remain a typed refusal: their laws
  need terms an `IfcAlignmentHorizontalSegment` does not carry, and forcing
  them into a nearby law would be a silent approximation.
- `ifc-alignment`: composite runs now end after a transition spiral rather
  than asserting a transition across it. Continuity *into* a spiral is
  provable from the predecessor's closed-form end point, but continuity *out*
  of one is a Fresnel-type integral, and `Transition` has no "unknown" member
  -- so the run ends instead of claiming a fact the crate cannot verify.
- `ifc-geometry`: the `IfcSameValue` tolerance family (`IfcSameValue`,
  `IfcSameCartesianPoint`, `IfcSameDirection`, `IfcSameAxis2Placement`),
  plus `IfcPointListDim`, `IfcOrthogonalComplement` and `IfcBuild2Axes`,
  transcribed in `resource::functions`. The schema's comparison band is
  exclusive -- values exactly `epsilon` apart are not equal -- so a
  symmetric `abs() <= eps` would have been subtly wrong; a unit test pins
  the boundary. `IfcSameAxis2Placement` diverges deliberately: the
  published function compares `ap1.Location` with itself, never testing
  the location at all, and that defect is not reproduced.
- `ifc-spatial`: space boundaries. `relation::boundary` reads
  `IfcRelSpaceBoundary` and both concrete subtypes, resolving the bounded
  space, the bounding element, `PhysicalOrVirtualBoundary`,
  `InternalOrExternalBoundary`, and the `ParentBoundary` /
  `CorrespondingBoundary` links that make second-level boundaries usable
  for heat transfer. `Model::ids_of_type` matches exact type names, so all
  three concrete types are queried: a lookup of the supertype alone misses
  every real BEM export and reports a building with no boundaries at all.
  The four `EXTERNAL_*` exposure members are kept apart from plain
  `EXTERNAL` rather than merged, since ground and water contact are
  different heat-transfer paths. `CorrectPhysOrVirt` agreement is reported
  through `physical_matches_element`, not enforced: this crate states what
  the file says and leaves rejection to `ifc-validate`.
- `ifc-spatial`: coverings. `IfcRelCoversBldgElements` and
  `IfcRelCoversSpaces` join the generic relationship reader, so a finish
  is reachable from the element it clads and from the space it bounds.
  The two stay distinct: one suspended ceiling can cover a slab and bound
  a room, and collapsing them loses which question was asked. Layer order
  is preserved, since a build-up is ordered.
- `ifc-spatial`: element connection and interference.
  `IfcRelConnectsElements` with its `PathElements` and
  `WithRealizingElements` subtypes, plus `IfcRelInterferesElements`. The
  connects family places `ConnectionGeometry` at slot 4, so its two ends
  sit at 5 and 6 rather than 4 and 5; reading the usual pair would name
  the connection geometry as the relating element. Interference is not a
  subtype of it and keeps the 4/5 layout. Both positions are asserted
  against the shipped schemas. Connection and interference stay separate
  kinds: a clash is not an adjacency.
- `ifc-spatial`: assignment relationships. `IfcRelAssignsToActor`,
  `ToProcess`, `ToProduct` and `ToGroupByFactor` join the generic reader,
  answering who is responsible for an object, which task consumes it, and
  what it belongs to. Their two ends bracket `RelatedObjectsType`: the
  related list comes first at slot 4, the enumeration sits at 5, and the
  relating end is at 6. A reader assuming the usual "relating at 5" finds
  an enumeration rather than a reference and the assignment disappears
  with no error, so the positions are asserted against the schemas.
- `ifc-spatial`: the last five relationship families. `IfcRelDeclares`
  (what a project context declares), `IfcRelDefinesByObject` (an
  occurrence defined by another), `IfcRelFlowControlElements` (controls
  governing a flow element), `IfcRelServicesBuildings` (which structures
  a system serves) and `IfcRelConnectsWithEccentricity`. These five do
  not share a layout: three put the relating end first, two put the
  related list first, and inverting either direction is silent. With
  these, all 42 concrete `IfcRel*` families in IFC4 are read.


### Changed

- `ifc-style/src/view.rs` split: the `Record` attribute reader moved to
  `view/record.rs`, keeping both halves under the repository's 800-line
  module gate as the light and extent projections landed.

- `ifc-step`: records are converted as the parser emits them instead of
  being collected into a `Vec` first. `openbim_step::parse_with` buffers
  every `DataRecord`, so the generic records and the converted model were
  both fully resident at peak; the conversion now runs inside an
  `EventSink`. A 115 MB / 2M-entity export goes from 1232 MB resident and
  3.2 s to 750 MB and 2.0 s -- 39% less memory, 40% faster. The budget in
  `tests/scale.rs` is tightened from 25x to 10x so the win cannot silently
  regress.

- `ifc-alignment`: kernel pin moved from axiolid v0.12.0 to v0.14.0, for
  `CurvatureLaw::Piecewise` and `CurvatureLaw::Composite`.

- `ifc-geometry`: the EXPRESS function registry now records
  `Implemented` and `NotApplicable` alongside `Scaffolded`, and no row
  remains `Scaffolded`: 20 implemented, 6 native primitives, and
  `IfcListToArray`/`IfcMakeArrayOfArray` marked `NotApplicable` because
  they only re-index a LIST into an ARRAY, which a `Vec` already is.
- `ifc-geometry`: five functions were implemented but filed under the
  wrong owner or left `Scaffolded` -- `IfcCurveDim` (`rules::dimension`),
  `IfcBaseAxis` (`resource::axes`), and `IfcBuildAxes`,
  `IfcFirstProjAxis`, `IfcSecondProjAxis` (all `transform`).

### Fixed
- `ifc-properties::create_quantity` wrote four attributes where every
  `IfcPhysicalSimpleQuantity` subtype declares five, so `Formula` could
  never be read back by this crate's own reader.
- `ifc-cost::create_quantity` encoded `IfcCountMeasure` as a real, emitting
  `4.` where EXPRESS declares INTEGER.
- `ifc-cost::assign_schedule_items` wrote `.CONTROL.` into
  `RelatedObjectsType`, which IFC4 redeclares as `IfcStrippedOptional`, a
  BOOLEAN. It is now left unset.
- `add_si_unit` wrote `IfcSIUnit.Dimensions` as `$`, an omitted value. The
  schema declares it DERIVE, which STEP spells `*`; the two are different
  claims and a validator rejects the first.
- `scripts/sync-capabilities.py` no longer reports "the IFC4 schema was not
  available" when the unaddressed-entity walk legitimately finds zero. The
  missing-schema case already raises above that branch, so an empty list is a
  measurement rather than a missing one; the page now says so, and still
  distinguishes *named* from *implemented*.

- `ifc-material`: the published MaterialResource inventory was missing
  `IfcMaterialDefinitionRepresentation`. A new completeness check derives
  the expected set from the normative EXPRESS schema, so a short list now
  fails instead of asserting its own length.

- CI now runs the schema-backed tests. `references/ifc-spec` is not
  committed (CC BY-ND 4.0), so every test loading it silently skipped --
  including in CI, which is how an IFC2X3 slot-name bug reached main
  green. `scripts/fetch-ifc-schemas.sh` fetches the three normative
  schemas against pinned checksums of line-ending-normalised content,
  and `IFC_SPEC_REQUIRED=1` turns a skip into a failure.

- `ifc-spatial`: `tests/slot_layout.rs` asserted the IFC4 attribute
  name against every bundled schema. IFC2X3 spells
  `IfcRelCoversSpaces` slot 4 `RelatedSpace`; IFC4 renamed it to
  `RelatingSpace`. The slot position is 4 in both, so the reader was
  always correct -- only the test was wrong, and it failed on a
  schema the crate supports. The expected name now resolves per
  version, leaving the IFC4/IFC4X3 assertions intact.

- `ifc-geometry`: the WHERE-rule coverage gate matched violations by rule
  label alone, so a violation raised by any entity satisfied any other
  entity's case for the same label. `IfcBooleanResult.SameDim` passed on a
  violation raised by `IfcPolyline.SameDim`. The gate now matches the entity
  type as well, accepting a declared subtype or a dimensionality-suffixed
  concrete form (`IfcCartesianTransformationOperator3D` for the operator
  supertype) and nothing wider. The equivalent hole in the *naming* gate had
  already been closed; the behavioural gate had kept it.
- `ifc-geometry`: `IfcBooleanResult.SameDim` had two implementations. The
  older one resolved operands through a local `operand_dim` that answered
  `Some(3)` for every family it recognised and `None` otherwise, so its
  comparison was unreachable -- dead code that still read as enforcement.
  Removed it along with `operand_dim`; `boolean_operands` owns the rule and
  resolves dimensionality through `dimension::dim_of`.
- `ifc-geometry`: `resource::functions` recorded eight functions as
  `Scaffolded` ("semantics remain to implement") that were already executing.
  Added `FunctionStatus::Implemented` and a manifest test asserting an
  `Implemented` row is named by its owner module, so the registry can no
  longer understate the crate.
- `ifc-geometry`: a `Scaffolded` row in the EXPRESS function registry can
  no longer stay stale after its function is implemented. The manifest
  checked only that `Implemented` rows were backed by code, so the
  registry rotted downward instead: eight rows understated the crate for
  two commits. The inverse check is now enforced.
- `docs`: the "Objectified relationship traversal" capability row said
  `ifc-spatial::relation` reads three `IfcRel*` families and that other
  families "are not interpreted". Sixteen crates read 26 of the schema.s
  40 concrete families: assignments, definitions, connections, port
  connectivity, voiding and filling, sequencing, and associations. The row
  now lists them by crate. It sits outside the generated sentinel blocks,
  so nothing had checked it against the source; a new
  `openbim-ifc/tests/relationship_census.rs` re-counts the families and
  fails when the stated number drifts from what the crates actually read.
- `docs`: the relationship census counted doc-comment mentions as readers,
  overstating it as 26 across 16 crates. A module doc that names a family to
  contrast its slot layout with one the crate does read is not a reader:
  `IfcRelServicesBuildings` and `IfcRelSpaceBoundary` were counted this way
  while no crate reads either. The real figure is 24 across 14 crates, with
  16 families unread. `relationship_census.rs` now skips comment lines.
- `docs`: two capability rows carried a literal `...[truncated]` marker on
  the published site, one since 3e349cf. Both restored to full prose.

### Added

- `ifc-alignment`: `lower_horizontal_layout_partial` lowers a horizontal
  layout as far as exactness allows instead of failing the whole layout on
  the first transition spiral. Real railway and highway alignments interleave
  spirals between their lines and arcs, so the all-or-nothing entry point
  refused essentially every production file. The partial result keeps maximal
  runs of exactly-lowered consecutive segments and reports each refused
  segment with its entity id and authored `PredefinedType`, so a caller can
  say "3 of 5 lowered, CLOTHOID #103 and #107 refused" rather than only that
  something failed. A run ends at every refusal: continuity across a segment
  this crate did not lower is not a fact it is entitled to assert. Still no
  approximation anywhere -- `lower_horizontal_layout` keeps its exact
  all-or-nothing contract unchanged.
- `ifc-alignment`: repinned the neutral kernel to `axiolid` v0.12.0, whose
  `Curve2::Intrinsic` carries a curvature-law-plus-start-frame natural
  equation. This is the representation the transition-spiral families need to
  be storable exactly; lowering them onto it is the follow-up to this change,
  not part of it.

- `ifc-geometry`: the remaining 20 IFC4 geometry-resource `WHERE` rules are
  enforced, completing all 95. Two readers were added under their declared
  owners rather than inside the rules layer: `surface::basis` implements
  `IfcGetBasisSurface`/`IfcAssociatedSurface`, and
  `solid::brep::non_advanced_faces` walks shell faces. These feed
  `SameSurface`, `DistinctSurfaces`, `HasAdvancedFaces` and
  `VoidsHaveAdvancedFaces`; the rest cover boolean operands, composite-curve
  continuity, directrix bounding, trim value kinds, the revolution axis, and
  mapped representations.
- `ifc-geometry`: `FunctionStatus::Implemented` records that a normative
  EXPRESS function is actually executed, and a manifest test rejects any row
  claiming it whose owner module does not name the function. Eight rows that
  understated the crate were corrected.

- Enforced the schema's normative EXPRESS functions, taking executable
  `WHERE` rules from 63 to 75 of 95. `src/rules/express.rs` transcribes
  `IfcConstraintsParamBSpline` and `IfcConsecutiveSegments` and is unit-tested
  against the specification text itself, so the transcription is checkable
  line by line rather than only through the rules that call it. Covers
  B-spline curve and surface parametrisation, weight positivity, knot/
  multiplicity correspondence, indexed poly-curve continuity, tapered profile
  correspondence, and `IfcLocalPlacement.WR21`.
- `IfcLocalPlacement.WR21` honours the function's three-valued result: the
  schema returns UNKNOWN for grid placements and unrecognised shapes, which
  EXPRESS treats as satisfied, so only the single explicit FALSE branch -- a
  3D relative placement on a 2D parent -- is reported.

- `ifc-georef`: `GeorefView::for_model` pins the declared schema to IFC4 or
  IFC4X3; IFC2X3 is refused with `GeorefError::UnsupportedSchema` since it
  declares no georeferencing entities at all (verified against `IFC2X3_TC1.exp`).
  `resolve_project_to_map_in` resolves an `IfcMapConversion` through a pinned
  view, distinguishing an IFC4X3-only coordinate-operation entity
  (`IfcMapConversionScaled`, `IfcRigidOperation`) read under IFC4 -- a schema
  mismatch -- from a genuinely wrong entity id.
- `ifc-georef`: `compose_project_frame` chains a resolved project-to-map
  operation onto a separately supplied project frame (`IfcLocalPlacement` or
  equivalent, owned by `ifc-geometry`), producing one map-frame transform;
  refuses a singular (non-invertible) project frame rather than propagating
  a degenerate composed transform.
- `ifc-georef`: `NorthReference::{Project,True,Grid}` distinguish IFC's three
  north references. `resolve_true_north` reads `IfcGeometricRepresentationContext.TrueNorth`
  when declared and falls back to IFC's documented default (the project Y
  axis) when absent, rather than silently treating "unspecified" as "equal
  to grid north". `grid_north_direction` derives grid north from the
  resolved map conversion's own rotation, so true, grid, and project north
  can all disagree simultaneously, matching real georeferenced files.
- `ifc-alignment`: `AlignmentView::for_model` pins the declared schema to
  IFC4X3 (`IFC4X3`/`IFC4X3_ADD2`); IFC2X3 and IFC4 ADD2 TC1 are refused with
  `AlignmentError::UnsupportedSchema` since `IfcAlignment*` entities do not
  exist in either schema at all.
- `ifc-alignment`: `CantLayout` resolves, orders, and C0-continuity-checks a
  full `IfcAlignmentCant` profile, with `cant_at`/`cant_at_distance`
  evaluating all seven `IfcAlignmentCantSegmentTypeEnum` closed-form base
  formulas (BLOSSCURVE, CONSTANTCANT, COSINECURVE, HELMERTCURVE,
  LINEARTRANSITION, SINECURVE, VIENNESEBEND) exactly -- cant states its
  elevation value directly as a function of arc-length, so every type is
  exactly representable with no integration and no approximation.
- `ifc-alignment`: `lower_horizontal_layout` assembles an `IfcAlignmentHorizontal`'s
  nested segment chain into one continuity-aware `CurveRelation::Composite`,
  observing (not assuming) the `Transition` between consecutive segments from
  exact endpoint equality.
- `ifc-alignment`: `resolve_linear_placement`/`resolve_point_by_distance`
  resolve `IfcLinearPlacement` through `IfcAxis2PlacementLinear` to the
  mandatory `IfcPointByDistanceExpression`; `station_equations` resolves every
  `IfcReferent` carrying `Pset_Stationing` into its distance-along/station
  mapping, including station-equation `IncomingStation` discontinuities.
- Enforced 47 more IFC4 geometry `WHERE` rules, taking the executable count
  from 16 to 63 of 95. New rule modules cover dimensionality (via a
  transcription of the schema's own `IfcCurveDim` derivation), positive
  scalars, list cardinality, type membership and surface degeneracy. Every
  newly enforced row carries a conforming and a violating model in
  `tests/where_rule_inventory.rs`, and the coverage assertion fails if a row
  claims implementation without one.
- `ifc-resource`: `IfcPerson`, `IfcOrganization`, `IfcOrganizationRelationship`,
  `IfcPersonAndOrganization`, and `IfcActorRole` projections, enforcing
  `IdentifiablePerson`, `ValidSetOfNames`, and `WR1` (`USERDEFINED` roles
  require `UserDefinedRole`).
- `ifc-resource`: all six concrete `IfcConstructionResourceType` kinds, with
  `IfcRelDefinesByType` assignment resolution that refuses a second relation
  naming a different type for the same occurrence.
- `ifc-resource`: `IfcInventory` metadata and `IfcActorSelect` jurisdiction
  projection, with `IfcRelAssignsToGroup` membership resolved in authored
  order.
- `ifc-resource`: `IfcPhysicalSimpleQuantity` (all six concrete measure
  kinds) and `IfcPhysicalComplexQuantity` usage-quantity projections,
  enforcing the shared non-negative/finite value rule and
  `NoSelfReference`.
- `ifc-resource`: IFC4X3 ADD2 accepted alongside IFC4 ADD2 TC1 for every
  resource, actor, inventory, and usage-quantity projection (entity shapes
  verified identical against `IFC4X3_ADD2.exp`). IFC2X3 remains an explicit
  `UnsupportedSchema` refusal: it does not declare `IfcConstructionResourceType`,
  `IfcResourceTime`, or `PredefinedType` on `IfcConstructionEquipmentResource`/
  `IfcCrewResource`, so there is no normative behavior to project.

### Fixed

- `IfcBooleanClippingResult.OperatorType` was implemented under the label
  `FirstOperandType`, so the operator check reported the wrong rule and the
  real `FirstOperandType` -- the first operand must be a swept area, swept
  disc or nested clipping result -- was never checked at all. Both now exist
  under their schema names.
- `IfcSubedge` now lowers instead of being refused. A subedge states its own
  `EdgeStart`/`EdgeEnd` and inherits the carrier curve from `ParentEdge`,
  which is reached by walking the parent chain: `ParentEdge` is typed
  `IfcEdge`, so a subedge of a subedge is legal and stopping at the first hop
  would leave the carved edge with no geometry.

### Fixed
- `IfcGeometricRepresentationContext.WorldCoordinateSystem` is now applied
  when lowering a product. It is a mandatory attribute defining model space,
  but was read and tested without ever reaching the lowered frame, so a file
  that surveys its site into a real coordinate system placed every product at
  the wrong location. Nearly all files write the identity, which is why a
  corpus pass never revealed it. The context frame composes above the
  placement chain, so a rotated context rotates the sited product.
- Parameter-space lowering for `IfcTrimmedCurve` and `IfcCompositeCurve` p-curve reference curves; trim parameters stay unscaled because a (u, v) address is dimensionless.
- Typed `position()` placem

... [OUTPUT TRUNCATED - 21,622 chars omitted out of 71,549 total] ...

thetic
  `IFC4X3_ADD2` document, asserts exact IFC4X3 bundle routing, and checks the
  typed `structure.required.missing` finding. It previously loaded an IFC4
  fixture while claiming IFC4X3 coverage.
- `Budget.max_findings` now hard-caps report storage in `Report::push` and
  `Report::extend`; previously validation marked a report truncated only after a
  phase had already recorded an unbounded number of findings.
- Duplicate GlobalIds are now reported once by the canonical
  `global.UniqueGlobalId` rule instead of once during structural validation and
  again during native-rule evaluation.
- The IFC4X3 inventory guard now counts only `TYPE ` declarations; a
  line-leading `TYPEOF(...)` expression had inflated the preliminary count from
  436 to 437.
- `ifc-classification` hierarchy budgets now count every followed
  `ReferencedSource` edge and every distinct resolved entity, including the
  terminal classification system; revisiting an already-counted reference
  reports a cycle before node exhaustion.
- The `ifc-schema-generate` tool hardcoded IFC4's entity/type counts, so it
  could not produce another schema's artifact and its guard could not detect
  the wrong source file for one. It now takes a schema selector with per-schema
  expected counts, pinned to the committed artifacts by a test.
- `ifc-validate` SELECT membership bounded the walk by loop iterations rather
  than distinct types visited. IFC4's value selects are wide -- reaching
  `IfcMonetaryMeasure` needs `IfcAppliedValueSelect -> IfcValue ->
  IfcDerivedMeasureValue`, whose 60 members exhausted the 32-iteration budget --
  so legal typed values were reported as non-members. Eight false findings
  across three committed fixtures. The bound is now on visited types, and an
  exhausted walk returns "cannot answer" instead of "not a member".
- `test/fixtures/nurbs/ifc4_rational_bspline_curve_surface.ifc` instantiated
  `IfcBSplineCurve` and `IfcBSplineSurface`, both `ABSTRACT SUPERTYPE` in IFC4
  and confirmed invalid by `ifcopenshell.validate`. Moved to
  `invalid_abstract_base_splines.ifc` so the valid fixture is schema-clean and
  usable as validation ground truth; the lowering test that needs them now
  reads the invalid file.
- The bundled IFC4 schema artifact was missing attributes on 124 entities.
  `LIST [1:?] OF UNIQUE X` contains the token `UNIQUE`, which the EXPRESS
  parser read as the start of a `UNIQUE` block, truncating the attribute list
  at that point. `IfcTypeProduct` lost `RepresentationMaps` and `Tag`, so every
  product type reported `ElementType` and `PredefinedType` two slots early.
  Also affected `IfcGrid`, `IfcPolyLoop`, `IfcEdgeLoop`, `IfcPropertyEnumeration`
  and `IfcPropertyTableValue`. Fixed in `openbim-step 0.4.0`; the artifact is
  regenerated and pinned by a test against known-correct layouts.
- `ifc-schedule` implements every plan task: work plans and schedules, tasks
  with task times, sequencing with signed lag, work calendars with recurrence,
  events, and deterministic timeline queries.
- `ifc-cost` completes rate/component trees, nesting and control assignment,
  currency agreement, and tree rollups over a validated fixture.
- `ifc-model` gains transactional authoring: `Transaction` stages structural
  edits, validates them against a projected end state, and commits atomically.
  Removing an entity that a surviving entity still references is refused, and
  a transaction opened against a stale model revision will not commit.
- `ifc-properties` completes `PROP-EDIT`: quantity authoring helpers stage onto
  a caller-owned transaction, preserving each quantity's declared measure type.

- `ifc-properties` implements property sets: property sets with
  every value family, quantities, units, templates, occurrence/type precedence,
  and comparison against externally computed measurements. Property values keep
  their declared measure type, so a length stays distinguishable from a count.
  SI prefixes are carried as exact decimal exponents rather than rounded
  factors. `WR21`/`WR22` quantity breaches are reported -- neither is checked by
  `ifcopenshell.validate`.

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
- `ifc-cost` read the wrong attribute slots for `IfcCostItem` and
  `IfcCostSchedule`, skipping `ObjectType` and `PredefinedType`. Identification,
  cost values and quantities were read one or more slots early, so real files
  produced empty or wrong results. The crate's own fixtures encoded the same
  mistake, which is why its tests passed.

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

- Relicensed repository-authored work from MIT to `AGPL-3.0-or-later`; historical releases remain under their published MIT terms, and third-party material retains its own terms.
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