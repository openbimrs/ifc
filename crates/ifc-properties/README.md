# ifc-properties

Property sets, quantities, templates and unit resolution as borrowed views
over `ifc-model`, with a permissive API for inspection and an exact,
release-bound API for rule engines. It never computes a shape measurement: an
authored quantity can be compared against a value the caller computed
elsewhere.

```bash
cargo add ifc-properties
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`properties` feature.

- API documentation: [docs.rs/ifc-properties](https://docs.rs/ifc-properties)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-properties)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
