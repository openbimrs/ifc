# ifc-xml

The ifcXML (ISO 10303-28) codec for the IFC model: the same `ifc_model::Codec`
trait as `ifc-step`, over the same model. With a schema it writes conformant
named attributes; without one it falls back to marked positional names, and
both round-trip losslessly.

```bash
cargo add ifc-xml
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`ifcxml` feature.

- API documentation: [docs.rs/ifc-xml](https://docs.rs/ifc-xml)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-xml)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
