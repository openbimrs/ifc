# AGENTS.md — crates/

Cargo workspace members live here, one directory per crate. Currently empty —
this is the scaffold stage.

## Before adding the first crate

- Name crates `nehirde-<domain>` (e.g. `nehirde-core`, `nehirde-ifc-parser`),
  lowercase, hyphenated — mirrors `../vendor/solibri`'s `sol-*` / current
  package convention, adjusted to this project's name.
- Add the new member to the root `Cargo.toml` `[workspace.dependencies]` table
  with a `path = "crates/<name>"` entry as soon as another crate depends on it,
  so version bumps happen in one place.
- Decide layering up front and write it down in this file once >1 crate
  exists (which crate may depend on which — see `../vendor/solibri/AGENTS.md`
  §"Current crate graph" for the shape of that documentation: an ASCII
  dependency diagram plus a stated hard invariant, e.g. "codec never depends on
  rules").
- Each crate gets its own `AGENTS.md` once it has non-obvious internal
  structure (parsing strategy, unsafe blocks, perf-sensitive code) — don't
  write one for a crate that's still a stub.

## Conventions to carry over from day one

- `rust-version`, `edition`, `license`, `authors` come from
  `[workspace.package]` in the root `Cargo.toml` — crates inherit them
  (`version.workspace = true` etc.), don't restate.
- Public API gets rustdoc comments (`///`) as it's written, not retrofitted —
  `cargo doc --workspace --no-deps` is the documentation deliverable per
  `HERMES.md`.
- Any crate reading `.ifc` test fixtures pulls them from `test/fixtures/`
  (relative path from crate root, e.g. `../../test/fixtures/...`), never from
  `references/` (those are read-only design evidence, not test inputs).
