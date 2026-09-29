# ifc-cost

Cost semantics as a borrowed view over the IFC model: cost schedules and
items, cost values, quantities, nesting and roll-up, plus transaction-staged
cost authoring. Cost records are read and authored in the model's declared
release (IFC2X3, IFC4 or IFC4X3), laid out by attribute name from that
release's table. The crate owns no data, so a file
with cost data round-trips identically with or without it.

```bash
cargo add ifc-cost
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`cost` feature.

- API documentation: [docs.rs/ifc-cost](https://docs.rs/ifc-cost)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-cost)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
