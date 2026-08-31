# Inspecting a structural-analysis model

This scenario reads an IFC structural-analysis exchange, inventories analytical members and actions, follows their relationships, and inspects authored loads without invoking a solver.

## What the crate provides

Enable the facade feature:

```toml
openbim-ifc = { version = "0.1", features = ["structural"] }
```

Then build a schema-resolved borrowed view:

```rust
use ifc::{Codec, StepCodec};
use ifc::structural::StructuralView;

let model = StepCodec.read_bytes(&std::fs::read("analysis.ifc")?)?;
let structural = StructuralView::for_model(&model)?;

for id in model.ids_of_type("IFCSTRUCTURALANALYSISMODEL") {
    let analysis = structural.analysis_model(*id)?;
    println!("{:?}: {:?}", analysis.id(), analysis.name()?);

    for item in structural.analysis_items(*id)? {
        println!("assigned analytical object: {:?}", item);
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

`for_model` accepts exactly one canonical IFC2X3, IFC4, or IFC4X3 schema token. Unknown, missing, or ambiguous headers are errors rather than an IFC4 fallback.

## Implemented semantics

The bounded view currently covers:

- `IfcStructuralAnalysisModel`, load groups, and result-group metadata;
- curve/surface analytical members and point/curve/surface connections;
- versioned point and curve/linear/surface/planar actions with compatible
  structural-load references;
- single, linear, planar, and temperature static-load values;
- model assignments, member-to-connection relationships, and activity assignments;
- selected analysis-model and static-load authoring through a caller-owned transaction.

References are checked before being returned. Missing records, dangling IDs,
wrong target types, incompatible action-load families, invalid assignment-select
members, self-referencing groups, duplicate `SET` members, multiply attached
structural activities, malformed aggregates, and required-value omissions
produce typed `StructuralError` values. Relationship traversal preserves
relation-record file order and each aggregate's declared member order. Duplicate
`SET` links and non-finite load drafts are rejected before transaction staging.

Version drift is resolved by attribute name against the selected schema. Examples include IFC4X3 `AxisDirection` versus earlier `Axis`, IFC2X3 temperature names with underscores, IFC2X3 action fields, and IFC4+ `SharedPlacement`.

## What remains application work

This crate does **not** calculate structural behaviour. An application must still provide or integrate:

- geometry/section-property extraction and coordinate transforms;
- FEM or other discretization;
- material constitutive models, stiffness assembly, load combinations, and solving;
- computed displacement, force, stress, reaction, or code-check results;
- result authoring beyond the currently exposed result-group metadata.

Those capabilities must not be inferred from the presence of `IfcStructural*` records.

## Why this is not `ifc-resource`

IFC construction resources describe labour, equipment, material/product capacity, consumption, time, cost, and allocation to construction processes. Structural-analysis entities describe idealized mechanics and analysis topology. A physical beam may have an analytical-member representation and require resources to fabricate or install, but those are distinct semantic relationships and remain separate crate boundaries.

## Evidence

The contract is exercised by `ifc-structural/tests/`: cross-version layouts, strict references, action/load groups, relationship traversal, rejected authoring, atomic commit, and STEP write/read round-trip.
