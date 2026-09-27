# ifc-geometry

The IFC side of geometry: typed views over IFC geometry resources,
representation selection, placements and units, and lowering of representation
items into the format-neutral Axiolid geometry DAG. It does not triangulate or
execute booleans; the opt-in `compile` features hand the lowered DAG to a
swappable backend.

```bash
cargo add ifc-geometry
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`geometry` feature.

- API documentation: [docs.rs/ifc-geometry](https://docs.rs/ifc-geometry)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-geometry)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
