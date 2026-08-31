# Contributing

OpenBIM.rs welcomes bug reports, design feedback, documentation, fixtures, and
code contributions. This repository values narrow changes with executable
evidence over broad, speculative rewrites.

## Find work

Start with the public [Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view. Each promoted issue links to one stable task ID in the owning `PLAN.md`,
states its scope and evidence, and has already been checked for blockers.

Before starting:

1. Search existing issues and pull requests.
2. Comment on the issue you want to take. For large work, wait for assignment so
   two contributors do not solve the same task.
3. Use [Discussions](https://github.com/openbimrs/ifc/discussions) for an idea or
   architecture question that is not implementation-ready.
4. Use the issue forms for a reproducible bug, feature request, or papercut.

Unchecked tasks that have not been promoted to GitHub are the full engineering
backlog, not an implicit invitation to implement them. They may still need a
dependency, design decision, fixture, or licensing check.

## Prepare the repository

Requires Rust 1.88 or newer. Documentation work also requires Node.js and npm.

```bash
git clone https://github.com/openbimrs/ifc.git
cd ifc
```

Read `HERMES.md`, the root `AGENTS.md`, and the nearest nested `AGENTS.md` for
the crate or module you will change. Read its `PLAN.md` when the issue names a
plan task.

## Make the change

- Keep the change within the issue and declared dependency boundaries.
- Add tests for behavior changes and failure cases.
- Keep unknown data round-trippable and failures explicit.
- Do not commit official IFC/ISO/CEN schema files, specification PDFs, or other
  standards material unless redistribution rights have been verified.
- Use reduced, redistributable fixtures rather than confidential building
  models. Record fixture provenance and licensing.

Changes consumed by `openbimrs/openbim` land here first. After the canonical IFC
commit is pushed and green, the integration repository updates its submodule
pin.

## Verify

Run focused tests while iterating, then the repository gate before requesting
review:

```bash
./scripts/gate.sh
```

For documentation changes also run:

```bash
npm ci
npm run docs:build
```

Judge commands by exit status. Do not filter a gate through a pipe that hides a
failure.

## Open the pull request

Open a draft pull request early for large or cross-crate work. The pull request
must:

- use `Closes #<issue>` for the promoted implementation issue;
- name the plan task ID when one exists;
- state important exclusions as well as included scope;
- list exact verification commands and concise results;
- check off the matching `PLAN.md` item and record proof when the task is done;
- update documentation and `CHANGELOG.md` for user-visible behavior.

Review may request additional negative tests, mutation evidence, corpus evidence,
or a narrower scope when the risk justifies it.

## Licensing contributions

Unless an explicitly signed agreement says otherwise, every contribution
submitted to this repository is licensed under `AGPL-3.0-or-later`. Submit only
work that you have the right to license. Identify third-party material and
preserve its license, attribution, and provenance.

Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Security
reports follow [SECURITY.md](SECURITY.md); do not disclose vulnerabilities in a
public issue.
