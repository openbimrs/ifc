# synthetic-compile/ — authored here, not upstream

`union_over_halfspace_unbounded.ifc`, derived from
`ifclite-geometry/issue_1155_halfspace_flyaway.ifc` by retyping
`IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,...)` to `IFCBOOLEANRESULT(.UNION.,...)`.
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
