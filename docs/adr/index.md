# Architecture decision records

An ADR records a decision that constrains future work: the context that forced
it, the choice made, and the consequences accepted. They are immutable once
accepted — a reversal is a new record that supersedes the old one, not an edit.

Records here describe decisions that are **already embodied in the code**. Where
a decision is still open, it is on the [roadmap](/project/roadmap) instead.

Use [`_template.md`](https://github.com/openbimrs/ifc/blob/main/docs/adr/_template.md)
for new records.

## Index

| # | Title | Status |
| ---: | --- | --- |
| [0001](/adr/0001-entity-graph-free-of-domain-and-codec) | Entity graph free of domain semantics and serialization | Accepted |
| [0002](/adr/0002-codec-as-a-model-crate-trait) | Codec as a trait owned by the model crate | Accepted |
| [0003](/adr/0003-domain-crates-as-borrowed-views) | Domain crates as borrowed views | Accepted |
| [0004](/adr/0004-geometry-bridge-not-kernel) | Geometry bridge, not geometry kernel | Accepted |
| [0005](/adr/0005-scaffold-modules-declare-ownership) | Scaffold modules declare ownership without claiming capability | Accepted |
| [0006](/adr/0006-facade-features-default-to-thin) | Facade features default to thin | Accepted |
| [0007](/adr/0007-authoring-is-a-schema-layer-not-a-model-layer) | Authoring is a schema-layer concern, not a model-layer one | Accepted |
