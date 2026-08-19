# geom-boolmesh instructions

Purpose: adapt the adopted `boolmesh` crate to `geom_kernel::MeshBoolean` (ADR 0014).
This crate owns conversion and contract enforcement; the algorithm is upstream's.

## Module ownership

convert.rs (TriMesh <-> Manifold, orientation gate); provider.rs (the trait impl,
result contract). Split before unrelated concerns grow together.

## Invariants

Orientation is checked on the way IN, per argument, naming which argument failed.
An inside-out mesh is structurally valid and manifold, so nothing else catches it;
`Difference` then behaves as `Union` and returns a LARGER mesh with no error. This
happened for real during the ADR 0014 evaluation.

Input faults are `InvalidInput`/`Degenerate`/`NotManifold` (caller's fault).
Result faults are `BackendContractViolation` (upstream's fault). Never blame the
caller for an upstream defect.

Scratch is `Unbounded`: `boolmesh` exposes no bound, so a caller with a hard
budget is refused rather than silently allowed past it.

Results carry no normals. `boolmesh` computes face normals; re-exporting them as
vertex normals would misrepresent the hard edges a cut creates.

## Verification

Volume conservation (`vol(a\b) + vol(a^b) == vol(a)`) is the gate, not index
comparison: it is triangulation-invariant, so it tests geometry rather than an
output buffer we do not control. Test helpers compute volume independently of the
crate's own helper, or the test would confirm the implementation with itself.

`boolmesh` must not be re-exported. It is MPL-2.0 and swappable; leaking its types
would make the adoption visible to consumers and defeat the seam.
