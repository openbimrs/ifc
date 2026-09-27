# Documentation instructions

Applies to `docs/`. Read the repository [`../AGENTS.md`](../AGENTS.md) first; this file adds only
what is specific to the documentation site.

## What this directory is

A VitePress site published to GitHub Pages at `https://openbimrs.github.io/ifc/`.
It complements rustdoc on docs.rs without restating it: the generated crate
reference links each released crate's docs.rs page. Deployment is handled by
`.github/workflows/pages.yml`.

## The rule that matters

**A capability claim must name the file that proves it.** Scaffold modules may
declare ownership of a schema area without implementing it (ADR 0005), so a
reader who trusts a module name can be wrong.
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
- `cargo run -p xtask -- docs --check` fails when any generated file is stale:
  `project/changelog.md` (from every crate's `CHANGELOG.md`), the generated
  regions of `capabilities.md`, `coverage.md`, `architecture/crates.md`,
  `guide/install.md`, `bindings/*.md` and `adr/index.md`, every page under
  `reference/`, and `.vitepress/data/{facts,adrs}.json`. Never hand-edit them;
  change the source and run `cargo run -p xtask -- docs`.
- Every crate declares `[package.metadata.openbim]` (`status`, `group`) in its
  `Cargo.toml`; the facade also describes each bundle feature there. A new
  crate or feature without them fails the check.
- Prose quotes counts and versions from `.vitepress/data/facts.json`, never as
  literals. The check rejects git dependencies (`git = "…"`), `version = "…"`
  in TOML fences, spelled-out crate counts and absolute `/home/` paths; install
  instructions use `cargo add`, `npm install` or `pip install` without a
  version.
- `scripts/gate.sh` refreshes `.vitepress/data/authored-coverage.json` from its
  own test run and fails if the committed copy differs; commit the refresh.
- `scripts/check-leakage.py` rejects XSD, PDF, and `references/` payloads from
  the built site. Normative IFC schema material is never published.
- Code on a page comes from a test. Mark it in the test with
  `// docs:snippet <name>` … `// docs:end` (e.g. in
  `crates/openbim-ifc/tests/docs_examples.rs`) and put
  `<!-- SNIPPET:<name> -->` `<!-- /SNIPPET -->` on the page; the generator copies
  it in. A `rust`/`python`/`js`/`ts`/`c` fence outside a snippet region fails
  the check (ADRs excepted). Use ```` ```text ```` for output or pseudo-code.

## Conventions

- ADRs are immutable once accepted; supersede rather than rewrite. Use
  `adr/_template.md`; the ADR index and sidebar are generated from the files.
- Prefer a table with an evidence column over prose for status.
- British/American spelling is not enforced; internal consistency within a page is.
