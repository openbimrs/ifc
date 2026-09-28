# ifc-schema

The IFC schema as data: the normative EXPRESS declarations for IFC2X3 TC1,
IFC4 ADD2 TC1, IFC4X1 FINAL, IFC4X2 FINAL and IFC4X3 ADD2 as bundled tables of
entities, supertype chains, attributes and types, instead of generated structs
per entity.

```bash
cargo add ifc-schema
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`schema` feature.

Each release is its own cargo feature (`ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`,
`ifc4x3`), all on by default. A size-sensitive build turns defaults off and
names the releases it ships; `for_version` then refuses the others with the
typed `NotBundled` error.

- API documentation: [docs.rs/ifc-schema](https://docs.rs/ifc-schema)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-schema)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
