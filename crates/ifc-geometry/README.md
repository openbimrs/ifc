# ifc-geometry

The IFC side of geometry: typed views over IFC geometry resources,
representation selection, placements and units, and lowering of representation
items into the format-neutral Axiolid geometry DAG. It does not triangulate or
execute booleans; the opt-in `compile` features hand the lowered DAG to a
swappable backend.

Products along an alignment are placed from a cached `CartesianPosition`, or,
with the `compile` feature, by deriving their `IfcLinearPlacement` through a
`CurveEvaluator` the caller injects (`LoweringSession::with_curve_evaluator`),
which also checks a cached position against the derivation; either is placed
in the frame of the alignment that carries the basis curve. Products on a
grid are placed at the intersection of their grid axes, curved axes through
the same evaluator, and any placement may be relative to a linear or grid
placement. Net geometry
(`lower::lower_product_net`) subtracts every opening's Body, and can take an
IFC4 Reference View opening that has only a `Reference` representation as
already applied (`lower_product_net_with`).

```bash
cargo add ifc-geometry
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`geometry` feature.

- API documentation: [docs.rs/ifc-geometry](https://docs.rs/ifc-geometry)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-geometry)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
