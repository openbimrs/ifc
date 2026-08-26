# Documentation site implementation plan

Status: active
Last updated: 2026-08-26

Implementation state for the published documentation site. Read the adjacent
AGENTS.md for standing rules. Scope is the site only; library work is tracked
in the repository-root plan and in each crate's own plan.

## Goal

A published site that lets an engineer or coding agent decide what this
repository can do without reading its source, and that cannot silently drift
from the code it describes.

## Work queue

### Wave 0 - site foundation

- [x] `DOC-000` - VitePress site, theme, and GitHub Pages workflow.
      Proof: `npm run docs:build` succeeds; dead-link check is enabled.
- [x] `DOC-001` - capability matrix with per-row evidence and a status
      vocabulary. Proof: `capabilities.md` workspace census.
- [x] `DOC-002` - ADRs 0001-0006 recording decisions already embodied in code.
- [x] `DOC-003` - changelog generated from the canonical root CHANGELOG.md.
      Proof: `python3 scripts/sync-changelog.py --check` in `scripts/gate.sh`.
- [x] `DOC-004` - documented Rust examples compiled by
      `openbim-ifc/tests/docs_examples.rs`. Proof: 4 tests pass; mutation of a
      documented attribute order fails the suite.
- [x] `DOC-005` - standards-material leakage gate on the built site.
      Proof: `python3 scripts/check-leakage.py docs/.vitepress/dist`.

### Wave 1 - follow-on

- [ ] `DOC-006` - per-use-case page for model checking / IDS auditing once the
      relevant crates leave scaffold status.
- [ ] `DOC-007` - publish a worked annotation round-trip fixture alongside the
      2D approval-plan guide when authoring helpers exist.
- [ ] `DOC-008` - link the site from the GitHub About metadata and add the
      repository homepage URL once Pages is enabled.

## Constraints

- A capability claim must cite the file that proves it. Absent evidence, the
  status is Scaffold or Absent.
- ADRs are immutable once accepted; supersede rather than rewrite.
- Normative IFC schema material is never published to the site.
