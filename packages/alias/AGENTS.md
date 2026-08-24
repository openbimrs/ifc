# AGENTS.md — packages/alias/

Alias crates. Each is a **pure `pub use` re-export** of one canonical
`openbim-*` crate, published so the standard is reachable under the short name
practitioners actually use.

| Alias | Canonical | Standard |
| --- | --- | --- |
| `icdd` | `openbim-icdd` | ISO 21597 |
| `idmxml` | `openbim-idm` | ISO 29481-3 |
| `loin` | `openbim-loin` | ISO 7817-3 / EN 17412-3 |

## 🚨 An alias must never define a type

This is the entire contract. If an alias crate declares its own `struct`,
`enum` or `trait`, then a dependency graph containing both the alias and its
canonical crate holds **two structurally identical but distinct types**, and no
Cargo version resolution can unify them. The error is
`expected icdd::Container, found openbim_icdd::Container` — worse than a version
conflict, because nothing resolves it.

So each `lib.rs` is exactly one `pub use` line, and stays that way.

## Why the `=` version pin

```toml
openbim-icdd = { path = "...", version = "=0.1.0" }
```

A caret range would let the alias resolve to a *different* version of the
canonical crate than the one a consumer already depends on — reintroducing the
duplicate-type problem the alias exists to avoid. Pin exactly, and bump both
together.

## Why these three and not the others

`ids`, `bcf`, `clash`, `diff` and `dt` are all taken on crates.io by unrelated
projects (verified 2026-08-24). The question is moot for them.

`openbim` itself is **not** an alias — it is the real facade crate, published
under its own name.

## Publishing order

Canonical first, alias second. `cargo publish` verifies dependencies against
the registry, so `openbim-icdd` must already exist there before `icdd` can be
published. Path dependencies alone do not satisfy it.
