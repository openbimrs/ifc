# Differential benchmarks

## vs IfcOpenShell

Compares our pipeline against IfcOpenShell on the committed fixture corpus:
per-product volume, edge-manifoldness, and timing.

### Regenerating

```sh
# 1. Reference (needs `pip install ifcopenshell numpy`).
python3 tools/differential/reference.py test/fixtures/ifclite-geometry/*.ifc > /tmp/ref.jsonl

# 2. Ours.
cargo run -q -p ifc-cli --release -- differential test/fixtures/ifclite-geometry/*.ifc > /tmp/ours.jsonl

# 3. Join.
python3 tools/differential/compare.py /tmp/ours.jsonl /tmp/ref.jsonl > docs/benchmarks/differential-ifcopenshell.md
```

Both sides emit the same JSON schema, so the comparison is a join on
`(file, entity id)` rather than an eyeball diff.

### Reading the table

- **rel.diff** is relative, not absolute: both paths sum a differently ordered
  triangle list, so bitwise equality is the wrong test. Agreement threshold is
  `1e-9` relative, or `1e-12` absolute for near-zero volumes.
- **timing is not a speedup.** Our figure is per-file compilation amortised
  over products; IfcOpenShell's is per-product `create_shape`. Different units
  of work.

### Pitfalls found the hard way

1. **IfcOpenShell numpy views alias freed C++ memory.** `get_vertices` and
   `get_faces` return views owned by the shape object. If the shape drops
   while the views live, the index buffer reads freed memory and yields
   out-of-range triangles — silently, since numpy will not complain until
   you index with them. Copy with `np.array(..., copy=True)` immediately.
   The harness also asserts `F.max() < len(V)` and reports a corrupt record
   rather than trusting the buffer.

2. **The divergence-theorem volume formula is unusable on survey
   coordinates.** It sums `a . (b x c)`, whose terms scale with the CUBE of
   the distance to the origin while the volume does not. At ~1.5e6 the terms
   reach 1e19 and a cubic-metre answer is reconstructed from differences
   sixteen digits down: a 0.125 m^3 cube measures 0.115. Both sides centre
   on the mesh centroid before summing.

### Known disagreements

- `shared_point_faceted_brep.ifc` (12 products, ~1.2e-2): both sides report
  NEGATIVE volume, so the fixture itself is inside-out. The residual is
  tessellation density on a curved B-rep, not a topology difference.
- `issue_1985_scaled_kinds.ifc` (2 pipe segments): swept-disk solids, where
  the disagreement is chord-flattening density on the swept profile.
- 6 products appear only in the reference: families our lowering reports as
  unsupported (B-rep, swept disk). Tracked as lowering gaps, not silent
  failures — `ifc mesh` reports them as `not lowered`.
