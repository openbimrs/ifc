# Roadmap

This page names the directions the project is heading in and links the issues
that track them. It deliberately carries no task lists or counts: open work
lives in [GitHub issues](https://github.com/openbimrs/ifc/issues), what exists
today is on the [capability matrix](/capabilities) and the
[coverage page](/coverage), and what changed is in the
[changelog](/project/changelog).

Milestones group the issues a release depends on:

- [v0.1: First usable release](https://github.com/openbimrs/ifc/milestone/2)
- [v1.0: Stable public API](https://github.com/openbimrs/ifc/milestone/3)
- [Backlog: accepted, unscheduled](https://github.com/openbimrs/ifc/milestone/1)

Umbrella issues carry the
[`tracking`](https://github.com/openbimrs/ifc/labels/tracking) label; accepted
capabilities the
[`capability`](https://github.com/openbimrs/ifc/labels/capability) label.

## Harden authoring and codecs

Authoring by attribute name, transactional edits, STEP and ifcXML are
implemented. What remains is making them dependable for applications that write
files others will read: schema-checked updates of existing entities,
deterministic STEP output across scalar edge cases, and strict ifcXML namespace
profiles with path-rich diagnostics.
Tracked in [#7](https://github.com/openbimrs/ifc/issues/7).

## Exact geometry interpretation

Every IFC4 representation item is addressed (see
[coverage](/coverage#representation-items)), but several are lowered only in
part and a few are refused with a typed reason. The goal is exact neutral
geometry for all of them, without approximation or silent substitution.
Tracked in [#19](https://github.com/openbimrs/ifc/issues/19).

Deriving plan drawings from solids needs a plane section, which is a kernel
capability: it belongs in [Axiolid](https://axiolid.github.io/kernel/), not
here.

## Georeferencing and alignment

`ifc-georef` and `ifc-alignment` are partial by design: their version profiles
are pinned before their coverage grows.
Tracked in [#8](https://github.com/openbimrs/ifc/issues/8).

## More schema versions

IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2 are bundled for validation; domain
views state per crate which releases they read. IFC4X1 and IFC4X2
([#33](https://github.com/openbimrs/ifc/issues/33)) and the scope of
IFC5/IFCX ([#35](https://github.com/openbimrs/ifc/issues/35)) are open
questions. Domain crates that read one release only are being bound to the
model's declared release one by one.

## Bindings and packaging

JavaScript, Python and C bindings ship from this repository (see
[install](/guide/install)). Broader platform coverage, a tested browser build
and CMake packaging are tracked in
[#34](https://github.com/openbimrs/ifc/issues/34).

## Scale

Lazy reads keep large files cheap, and the benchmark suite measures parsing and
lookup. Memory and lookup baselines for the entity graph are tracked in
[#14](https://github.com/openbimrs/ifc/issues/14); a cross-backend geometry
benchmark in [#31](https://github.com/openbimrs/ifc/issues/31).

## Proposing work

Ideas and unresolved ownership questions start in
[Discussions](https://github.com/openbimrs/ifc/discussions). Concrete bugs and
feature requests use the issue forms. Priority among unfinished work is
demand-driven: an issue that describes a real use case moves faster. See the
[contributing guide](/guide/contributing).
