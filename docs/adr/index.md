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
| [0008](/adr/0008-fixed-slot-constants-for-stable-relationships) | Fixed slot constants for stable relationships | Accepted |
| [0009](/adr/0009-derived-attributes-resolve-through-the-parent-context) | DERIVED attributes resolve through the parent context | Accepted |
| [0010](/adr/0010-checked-mutation-is-a-model-level-primitive) | Checked mutation is a model-level primitive, not a bare accessor | Accepted |
| [0011](/adr/0011-geometry-authoring-is-bidirectional-in-the-bridge) | Geometry authoring is bidirectional inside the bridge | Accepted |
| [0012](/adr/0012-geometry-backends-are-swappable) | Geometry backends are swappable, whole or per area | Accepted |
| [0013](/adr/0013-language-bindings-wrap-the-facade) | Language bindings wrap the facade, one crate per target | Accepted |
| [0014](/adr/0014-net-geometry-is-an-explicit-request) | Net geometry is an explicit request; openings are exact booleans | Accepted |
| [0015](/adr/0015-strict-step-reads-load-lazily) | Strict STEP reads validate eagerly and decode lazily | Accepted |
