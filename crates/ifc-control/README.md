# ifc-control

Bounded IFC control semantics: permits, project orders, action requests and
performance history, staged into an `ifc-model` transaction and validated
against the schema, together with the `IfcRelAssignsToControl` relationships
those controls own. `read_control` and `read_controls` read them back as
borrowed views, by attribute name in the model's declared release (IFC2X3,
IFC4, IFC4X3), so everything the crate writes, assignments included, can be
read again.

```bash
cargo add ifc-control
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`control` feature.

- API documentation: [docs.rs/ifc-control](https://docs.rs/ifc-control)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-control)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
