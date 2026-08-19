# geom-model instructions

Purpose: Immutable format-neutral geometry DAG.

Allowed internal dependencies: all L1 representation crates. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

id.rs; graph.rs; node.rs; value.rs; validation.rs; curve_relation.rs; surface_relation.rs;
solid_operation.rs. `value.rs` owns sealed built-in-to-node conversions only;
`validation.rs` owns graph-reference family checks; graph ownership stays in `id.rs`/`graph.rs`.
execution/provider traits must remain open in `geom-kernel`. Split a module before
unrelated data, validation, and algorithms grow together. Add no empty
placeholder files.

## Invariants

Every reference points to a prior NodeId. Keep source IDs outside the graph. Preserve instancing and exact operations; never lower to meshes here.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
