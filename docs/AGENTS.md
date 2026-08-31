# Documentation instructions

Applies to `docs/`. Read the repository [`../AGENTS.md`](../AGENTS.md) first; this file adds only
what is specific to the documentation site. The adjacent PLAN.md is opt-in
implementation state — read it only when picking up site work, not for the
standing rules below.

## What this directory is

A VitePress site published to GitHub Pages at `https://openbimrs.github.io/ifc/`.
It complements locally generated rustdoc without restating it; the workspace
crates are not published on docs.rs yet. Deployment is handled by
`.github/workflows/pages.yml`.

## The rule that matters

**A capability claim must name the file that proves it.** This repository
contains large scaffold module trees that declare ownership of a schema area
without implementing it, so a reader who trusts a module name will be wrong.
Every status in `capabilities.md` uses the vocabulary defined at the top of
that page, and cites evidence.

If you cannot cite a file, the status is `Scaffold` or `Absent`.

## Local workflow

```bash
npm ci
npm run docs:dev      # live preview
npm run docs:build    # what CI runs; dead links fail the build
```

## Gates

- Dead internal links fail `docs:build`.
- `scripts/sync-changelog.py --check` fails if `project/changelog.md` drifts
  from the canonical root `CHANGELOG.md`. Never hand-edit the page; edit
  `CHANGELOG.md` and re-run the script.
- `scripts/check-leakage.py` rejects XSD, PDF, and `references/` payloads from
  the built site. Normative IFC schema material is never published.
- Rust examples shown here are compiled by
  `openbim-ifc/tests/docs_examples.rs`. Add an example there before adding it
  to a page.

## Conventions

- ADRs are immutable once accepted; supersede rather than rewrite. Use
  `adr/_template.md` and register new files in `.vitepress/config.ts`.
- Prefer a table with an evidence column over prose for status.
- British/American spelling is not enforced; internal consistency within a page is.
