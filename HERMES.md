# HERMES.md — nehirde

Root brief. Read this once per session; it stays short by design. Nested
`AGENTS.md` files under each subtree carry the detail for that subtree — read
the one for the directory you're about to touch, not this file again.

## What this is

The best IFC library in Rust — a **lightweight, high-performance alternative to
IfcOpenShell**, which is capable but couples IFC to OpenCascade, a very heavy
C++ geometry dependency. Applications are meant to be built on top of this.

Two package groups, each holding crates:

- **`geom/`** — shared geometry kernel (CSG, meshes, spatial queries). Knows
  nothing about IFC; other formats can use it. Hardware-abstracted from the
  start: scalar / SIMD / optional GPU behind one contract.
- **`ifc/`** — pure IFC logic. Depends on the geom **contract** (`geom-kernel`
  traits), never on a geometry backend, so the whole geometry kernel can be
  swapped for a better one without touching the IFC layer.

That swap boundary is enforced by a test, not by convention — see
`ifc/AGENTS.md`.

Read `docs/adr/0001` (the split), `0002` (hardware abstraction), `0003`
(pure-Rust boolean, the decision the project rests on).

## Layout

- `geom/` — geometry kernel crates: `core`, `kernel`, `cpu`, `simd`, `gpu`,
  `dispatch`. See `geom/AGENTS.md`.
- `ifc/` — IFC crates: `schema`, `parser`, `model`, `shape`. See `ifc/AGENTS.md`.
- `references/` — **symlinks only**, real trees live on `/mnt/backup/` (see
  `references/AGENTS.md`). Read-only design evidence: IfcOpenShell (LGPL-3.0)
  and ifc-lite (MPL-2.0). Never a build dependency — no crate may
  `include!`/vendor from here.
- `test/fixtures/` — small `.ifc` edge-case files pulled from those reference
  repos for use in crate tests. See `test/fixtures/AGENTS.md`.
- `docs/` — `ROADMAP.md`, `adr/` (architecture decisions), `CHANGELOG.md`. See
  `docs/AGENTS.md`. Rustdoc API reference is generated later via `cargo doc`,
  not hand-written here.
- `target/` — symlink to `/mnt/backup/build-cache/nehirde-target`. Root disk
  (`/`) is sparse; build artifacts go on `/mnt/backup` next to
  `../vendor/solibri`'s `target -> /mnt/backup/build-cache/solibri-target`.
  **`/mnt/dev` does not exist on bbv-dev** (no spare block device — checked
  `lsblk`: only `sda`=root, `sdb1`=archive, `sdc1`=backup). If a real `/mnt/dev`
  disk is attached later, `rsync` the target dir over and re-point the symlink;
  don't block on it now.

## Related repos on this host

- `~/projects/vendor/solibri` — sibling Rust Solibri/openBIM engine
  (`docs/PROVENANCE.md`, `AGENTS.md` there). Independent project; consult it
  for prior art (STEP parser, `.smc` reader, rule framework) but do not import
  from it directly — copy the *idea*, cite the source.
- `~/projects/poing` — pnpm/uv monorepo, BIM viewer + apps. Its
  `docs/adr/_template.md` and `docs/changelog.md` (Keep a Changelog format) are
  the house convention this repo follows for `docs/adr/` and `docs/CHANGELOG.md`.

## Commands (once crates exist)

```bash
export PATH="$HOME/.cargo/bin:$PATH"   # cargo/rustfmt aren't on the minimal PATH
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

## Git

Hot on `master`. No feature branches. Conventional Commits
(`feat`/`fix`/`docs`/`chore`/`refactor`/`perf`). One logical change = one
commit. Never commit vendor IP, `.ifc` files outside `test/fixtures/` without
checking provenance, or anything under `references/*` besides its `AGENTS.md`.

## Pitfalls discovered

- `cargo`/`rustfmt` live in `~/.cargo/bin`; export `PATH` before running them
  (inherited from `../vendor/solibri`'s AGENTS.md — same host, same issue).
- `.cargo/config.toml` sets `-C target-cpu=native`. Fine on this dev box,
  **wrong for anything published** — it SIGILLs on older CPUs. `geom-simd`'s
  runtime detection is the intended mechanism; remove or gate the flag before
  release (tracked in `docs/ROADMAP.md`).
- The architecture gate in `ifc/shape/tests/no_backend_dependency.rs` strips
  `#` comments before scanning, so a `# NOTE: geom-cpu must not appear` line in
  a manifest does not false-positive.
