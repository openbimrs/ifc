# ifc-constraint

Bounded IFC4 constraint semantics: `IfcMetric`, `IfcObjective` and their
relationships, as borrowed views over `ifc-model` plus transaction-staged
authoring. Values are preserved as authored; the crate does not evaluate
compliance or formulas.

```bash
cargo add ifc-constraint
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`constraint` feature.

- API documentation: [docs.rs/ifc-constraint](https://docs.rs/ifc-constraint)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-constraint)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
