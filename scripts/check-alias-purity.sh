#!/usr/bin/env bash
# Alias crates under packages/alias/ must be PURE re-exports.
#
# Why this is a gate and not a review convention: if an alias crate defines its
# own type, a dependency graph containing both the alias and its canonical
# crate ends up with two structurally identical but DISTINCT types. Cargo has
# no mechanism to unify them -- it is not a version conflict, so no resolution
# fixes it. The consumer just cannot compile.
#
# The invariant is therefore checked structurally: an alias lib.rs may contain
# doc comments, attributes, and `pub use` -- nothing that introduces an item.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
for lib in packages/alias/*/src/lib.rs; do
    [ -e "$lib" ] || continue
    crate=$(basename "$(dirname "$(dirname "$lib")")")

    # Strip doc comments, plain comments, inner attributes and blank lines;
    # whatever remains must be `pub use` and nothing else.
    body=$(sed -e 's://!.*::' -e 's://.*::' -e '/^[[:space:]]*#!\[/d' "$lib" \
           | grep -v '^[[:space:]]*$')

    if [ -z "$body" ]; then
        echo "  $crate: lib.rs has no re-export at all"
        fail=1
        continue
    fi

    offending=$(printf '%s\n' "$body" | grep -vE '^[[:space:]]*pub use [A-Za-z0-9_]+::\*;[[:space:]]*$')
    if [ -n "$offending" ]; then
        echo "  $crate: alias must contain only 'pub use <crate>::*;'"
        printf '%s\n' "$offending" | sed 's/^/      /'
        fail=1
    fi

    # The '=' pin keeps the alias and its canonical crate on the same version.
    # A caret range would let them drift apart and reintroduce the duplicate
    # type problem this script exists to prevent.
    if ! grep -qE 'version = "=[0-9]' "packages/alias/$crate/Cargo.toml"; then
        echo "  $crate: canonical dependency must be pinned with '=' (found no '=' version)"
        fail=1
    fi
done

exit "$fail"
