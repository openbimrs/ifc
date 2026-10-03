# ifc-georef

Georeferencing: resolves `IfcMapConversion`, IFC4X3 `IfcMapConversionScaled`
and length-coordinate `IfcRigidOperation`, projected CRS metadata and true
north into a format-neutral project-to-map transform, and validates each
operation's source context or CRS. A plane-angle rigid operation onto an
IFC4X3 `IfcGeographicCRS` is read as authored, not lowered: no metre
transform expresses a latitude/longitude offset. `IfcSite` reference
latitude, longitude and elevation are read for IFC2X3, IFC4 and IFC4X3, and
a site elevation that disagrees with the map conversion's orthogonal height
is reported, never silently resolved. An omitted `IfcProjectedCRS.MapUnit`
resolves to the project length unit, while the unit as authored stays
readable (`ProjectToMap::declared_map_unit`, `None` when unset). It does not
place products or reproject coordinates.

```bash
cargo add ifc-georef
```

The default `transform` feature adds the `axiolid_core::Transform3` view
(`ProjectToMap::transform`, `compose_project_frame`). A semantic consumer
that needs only the parameters (CRS, units, eastings, northings, height,
axis, scale, north, site reference) turns it off and links no geometry
crate:

```bash
cargo add ifc-georef --no-default-features
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`georef` feature.

- API documentation: [docs.rs/ifc-georef](https://docs.rs/ifc-georef)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-georef)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
