# Contributing

OpenBIM.rs welcomes bug reports, design feedback, documentation, fixtures, and
code contributions. This repository values narrow changes with executable
evidence over broad, speculative rewrites.

## Find work

Start with the public [Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view. Each promoted issue states its scope and the evidence that completes it,
and has already been checked for blockers.

Before starting:

1. Search existing issues and pull requests.
2. Comment on the issue you want to take. For large work, wait for assignment so
   two contributors do not solve the same task.
3. Use [Discussions](https://github.com/openbimrs/ifc/discussions) for an idea or
   architecture question that is not implementation-ready.
4. Use the issue forms for a reproducible bug, feature request, or papercut.

Open issues that have not been promoted to that view are the full engineering
backlog, not an implicit invitation to implement them. They may still need a
dependency, design decision, fixture, or licensing check.

## Prepare the repository

Requires Rust 1.99 or newer. Documentation work also requires Node.js and npm.

```bash
git clone https://github.com/openbimrs/ifc.git
cd ifc
```

Read the root `AGENTS.md`, then the `README.md` of the crate you will change
and the module docs (`//!`) of the files you touch.

## Make the change

- Keep the change within the issue and declared dependency boundaries.
- Add tests for behavior changes and failure cases.
- Keep unknown data round-trippable and failures explicit.
- Do not commit official IFC/ISO/CEN schema files, specification PDFs, or other
  standards material unless redistribution rights have been verified.
- Use reduced, redistributable fixtures rather than confidential building
  models. Record fixture provenance and licensing.

Changes consumed by `openbimrs/openbim` land here first. The integration
repository consumes released crates.io versions only, so it picks a change up
when the owning crate is released.

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
- state important exclusions as well as included scope;
- list exact verification commands and concise results;
- reference an issue from any new `TODO(#N)` marker it leaves behind;
- update documentation and the owning crate's `CHANGELOG.md` for user-visible
  behavior.

Review may request additional negative tests, mutation evidence, corpus evidence,
or a narrower scope when the risk justifies it.

## Licensing contributions

OpenBIM.rs is dual-licensed: `AGPL-3.0-or-later OR LicenseRef-Commercial`
(see [`LICENSING.md`](LICENSING.md)). To keep the commercial option
possible, every contribution is accepted on the following terms, which you
agree to by submitting it (a pull request, patch, issue attachment or any
other material intended for inclusion):

1. **Licence to the project.** Your contribution is licensed under
   `AGPL-3.0-or-later`.
2. **Licence to the copyright holder.** You also grant the copyright holder
   named in [`NOTICE`](NOTICE) a perpetual, worldwide, non-exclusive,
   royalty-free, irrevocable licence to use, reproduce, modify, distribute,
   sublicense and relicense your contribution, including under proprietary
   or commercial terms, as part of OpenBIM.rs or works derived from it.
3. **You keep your copyright.** These terms grant a licence; they do not
   transfer ownership.
4. **You have the right to grant this.** The contribution is your own work,
   or you have permission to submit it on these terms (for example from
   your employer). Identify third-party material, keep its licence,
   attribution and provenance, and do not submit third-party material whose
   licence does not allow these terms.
5. **Patents.** You grant the same parties a licence under any patent claims
   you can license that your contribution necessarily infringes, on the terms
   of section 11 of the AGPL.

If you cannot agree to these terms, say so in the issue or pull request
before submitting; the contribution can then not be merged.

Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Security
reports follow [SECURITY.md](SECURITY.md); do not disclose vulnerabilities in a
public issue.
