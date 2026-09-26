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
python3 scripts/assemble-changelog.py          # regenerate
python3 scripts/assemble-changelog.py --check  # CI: fail if out of date
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

## Releasing a crate

Every crate is versioned independently. Touching `ifc-geometry` means
releasing `ifc-geometry` -- not the other twenty-six.

Start by asking what a release would cost:

```bash
python3 scripts/release-crate.py ifc-geometry --set 0.2.1
```

This reports the crate's dependents and whether the bump is
compatible. Cargo's `^` requirement is what decides:

| Bump | Meaning | Cost |
| --- | --- | --- |
| `0.2.0` to `0.2.1` | compatible | publish that crate alone |
| `0.2.0` to `0.3.0` | breaking | every dependent must be edited and released too |

A dependent declaring `ifc-geometry = "0.2.0"` accepts `0.2.1`
silently, so an additive change reaches consumers on their next
`cargo update` with no republishing anywhere else. It *rejects*
`0.3.0`, which is why a breaking bump cascades.

Write the bump once the cost is acceptable:

```bash
python3 scripts/release-crate.py ifc-geometry --set 0.2.1 --apply
```

That edits the manifest, opens a dated section in the crate's
`CHANGELOG.md`, and -- for a breaking bump only -- lifts the
requirement in each dependent's manifest. Fill in the changelog
entry, run `bash scripts/gate.sh`, and commit.

Then release:

```bash
python3 scripts/release-crate.py ifc-geometry --publish
```

This refuses a dirty tree, then tags `ifc-geometry-v0.2.1` and pushes
the tag. The tag triggers `.github/workflows/release.yml`, which checks
the tag is on `main`, runs the full gate on the tagged commit, and
publishes to every registry the crate targets:

| Crate | Registries |
| --- | --- |
| any crate without `publish = false` | crates.io |
| `openbim-ifc-wasm` | npm only (`@openbim/ifc`) |
| `openbim-ifc-py` | PyPI only (`openbim-ifc`): Linux, macOS and Windows wheels plus an sdist |

The version in `openbim-ifc-wasm/npm/package.json` or
`openbim-ifc-py/pyproject.toml` must equal the crate's. `--set --apply`
bumps it together with `Cargo.toml`, and refuses if the two were already
out of step. The workflow refuses a tag that disagrees with any manifest. Every publish step
skips a version that is already live, so re-running a partly failed
release finishes it.

Credentials live in the `release` environment, the only place publish
jobs can read them:

- crates.io: the `CARGO_REGISTRY_TOKEN` secret.
- npm and PyPI: trusted publishing (OIDC), no token. Both registries
  trust exactly `release.yml` and `release`; renaming either breaks
  publishing until the registry settings are changed to match.

If CI cannot publish, `--publish --local` runs `cargo publish` from a
detached worktree at the tag instead, so the embedded VCS hash stays
deterministic. A crate already live at its committed version is a
no-op, so a rate-limited run can be repeated safely.

### Changelogs

Each crate owns a `CHANGELOG.md` next to its `Cargo.toml`. The
documentation page is assembled from all of them by
`scripts/assemble-changelog.py`; never edit `docs/project/changelog.md`
directly. The root `CHANGELOG.md` is a frozen archive of the
lockstep era through 0.2.0 and takes no new entries.

The gate fails if a publishable crate has no changelog, which is what
stops a newly added crate from escaping this process.

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
