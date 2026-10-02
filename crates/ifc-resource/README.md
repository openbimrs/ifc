# ifc-resource

Construction resources for IFC4 and IFC4X3: labour, equipment, material, crew
and subcontract resources and their types, actors and inventories, resource
usage and allocation, as schema-resolved borrowed views with
transaction-staged authoring. It schedules, levels and costs nothing.

IFC2X3 TC1 is read, not authored, through its own table: shared concepts
answer the shared accessors (`ResourceIdentifier` and `Id` answer
`identification()`), IFC2X3-only attributes (`ResourceGroup`,
`ResourceConsumption`, `BaseQuantity` as an `IfcMeasureWithUnit`,
`SkillSet`, `Suppliers`, `UsageRatio`, `SubContractor`, `JobDescription`)
have their own accessors, and what IFC2X3 lacks (resource types,
`IfcResourceTime`, `PredefinedType`, `BaseCosts`) is a typed
`ResourceError::NotInSchema`.

```bash
cargo add ifc-resource
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`resource` feature.

- API documentation: [docs.rs/ifc-resource](https://docs.rs/ifc-resource)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-resource)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
