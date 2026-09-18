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

## Pitfall: a version bump silently disables the patch

A patch entry only applies when its version satisfies the requirement in
`Cargo.toml`. When the kernel released 0.3.0 while this workspace still
pinned 0.2.1, every entry became inert and cargo reported it as a warning
with a zero exit code:

```
warning: Patch `axiolid-core v0.3.0 (...)` was not used in the crate graph.
```

The build then silently used the published crates instead. Tests that
depend on unreleased work fail in a way that looks like the feature broke.

Check the patch is live rather than assuming it:

```sh
cargo metadata --format-version 1 2>&1 >/dev/null | grep "was not used"
```

Silence means every entry applied. Any output means the local checkout
moved to a version the pins no longer accept: bump the pins in
`Cargo.toml`, or check out a kernel revision matching them.
