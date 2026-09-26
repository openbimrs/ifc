# Contributing

## Choose work before coding

The engineering backlog is the repository's
[GitHub issues](https://github.com/openbimrs/ifc/issues). Start from the
[Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view, choose an unassigned issue, and comment before beginning a substantial
change. Every promoted issue names its scope and the proof needed to complete
it.

Use
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

The gate has four sections, `lint`, `test`, `features` and `bindings`.
`scripts/gate.sh test` runs one section; no argument runs them all. CI runs
each section as its own job, in parallel, and the `Standalone IFC gate` check
passes only when all four do.

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

**Code in the docs is a test.** Every Rust, JavaScript, Python and C example
on the site is copied from a test that runs in the gate. The test marks the
lines to publish:

```text
// docs:snippet getting-started-read
let model = ifc::read(&bytes)?;
// docs:end
```

and the page names the region the generator fills:

```text
<!-- SNIPPET:getting-started-read -->
<!-- /SNIPPET -->
```

A code fence in one of those languages outside such a region fails the gate, so
an example cannot be pasted in by hand. Python tests use `#` instead of `//`.

**Facts are generated, not typed.** Crate counts, versions, install commands,
the crate reference, the binding API tables, the coverage tables, the
capability census and the changelog are all produced from the source:

```bash
cargo run -p xtask -- docs          # regenerate every generated file and region
cargo run -p xtask -- docs --check  # the gate: fail if any is out of date
```

Generated regions sit between `<!-- NAME:BEGIN -->` and `<!-- NAME:END -->`;
edit the source, never the region. A number in prose comes from
`docs/.vitepress/data/facts.json` (`{{ facts.crates.total }}`), not from the
author's memory. The gate also rejects git dependencies, pinned versions in
TOML examples, spelled-out crate counts and absolute home paths anywhere on the
site.

**Claims need evidence.** Do not describe a module as supporting something
because it is named after it. A capability claim on the
[capability matrix](/capabilities) must point at executable behaviour with a
test. If it is reserved structure, it is `Scaffold`.

**Scaffold modules stay honest.** A placeholder module states `Planned owner:`
on its first doc line and stays crate-private until it owns a tested public
contract. See [ADR 0005](/adr/0005-scaffold-modules-declare-ownership).

## Context files

**AGENTS.md** is stable ambient context — purpose, boundaries, invariants, gates.
The files are nested so that an agent reads only the files on the path to its
target, and a deeper file never repeats its parent.

Open work is a GitHub issue, never a checklist in the repository. A marker left
in code names its issue, `TODO(#123)`; `cargo run -p xtask -- todo --check`
fails the gate on one that does not, and `cargo run -p xtask -- todo` lists them
all so a closed issue's leftovers can be found. The proof of finished work goes
in the pull request. A decision that should outlive it goes into the owning
**AGENTS.md**, the module docs, or an ADR. Progress logs do not belong in
**AGENTS.md**.

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

## Developing against an unreleased Axiolid

When a capability is on Axiolid's `main` but not yet released, develop against
a local checkout without changing a tracked file. `Cargo.toml` always names
published versions: a `path` or `git` dependency builds only on the machine that
has it, and the architecture gate
(`the_kernel_is_consumed_as_a_published_release`) rejects it.

Redirect the published crates instead, in the gitignored `.cargo/config.toml`:

```toml
[patch.crates-io]
axiolid-core = { path = "../axiolid/kernel/crates/foundation/core" }
# one line per axiolid crate the workspace pins
```

CI resolves from crates.io and ignores the patch; returning to the registry is
deleting `.cargo/`. Two pitfalls:

- Local tests passing does not prove the commit builds in CI, where the kernel
  is older. A change that needs an unreleased capability waits for the release.
- A patch applies only when its version satisfies the pin. After a kernel
  version bump every entry silently goes inert and cargo only warns. Check the
  patch is live: `cargo metadata --format-version 1 2>&1 >/dev/null | grep "was not used"`
  prints nothing when every entry applied.

Never commit a `Cargo.lock` that records the local paths.

## Releasing a crate

Every crate is versioned independently. Touching `ifc-geometry` means
releasing `ifc-geometry` -- not every other crate in the workspace.

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
the tag is on `main`, waits for CI on the tagged commit and requires it to
have passed, and publishes to every registry the crate targets. The gate
runs once, in CI; the release does not run it again:

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

Publishing runs in the `release` environment. Only release tags
(`*-v*`) and `main` (for rehearsals) may deploy to it, and only admins
may create, move or delete release tags.

- npm, PyPI and crates.io: trusted publishing (OIDC), no long-lived
  token. Each registry trusts exactly `release.yml` and `release`;
  renaming either breaks publishing until the registry settings are
  changed to match.
- crates.io is configured per crate: in each crate's crates.io settings,
  add a trusted publisher with repository `openbimrs/ifc`, workflow
  `release.yml` and environment `release`. A new crate is configured
  after its first publish. Until every crate is configured, the workflow
  falls back to the `CARGO_REGISTRY_TOKEN` secret and warns; delete the
  secret once the warning stops appearing.
- npm: the first version of a new scoped package must be published by hand
  (npm cannot attach a trusted publisher to a package that does not exist
  yet); every later version comes from `release.yml`.

If CI cannot publish, `--publish --local` runs `cargo publish` from a
detached worktree at the tag instead, so the embedded VCS hash stays
deterministic. A crate already live at its committed version is a
no-op, so a rate-limited run can be repeated safely.

### Changelogs

Each crate owns a `CHANGELOG.md` next to its `Cargo.toml`. The
documentation page is assembled from all of them by
`cargo run -p xtask -- docs`; never edit `docs/project/changelog.md`
directly. The root `CHANGELOG.md` is a frozen archive of the
lockstep era through 0.2.0 and takes no new entries.

The gate fails if a publishable crate has no changelog, which is what
stops a newly added crate from escaping this process.

## Where help is useful

The [roadmap](/project/roadmap) explains product direction and shipped evidence.
The public Project's
[Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view is the authoritative shortlist of independently assignable work. Issues
not shown there may be blocked, too broad, or awaiting an ownership decision.

Good contributions often include one of:

- a narrow model, codec, or authoring hardening task with mutation-sensitive tests;
- a redistributable IFC fixture and the test or guide that consumes it;
- an evidence-backed schema/version inventory that unblocks a domain slice;
- documentation that distinguishes shipped behavior from planned architecture.

## Public API documentation

Every public item carries a doc comment. Every crate opts into
`[workspace.lints]`, where `missing_docs = "deny"` makes an undocumented public
item a build failure, and `scripts/check-missing-docs.py` fails the gate if a
library crate stops opting in. A new crate adds `[lints] workspace = true` to
its `Cargo.toml` from the start.

The crate-level docs (`//!` in `src/lib.rs`) are also the overview on the
crate's [reference page](/reference/), so write them for a reader choosing
whether to depend on the crate.
