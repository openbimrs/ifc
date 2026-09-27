# ifc-cost

Cost semantics as a borrowed view over the IFC model: cost schedules and
items, cost values, quantities, nesting and roll-up, plus transaction-staged
IFC4 cost authoring. The crate owns no data, so a file with cost data
round-trips identically with or without it.

```bash
cargo add ifc-cost
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`cost` feature.

- API documentation: [docs.rs/ifc-cost](https://docs.rs/ifc-cost)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-cost)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
