# ifc-validate

Schema conformance for an IFC model: attribute arity and types, cardinality,
GlobalId and reference integrity, and WHERE rules, checked against the
declared release's own tables in a separate pass from parsing. Rules it cannot
evaluate are reported as such, never passed silently.

```bash
cargo add ifc-validate
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`validate` feature.

- API documentation: [docs.rs/ifc-validate](https://docs.rs/ifc-validate)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-validate)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
