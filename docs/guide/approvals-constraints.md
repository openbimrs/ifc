# Approvals, constraints, and external references

These three bounded IFC4 domains compose through the shared entity graph:

- [`ifc-classification`](https://github.com/openbimrs/ifc/tree/main/ifc-classification) owns concrete
  classification/document/library references and
  `IfcExternalReferenceRelationship`;
- [`ifc-approval`](https://github.com/openbimrs/ifc/tree/main/ifc-approval) owns `IfcApproval`, direct
  approval relationships, resource relationships, and rooted object
  associations;
- [`ifc-constraint`](https://github.com/openbimrs/ifc/tree/main/ifc-constraint) owns concrete metrics and
  objectives plus resource and rooted object associations.

The facade keeps each domain optional:

```toml
[dependencies]
openbim-ifc = {
  git = "https://github.com/openbimrs/ifc.git",
  features = ["step", "classification", "approval", "constraint"]
}
```

## One graph, stable IDs

Sibling domain crates do not depend on each other. Each projection borrows the
same `Model`, and relationship endpoints remain ordinary `EntityId` values:

```rust
use ifc::approval::ApprovalView;
use ifc::classification::ClassificationView;
use ifc::constraint::ConstraintView;

let approval = ApprovalView::new(&model).approval(approval_id)?;
let metric = ConstraintView::new(&model).metric(metric_id)?;
let evidence = ClassificationView::new(&model)
    .external_reference_relationship(evidence_relationship_id)?;

assert_eq!(approval.id(), approval_id);
assert_eq!(metric.id(), metric_id);
assert_eq!(evidence.related_resources()?, vec![approval_id]);
```

The executable counterpart in
[`openbim-ifc/tests/resource_domains.rs`](https://github.com/openbimrs/ifc/blob/main/openbim-ifc/tests/resource_domains.rs)
authors all three domains in one transaction and repeats these assertions after
a STEP write/read round trip.

## Strict projections

The views validate only their declared bounded semantics and return typed errors
for malformed records:

- exact IFC4 inherited-first slot layouts;
- required and optional scalar/reference/aggregate shapes;
- duplicate members in EXPRESS `SET` values;
- dangling or wrong-kind references;
- `IfcResourceObjectSelect`, `IfcDefinitionSelect`, actor, and metric-value
  membership through bundled schema metadata;
- approval identifier/name and constraint user-defined WHERE rules;
- compressed GlobalIds on rooted associations.

Metric values are preserved as either entity references or explicitly typed
values. The constraint crate does **not** evaluate the value, formula, table,
time series, or reference path.

## Transaction-staged authoring

Domain functions take a caller-owned `ifc_model::Transaction`. They validate the
complete draft—including earlier records staged in the same transaction—before
their first edit. A rejected draft leaves transaction length unchanged; the
caller chooses whether and when to commit.

This is authoring, not business authorization. An `IfcApproval` status is stored
and projected but is not interpreted as a signature, permission, workflow
transition, or policy decision. Likewise, authored constraints are data, not a
compliance verdict.
