# ifc-element-type

Element, resource and process type definitions: the concrete `IfcTypeObject`
catalogue, generated from the schema, and a writer that stages each type with
the schema's rules for it checked first.

```bash
cargo add ifc-element-type
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`element-type` feature.

- API documentation: [docs.rs/ifc-element-type](https://docs.rs/ifc-element-type)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-element-type)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- `create_type` and `create_supertype` take no model and write the
  catalogue's IFC4X3 layout. `create_type_in`, `create_supertype_in` and
  their `*_with_owner_history` variants write the model's declared release
  instead, laid out from its own table; IFC2X3, which requires
  `IfcRoot.OwnerHistory`, needs the owner-history variants.
