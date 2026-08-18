# AGENTS.md — references/

**These entries are symlinks, not real directories.** The actual clones live on
`/mnt/backup/references/` (bulk storage; root disk on bbv-dev is sparse). Do
not `git add` anything under here except this file and `README.md` — see the
root `.gitignore` (`references/*` ignored, `!references/AGENTS.md` /
`!references/README.md` un-ignored).

## What's here

| Symlink | Real path | Upstream | License | Purpose |
|---|---|---|---|---|
| `ifcopenshell` | `/mnt/backup/references/ifcopenshell` | github.com/IfcOpenShell/IfcOpenShell | LGPL-3.0-or-later | Reference C++/Python IFC toolkit + geometry engine. Design evidence for schema handling, validation semantics, geometry edge cases. |
| `ifclite` | `/mnt/backup/references/ifclite` | github.com/LTplus-AG/ifc-lite | MPL-2.0 | Reference Rust IFC processing/geometry crates. Closest prior-art in the same language; its `rust/geometry/tests/fixtures` and `rust/processing/tests/fixtures` are curated edge-case `.ifc` files (mapped items, swept-disk solids, CSG, halfspace flyaway, void-order invariance, etc.) — see `test/fixtures/AGENTS.md` for the subset copied into this repo.

Both were `git clone --depth 1`'d on 2026-08-18; re-clone (don't `pull`, upstream
history isn't needed) if you need a newer snapshot:

```bash
cd /mnt/backup/references
rm -rf ifcopenshell && git clone --depth 1 https://github.com/IfcOpenShell/IfcOpenShell.git ifcopenshell
rm -rf ifclite      && git clone --depth 1 https://github.com/LTplus-AG/ifc-lite.git ifclite
```

## Agent rules

1. **Read-only design evidence, never a build dependency.** No crate under
   `packages/` may `include!`, vendor, or `path = "../references/..."` into
   these trees. If code here is genuinely useful (e.g. schema tables), port the
   *idea* independently and cite the source in a comment — same clean-room
   posture as `../vendor/solibri` (see its `docs/PROVENANCE.md`).
2. **License awareness when porting ideas.** LGPL-3.0 (ifcopenshell) and
   MPL-2.0 (ifclite) both permit reading for understanding; neither permits
   copying source into a differently-licensed crate. When in doubt, re-derive
   from the IFC/STEP spec instead of the reference implementation.
3. **`.ifc` sample/test files found here are fair game to copy** into
   `test/fixtures/` (they're data, not source, and typically minimal repro
   cases the upstream projects themselves ship for testing) — but check the
   specific file's header/license note first; don't copy anything that looks
   like a real client project export.
4. If `/mnt/backup` is ever unmounted or these symlinks go stale, re-run the
   clone commands above — nothing under `packages/` should break, since nothing
   depends on this directory at build time.
