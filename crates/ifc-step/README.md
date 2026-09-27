# ifc-step

The STEP physical file (ISO 10303-21) codec for the IFC model: reads `.ifc`
files into an `ifc-model` graph, eagerly or lazily, and writes them back.
Generic STEP syntax lives in `openbim-step`; this crate owns only the
conversion to and from the IFC model.

```bash
cargo add ifc-step
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`step` feature.

- API documentation: [docs.rs/ifc-step](https://docs.rs/ifc-step)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-step)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
