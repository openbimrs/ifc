#!/usr/bin/env bash
# Every .rs file under a crate's src/ must be reachable from its crate root.
#
# Why: a module file that is not declared with `mod`/`pub mod` is invisible to
# the compiler. It is not built, not linted, not tested, and not documented --
# but it still sits in the tree looking like live code, and it drifts out of
# sync with the API it claims to implement. Reviewers read it as real. It is
# not.
#
# This bit us for real: a refactor rewrote packages/ifc/ifc-geometry/src/lib.rs
# and dropped ten `pub mod` lines. 170 lines of representation scaffold
# (brep, csg, placement, profile, swept, ...) silently stopped being part of
# the crate while every gate stayed green, because nothing references them.
#
# Detection is deliberately simple: collect declared module names from the
# crate root plus any mod.rs, and compare against the files on disk.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

for manifest in $(find packages apps bindings -name Cargo.toml -not -path '*/target/*' | sort); do
    crate_dir="$(dirname "$manifest")"
    src="$crate_dir/src"
    [ -d "$src" ] || continue

    # Module names declared anywhere in the crate (root, mod.rs, submodules).
    declared="$(grep -rhoE '^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+[a-z_0-9]+[[:space:]]*;' "$src" 2>/dev/null \
        | sed -E 's/.*mod[[:space:]]+//; s/[[:space:]]*;//' | sort -u)"

    while IFS= read -r file; do
        base="$(basename "$file" .rs)"
        # Crate roots and mod.rs are reachable by definition.
        case "$base" in
            lib|main|mod) continue ;;
        esac
        if ! printf '%s\n' "$declared" | grep -qx "$base"; then
            echo "  ORPHAN  ${file#./}"
            fail=1
        fi
    done < <(find "$src" -name '*.rs' | sort)
done

if [ "$fail" -eq 0 ]; then
    echo "no orphaned module files"
else
    echo
    echo "The files above are NOT compiled: no \`mod\` declaration reaches them."
    echo "Either declare them in the crate root / parent mod.rs, or delete them."
fi
exit "$fail"
