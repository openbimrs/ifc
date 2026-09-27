# ifc-georef

Georeferencing: resolves `IfcMapConversion`, rigid operations, projected CRS
metadata, true north and site elevation into a format-neutral project-to-map
transform. It does not place products or reproject coordinates.

```bash
cargo add ifc-georef
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`georef` feature.

- API documentation: [docs.rs/ifc-georef](https://docs.rs/ifc-georef)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-georef)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
