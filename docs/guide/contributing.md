# Contributing

## Choose work before coding

The detailed engineering backlog lives in the nearest **PLAN.md**; GitHub owns
public discussion and assignment. Start from the
[Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view, choose an unassigned issue, and comment before beginning a substantial
change. Every promoted issue names one stable plan task ID and the proof needed
to complete it.

Do not create an issue for every unchecked plan item. Use
[Discussions](https://github.com/openbimrs/ifc/discussions) for design questions
and the issue forms for reproducible bugs or concrete proposals. See the root
[contribution policy](https://github.com/openbimrs/ifc/blob/main/CONTRIBUTING.md)
for the complete claim-to-merge workflow.

## Verification gate

One command decides whether a change is acceptable:

```bash
scripts/gate.sh
```

It runs formatting, a workspace build of all targets, the full test suite with
all features, clippy with `-D warnings`, and rustdoc with `-D warnings`. It then
runs the architecture gates and a feature matrix.

::: danger Judge by exit code
The gate decides on **exit codes**. Never summarise a run by piping through
`grep` or `awk` — a pipe hides the exit status and turns a failing suite into a
clean-looking report.
:::

## Architecture gates

These are tests, so architectural rules fail CI rather than relying on review:

| Gate | Enforces |
| --- | --- |
| `ifc-model --test package_architecture` | Dependency tiers; no sibling domain dependencies |
| `ifc-model --test progressive_context` | Every directory an agent may enter has an **AGENTS.md** |
| `ifc-model --test module_reachability` | No orphaned modules |
| `ifc-model --test no_monolithic_files` | File size limits |
| `ifc-geometry --test declaration_manifest` | Schema declaration inventory matches reality |
| `ifc-geometry --test no_backend_dependency` | No CPU/GPU execution provider leaks in |
| `openbim-ifc --test thin_build` | Default features stay thin |
| `openbim-ifc --test docs_examples` | Code in `docs/` compiles and behaves as shown |

## Documentation rules

**Code in the docs is compiled.** Every non-trivial Rust snippet on the
documentation site has a counterpart in `openbim-ifc/tests/docs_examples.rs`.
Changing a documented example means changing that test, and the gate will catch
a mismatch. Documentation that ships uncompiled code drifts silently, and a
coding agent will reproduce the drift.

**The changelog has one source.** `CHANGELOG.md` at the repository root is
canonical. The docs page is generated:

```bash
python3 scripts/sync-changelog.py          # regenerate
python3 scripts/sync-changelog.py --check  # CI: fail if out of date
```

**Claims need evidence.** Do not describe a module as supporting something
because it is named after it. A capability claim on the
[capability matrix](/capabilities) must point at executable behaviour with a
test. If it is reserved structure, it is `Scaffold`.

**Scaffold modules stay honest.** A placeholder module states `Planned owner:`
on its first doc line and stays crate-private until it owns a tested public
contract. See [ADR 0005](/adr/0005-scaffold-modules-declare-ownership).

## Context files

**AGENTS.md** is stable ambient context — purpose, boundaries, invariants, gates.
**PLAN.md** is implementation state. They are nested so that an agent reads only
the files on the path to its target, and a deeper file never repeats its parent.

When finishing a plan item: check it off, record the proof command and its
result, and note follow-up work found along the way. Progress logs and
speculative TODOs do not belong in **AGENTS.md**.

## ADRs

Decisions that constrain future work get a record in `docs/adr/`, using
[`_template.md`](https://github.com/openbimrs/ifc/blob/main/docs/adr/_template.md).
ADRs are immutable once accepted — a reversal is a new record that supersedes
the old one, not an edit to it.

Record decisions already embodied in code. Open questions belong on the
[roadmap](/project/roadmap).

## Standards material

ISO and CEN schema files, specification PDFs, and other licensed standards
material are **never** committed to this repository or shipped in a published
crate. Local reference copies live outside the repository tree.

## Building the docs site

```bash
npm ci
npm run docs:dev      # local preview
npm run docs:build    # production build
```

The site deploys to GitHub Pages from `main` via `.github/workflows/pages.yml`.

## Where help is useful

The [roadmap](/project/roadmap) explains product direction and shipped evidence.
The public Project's
[Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view is the authoritative shortlist of independently assignable work. Tasks not
shown there may be blocked, stale, too broad, or awaiting an ownership decision
even when their **PLAN.md** checkbox remains open.

Good contributions often include one of:

- a narrow model, codec, or authoring hardening task with mutation-sensitive tests;
- a redistributable IFC fixture and the test or guide that consumes it;
- an evidence-backed schema/version inventory that unblocks a domain slice;
- documentation that distinguishes shipped behavior from planned architecture.

## Documentation debt

Every public item carries a doc comment. Thirteen crates enforce this
permanently: their `Cargo.toml` opts into `[workspace.lints]`, where
`missing_docs = "deny"` makes an undocumented public item a build failure.

The remaining crates carry a measured debt and `#![allow(missing_docs)]`.
`scripts/check-missing-docs.py` pins each crate's count so it can only shrink:

- adding an undocumented public item to a debt crate fails the gate;
- adding one to an enforced crate fails the build;
- documenting items and *not* lowering the budget also fails, so the ceiling
  cannot drift into slack. Run the script with `--update` to record the win.

When a crate reaches zero, delete its `#![allow(missing_docs)]`, add
`[lints] workspace = true` to its `Cargo.toml`, and remove it from the
script's budget table. It is then enforced like the rest.

The lint is measured with `--force-warn`, which reports through the crate's own
`allow`. Measuring without it would report zero for every debt crate and the
budget would be meaningless.
