# ifc-approval

Bounded approval semantics: `IfcApproval`, its resource-level
relationships and `IfcRelAssociatesApproval`, as borrowed views over
`ifc-model` plus transaction-staged authoring, read and written by
attribute name in the model's declared release (IFC2X3, IFC4 or IFC4X3).
Approval status is an authored fact; the crate implements no workflow,
signatures or authorization.

```bash
cargo add ifc-approval
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`approval` feature.

- API documentation: [docs.rs/ifc-approval](https://docs.rs/ifc-approval)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-approval)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
