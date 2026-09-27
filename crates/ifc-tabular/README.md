# ifc-tabular

Structured IFC value containers: `IfcTable` with its rows and columns, and
regular and irregular `IfcTimeSeries`, authored and read back with the
schema's WHERE rules checked.

```bash
cargo add ifc-tabular
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`tabular` feature.

- API documentation: [docs.rs/ifc-tabular](https://docs.rs/ifc-tabular)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-tabular)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- Tables and time series carry no domain meaning, and their consumers
  are spread across constraints, cost and resources. Housing them in
  any one of those crates would make the other two reach into it, so
  they live here and are referenced by `EntityId`.
- This is not a bucket for leftovers. An entity belongs here only if it
  is a container of `IfcValue` indexed by position or time.
