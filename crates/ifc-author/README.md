# ifc-author

Schema-checked IFC authoring: build and edit entities by attribute name, with
arity, declared-type, aggregate and GlobalId checks resolved against the
bundled `ifc-schema` tables, and stage the result on an `ifc-model`
transaction.

```bash
cargo add ifc-author
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`author` feature.

- API documentation: [docs.rs/ifc-author](https://docs.rs/ifc-author)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-author)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
