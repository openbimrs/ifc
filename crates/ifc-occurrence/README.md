# ifc-occurrence

Authoring for the concrete built-element and distribution occurrence classes,
with the schema's occurrence-to-type pairing enforced: a pump typed by a valve
type is refused rather than written. The catalogue is generated from the
IFC4X3 ADD2 schema.

```bash
cargo add ifc-occurrence
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`occurrence` feature.

- API documentation: [docs.rs/ifc-occurrence](https://docs.rs/ifc-occurrence)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-occurrence)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- A class is left out of the catalogue only when another crate authors
  it (the generic distribution classes, which `ifc-systems` owns). Test
  fixtures and doc examples elsewhere never count as authoring a class,
  which is why common classes such as `IfcWall` are generated here.
- The catalogue names the classes; the model's declared release lays each
  record out. `create` and `create_with_owner_history` take slots, the
  `PredefinedType` enumeration and required attributes from that
  release's table, so a class IFC4 lacks is refused in an IFC4 model, and
  IFC2X3, which requires `IfcRoot.OwnerHistory`, needs the owner-history
  variant.
