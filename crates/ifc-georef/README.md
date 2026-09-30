# ifc-georef

Georeferencing: resolves `IfcMapConversion`, IFC4X3 `IfcMapConversionScaled`
and length-coordinate `IfcRigidOperation`, projected CRS metadata and true
north into a format-neutral project-to-map transform, and validates each
operation's source context or CRS. A plane-angle rigid operation onto an
IFC4X3 `IfcGeographicCRS` is read as authored, not lowered: no metre
transform expresses a latitude/longitude offset. It does not place products
or reproject coordinates. Site reference elevation is not read yet
([#242](https://github.com/openbimrs/ifc/issues/242)).

```bash
cargo add ifc-georef
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`georef` feature.

- API documentation: [docs.rs/ifc-georef](https://docs.rs/ifc-georef)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-georef)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
