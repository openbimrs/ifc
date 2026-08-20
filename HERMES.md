# HERMES.md — nehirde

Nehirde is a pure-Rust IFC library. Its format-agnostic geometry kernel is the separate **Axiolid** repository at `../axiolid` (private GitHub publication remains to be configured).

## Dependency direction

```text
Axiolid (external kernel) → packages/ifc → packages/openbim → bindings, apps
```

Only the explicit IFC bridge crates (`ifc-geometry`, `ifc-georef`, `ifc-alignment`) may depend on Axiolid representation crates. No IFC crate may depend on Axiolid algorithms, kernel contracts, or backends; applications choose execution providers.

## Commands

```bash
cargo build --workspace
cargo test --workspace
scripts/gate.sh
```

Run Axiolid's kernel-specific feature/layering and mutation gates from `../axiolid`.

## Git

`master` is shared and hot. Stage narrowly, re-read HEAD, and use a compare-and-swap update when landing a detached-worktree commit.
