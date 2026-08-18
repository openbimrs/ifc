# Roadmap

Validation-gated stages. No stage is "done" on a claim — every stage needs
(a) a cross-check against `test/fixtures/` (or a larger oracle corpus once one
exists) and (b) a measured wall-clock wherever a performance claim is made.

Hardware (bbv-dev): Intel Xeon w7-3565X, 20 vCPU, AVX-512
(f/dq/bw/vl/vbmi/vbmi2/ifma/cd/vnni/bf16) + AMX (tile/int8/bf16), 62 GB RAM.
Same box as `../vendor/solibri` — see that project's `docs/ROADMAP.md` for
prior art on exploiting it (native `target-cpu`, mimalloc under
high-thread-count allocation, parallel STEP partitioning/resolve).

## Stage 0 — Scaffold ✅ DONE (2026-08-18)

- [x] Repo skeleton: `crates/` (empty, workspace `members = ["crates/*"]`),
      `docs/` (`ROADMAP.md`, `adr/`, `CHANGELOG.md`), `references/` (symlinks
      to IfcOpenShell + ifc-lite clones on `/mnt/backup/`), `test/fixtures/`
      (18 edge-case `.ifc` files from those two repos).
- [x] `target` symlinked to `/mnt/backup/build-cache/nehirde-target` (sparse
      root disk; same pattern as `../vendor/solibri`'s target).
- [x] Progressive agent context: root `HERMES.md` + nested `AGENTS.md` per
      subtree (`crates/`, `references/`, `test/fixtures/`, `docs/`).
- [ ] First crate. Nothing compiles yet — `crates/*` glob is empty.

## Stage 1 — TBD

Not yet planned. Candidates to evaluate against `../vendor/solibri`'s existing
Stage 0/1 (STEP/IFC parsing, schema-as-data, geometry kernel) before deciding
scope here — this project's relationship to that sibling repo needs an ADR
before Stage 1 starts (are we building something solibri-rs doesn't have, or
exploring an alternative design for the same problem?).
