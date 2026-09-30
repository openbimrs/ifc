# ifc-georef

Georeferencing: resolves `IfcMapConversion`, projected CRS metadata and true
north into a format-neutral project-to-map transform. It does not place
products or reproject coordinates. IFC4X3 rigid operations and scaled map
conversions are refused with a typed error for now ([#241](https://github.com/openbimrs/ifc/issues/241)). `IfcSite`
reference latitude, longitude and elevation are read for IFC2X3, IFC4 and
IFC4X3, and a site elevation that disagrees with the map conversion's
orthogonal height is reported, never silently resolved.

```bash
cargo add ifc-georef
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`georef` feature.

- API documentation: [docs.rs/ifc-georef](https://docs.rs/ifc-georef)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-georef)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
