# 0014 — Net geometry is an explicit request; openings are exact booleans

- **Status:** Accepted
- **Date:** 2026-09-24
- **Deciders:** openbimrs contributors
- **Supersedes:** —
- **Amends:** [ADR 0012](/adr/0012-geometry-backends-are-swappable) — adds a second compile entry point with the same backend seam

## Context

A product's Body representation is its **gross** shape. Openings are separate
products (`IfcOpeningElement`, `IfcVoidingFeature`, any
`IfcFeatureElementSubtraction`) related to their host by
`IfcRelVoidsElement`. The file never authors the net result.

Until #44, `compile_product_mesh` compiled only the Body, so every wall came
back solid through its doors. On the local real-model corpus the relation
appears 49 to 775 times per model. No compiled mesh reflected any of them.

Both answers are legitimate. Quantity takeoff is usually gross, with openings
reported separately. Clearances, net areas, window-to-floor ratios and clash
tests through an opening need net. Neither can replace the other.

## Decision

We will keep `compile_product_mesh` **gross and unchanged**, and add a
separate net entry point:

- `lower::lower_product_net` lowers the host Body, then each voiding
  opening's Body through the opening's own placement chain, and appends exact
  `SolidOperation::Boolean { Difference }` nodes. Nothing is evaluated in the
  bridge, per [ADR 0004](/adr/0004-geometry-bridge-not-kernel).
- `compile::compile_product_mesh_net` and `compile_product_mesh_net_with`
  compile that graph. They return `NetMesh { mesh, openings }`, where
  `openings` lists every subtracted opening in ascending id. The `_with` form
  takes any `MeshCompiler`, as ADR 0012 requires.
- `openings_of(model, host)` is a kernel-free read of the relation.
- **Refusal, not fallback.** If any opening cannot be removed, the result is
  `GeometryError::OpeningNotSubtracted { host, opening, cause }`. Returning the
  gross body would hand back geometry that claims to be net and is not.
- **Multi-item bodies are split into solid parts.** The neutral graph accepts
  only solids as boolean operands, while a multi-item Body lowers to a
  `Collection` (or an `Instance` of one, for a mapped item). Both host and
  opening are flattened to their solid parts, with instance transforms carried
  onto each part. Each host part is cut on its own, since
  `(A ∪ B) − O = (A − O) ∪ (B − O)`.
- **Order is content-defined.** Openings are subtracted in ascending opening
  id, and an opening named by two relations is subtracted once.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Make `compile_product_mesh` net by default | Silently changes every existing caller's volumes, and gross is the right answer for takeoff. |
| A `bool` / options flag on the existing function | Hides a different return contract (the subtracted-opening list, a new error) behind a flag. Two named functions are clearer to callers, most of them agents. |
| Skip an opening that fails, return the rest | The mesh would claim to be net while still solid through that opening. This is the failure #44 was filed to remove. |
| Evaluate the subtraction in the bridge | Violates ADR 0004; the backend decides how booleans are computed. |
| Union all opening parts into one tool first | Adds a union the file never authored and a second failure mode for no benefit. Sequential differences give the same region. |

## Consequences

**Positive**

- Net geometry is reachable with the same backend seam as gross.
- Every refusal names the opening at fault. On a backend refusal, the host is
  compiled alone first, so a host that cannot mesh even gross is blamed as the
  host rather than on its first opening.
- The success path compiles the net graph once.

**Negative / costs**

- A refusal pays a diagnostic pass: the host alone, then each chain prefix
  until one fails. That cost only lands on calls that already failed.
- `voidings_of` scans all relations per call. Netting every product of a
  model is O(products × relations). That is fine at the measured hundreds of
  relations; an index is the fix if it ever shows up in a profile.
- Net quality is bounded by the backend's boolean on real exporter data.
  Non-manifold operands are refused by the reference kernel, and those
  refusals surface as `OpeningNotSubtracted`. On the local corpus that is 14
  of 423 hosts once axiolid/kernel#166 is released.

**Follow-ups / risks to watch**

- `IfcRelProjectsElement` (additions) is the mirror case and is not handled.
- `axiolid-construct` 0.3.0 winds a downward extrusion inside-out, which is
  how many exporters author opening bodies. Fixed in the kernel (#166); until
  a release carries it, those subtractions are refused, never inverted.

## Relation to existing code

- `ifc-geometry/src/input/openings.rs` — kernel-free relation read; slot
  constants asserted in `tests/context_slots.rs` (ADR 0008 pattern).
- `ifc-geometry/src/lower/net.rs` — graph construction and body splitting.
- `ifc-geometry/src/lower/session.rs` — `NodeShape` side table recorded at push.
- `ifc-geometry/src/compile.rs` — net entry points and refusal attribution.
- Tests: `tests/opening_subtraction_compile.rs` (analytic volumes),
  `tests/opening_attribution.rs` (backend refusals named precisely).
