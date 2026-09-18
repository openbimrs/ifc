# Developing against an unreleased kernel

Axiolid is under continuous development. When a capability we need is on
its `main` but not yet published, we develop against a local checkout
WITHOUT changing any tracked file.

## The rule

Never put a `path` or `git` dependency in `Cargo.toml`. A local path
builds only on the machine that has it, so committing one breaks CI for
every other clone. The architecture gate
(`the_kernel_is_consumed_as_a_published_release`) enforces this and will
fail the build.

## Instead: patch the registry

Cargo can redirect a published crate to a local path without the
manifest knowing:

```toml
# .cargo/config.toml -- gitignored, never committed
[patch.crates-io]
axiolid-core = { path = "/home/friedrich/projects/axiolid/kernel/crates/foundation/core" }
# ... one line per pinned crate
```

Regenerate the full table for every crate the workspace pins:

```sh
K=/home/friedrich/projects/axiolid/kernel
mkdir -p .cargo
printf "[patch.crates-io]\n" > .cargo/config.toml
for c in $(grep -oE "^axiolid[a-z-]*" Cargo.toml | sort -u); do
  p=$(find $K -name Cargo.toml -not -path "*/target/*" \
        -exec grep -l "^name = \\"$c\\"$" {} \\; | head -1)
  echo "$c = { path = \\"$(dirname $p)\\" }" >> .cargo/config.toml
done
```

## Why this is safe

`Cargo.toml` keeps naming published versions, so:

- the architecture gate passes unchanged;
- CI resolves from crates.io and ignores the patch entirely;
- returning to the registry is `rm -rf .cargo`.

Only `Cargo.lock` records the local paths, and it is regenerated on the
next resolve. Check before committing:

```sh
git show HEAD:Cargo.toml | grep -c "axiolid.*path ="   # must be 0
git status --porcelain Cargo.lock                       # do not commit local paths
```

## Pitfall

Tests passing locally do NOT prove the commit builds in CI: locally the
kernel is newer than what crates.io has. A commit that depends on an
unreleased capability must WAIT for the release, even though it is green
here. Land only what the published kernel can also build.
