# Switching to a local axiolid kernel

The workspace pins axiolid by tag, which ADR 0004 requires: a consumer
builds against an immutable published revision.

While an unreleased kernel change is being evaluated, point the manifest at
a working copy instead. Keep it local: a path dependency in a pushed commit
breaks CI and every other clone.

## Switch to local

A helper script is not committed on purpose -- the switch should be
deliberate. Rewrite each axiolid entry in the root `Cargo.toml`:

```toml
# from
axiolid-curve = { git = "https://github.com/axiolid/kernel.git", tag = "v0.1.8" }
# to
axiolid-curve = { path = "/home/you/projects/axiolid/kernel/crates/representations/analytic/curve" }
```

Crate directories do not match crate names, so resolve each one:

```bash
grep -rl 'name = "axiolid-curve"' crates --include=Cargo.toml
```

## Before committing

Restore the tag and re-run the gate. The committed manifest must never
contain a path dependency:

```bash
git diff Cargo.toml          # expect no path = "/..." entries
bash scripts/gate.sh
```

## When the release lands

Bump the tag in one place and delete nothing else -- the local switch is
never committed, so there is no cleanup:

```toml
axiolid-curve = { git = "https://github.com/axiolid/kernel.git", tag = "v0.1.9" }
```

## What is waiting on the release

`Curve3::Elevated` and `Curve3::Intrinsic`, with `elevated_point`,
`elevated_tangent`, `intrinsic_point` and `intrinsic_tangent`, exist in the
local kernel but in no published tag. They unblock:

- lowering `IfcGradientCurve` and `IfcSegmentedReferenceCurve`
- deriving a linear placement frame from the basis curve, replacing the
  typed refusal in `constraint/placement/linear.rs`

Neither is implemented here yet, because ADR 0004 does not admit a path
dependency into a landed commit.
