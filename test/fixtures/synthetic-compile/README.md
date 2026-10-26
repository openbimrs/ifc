# synthetic-compile/ — authored here, not upstream

`union_over_halfspace_unbounded.ifc`, derived from
`ifclite-geometry/issue_1155_halfspace_flyaway.ifc` by retyping
`IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,...)` to `IFCBOOLEANRESULT(.UNION.,...)`.
Its representation type is `CSG` (a union is no `Clipping` result) and the empty
intermediate `IfcLocalPlacement`s of the upstream column were dropped (IFC2X3
requires every placement to place exactly one product), so the file satisfies
every EXPRESS rule of its release (#536).
Not byte-identical to upstream, so it must never move into
`ifclite-geometry/`, whose provenance rule requires exactly that.

Difference and intersection stay inside the finite left operand, so a prism
covering its bounds is an exact stand-in -- which is why `issue_1155`
compiles. Union escapes that bound and the reference compiler refuses.
The file validates clean: the refusal belongs to the mesh provider, not the
file. It keeps the refusal branch of `crates/ifc-geometry/tests/compile_pairing.rs`
executable; a mutation run confirmed that branch is dead code without it.

- vertex_loop_zero_area.ifc: derived from shared_point_faceted_brep.ifc by
  replacing one IfcPolyLoop face bound with an IfcVertexLoop. Proves the
  degenerate zero-area loop reports a typed refusal naming the entity,
  not a wrong-type error that reads as a corrupt file.

- gable_wall_polygonal_clip_ifc2x3.ifc, gable_wall_polygonal_clip_ifc4.ifc:
  hand-written for #515 from the issue's repro (an axioval/editor export,
  names stripped). One 9775 x 350 mm wall extruded 16165.03 mm, clipped by
  four `IfcPolygonalBoundedHalfSpace`s: sloped `IfcPlane`s, each bounded by
  a vertical prism over a 350 mm strip of the wall's top, `AgreementFlag`
  `.T.`. The geometry entities (#8969-#9031) are the repro's, byte for byte;
  the header, owner history (IFC2X3 only), units and context are cut to
  what the schema requires, and `IfcWallStandardCase` became `IfcWall`
  (IFC4's `IfcWallStandardCase` requires a material layer set usage). Both
  are `ifcopenshell.validate --rules` clean. IfcOpenShell 0.8.5 meshes
  each to 14 vertices and 49.999912 m³, the top at 11537.10 mm (x = 0),
  13536.68 mm, 16165.03 mm (the ridge, x = 4925), 13536.68 mm and
  11809.43 mm (x = 9775). Every prism is oblique to its plane; each clip
  lowers to Axiolid's `BoundedHalfSpaceAlongZ` (axiolid/kernel#321), and
  the wall compiles to IfcOpenShell's volume, exactly and as a mesh
  certified against it (`tests/bounded_halfspace_prism_axis.rs`).

- tall_wall_small_bounded_clip_ifc2x3.ifc: synthetic, hand-written for
  #519. Only the clip's geometry parameters are taken from a third-party
  IFC2X3 export (a wall's end clip); every entity, GUID and name is new.
  One 10125 x 350 mm wall extruded 16165.033 mm, minus an
  `IfcPolygonalBoundedHalfSpace`: the horizontal plane z = 11404.747 mm,
  `AgreementFlag` `.F.`, bounded by the 238.549 x 350 mm strip at the
  wall's end, `Position` unrotated on the plane. It removes that strip
  from the plane to the top (4760.286 mm): 56.887388853 m³, matched by
  IfcOpenShell 0.8.5 (12 vertices); `ifcopenshell.validate --rules` clean.
  Both of Axiolid's compilers build that volume, the mesh certified
  (`tests/bounded_halfspace_depth.rs`); up to axiolid-mesh-compile 0.3.26
  the mesh cut the strip only 0.7 m deep (twice the boundary's largest
  coordinate, axiolid/kernel#325).
