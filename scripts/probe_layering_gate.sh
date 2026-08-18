#!/usr/bin/env bash
# Mutation-verify packages/geometry/geom-core/tests/layering.rs.
#
# A gate that has never failed is decoration with a green light. Each mutation
# below is a real architectural violation of the kind a hurried manifest edit
# would introduce. The gate must go RED for every one of them, and stay GREEN
# for the comment-only decoy.
#
# The script REFUSES to report a result when a mutation did not actually land
# (diff against the backup), because an unapplied patch and a blind gate look
# identical from the outside.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."

GATE=(cargo test -p geom-core --test layering)
BAK=/tmp/layering_mut.bak
fail=0

# Run the gate; echo "GREEN" or "RED".
run_gate() {
    if "${GATE[@]}" >/tmp/layering_mut_out.txt 2>&1; then echo GREEN; else echo RED; fi
}

# mutate <label> <manifest> <line-to-append-to-[dependencies]> <expected>
mutate() {
    local label="$1" manifest="$2" line="$3" expect="$4"
    cp "$manifest" "$BAK"
    # Insert directly under the [dependencies] header.
    python3 - "$manifest" "$line" <<'PY'
import sys
path, line = sys.argv[1], sys.argv[2]
src = open(path).read()
assert "[dependencies]" in src, f"no [dependencies] table in {path}"
open(path, "w").write(src.replace("[dependencies]", "[dependencies]\n" + line, 1))
PY
    if diff -q "$manifest" "$BAK" >/dev/null; then
        echo "  $label: MUTATION DID NOT APPLY -- result would be meaningless"
        cp "$BAK" "$manifest"
        fail=1
        return
    fi
    local got; got=$(run_gate)
    cp "$BAK" "$manifest"
    if [ "$got" = "$expect" ]; then
        printf '  %-58s %s (expected %s)  ok\n' "$label" "$got" "$expect"
    else
        printf '  %-58s %s (expected %s)  MISS\n' "$label" "$got" "$expect"
        fail=1
    fi
}

echo "=== baseline ==="
base=$(run_gate)
printf '  %-58s %s\n' "unmutated tree" "$base"
[ "$base" = GREEN ] || { echo "baseline is not green; fix that before mutating"; exit 1; }

echo "=== mutations ==="
G=packages/geometry

# 1. The seam reversed: geometry reaching back into IFC.
mutate "geom-mesh depends on ifc-model" \
    "$G/geom-mesh/Cargo.toml" "ifc-model.workspace = true" RED

# 2. Tier inversion: a representation crate pulling in an algorithm crate.
mutate "geom-mesh (L1) depends on geom-kernel (L2)" \
    "$G/geom-mesh/Cargo.toml" 'geom-kernel = { workspace = true, default-features = false }' RED

# 3. The root gaining a sibling. A normal dep would be a cargo CYCLE (and so
#    inconclusive), but cargo permits dev-dependency cycles -- and the gate
#    counts dev-dependencies deliberately, because a test that reaches across
#    the boundary disproves the boundary just as well as a release dep does.
cp "$G/geom-core/Cargo.toml" "$BAK"
printf '\n[dev-dependencies]\ngeom-mesh.workspace = true\n' >> "$G/geom-core/Cargo.toml"
if diff -q "$G/geom-core/Cargo.toml" "$BAK" >/dev/null; then
    echo "  geom-core dev-depends on geom-mesh: MUTATION DID NOT APPLY"; fail=1
else
    got=$(run_gate); cp "$BAK" "$G/geom-core/Cargo.toml"
    if [ "$got" = RED ]; then
        printf '  %-58s %s (expected RED)  ok\n' "geom-core dev-depends on geom-mesh" "$got"
    else
        printf '  %-58s %s (expected RED)  MISS\n' "geom-core dev-depends on geom-mesh" "$got"; fail=1
    fi
fi

# 4. A new crate appearing with no declared tier -- the way a layered design
#    quietly stops being one.
mkdir -p "$G/geom-untiered/src"
cat > "$G/geom-untiered/Cargo.toml" <<'TOML'
[package]
name = "geom-untiered"
version.workspace = true
edition.workspace = true

[dependencies]
geom-core.workspace = true
TOML
echo "// mutation probe" > "$G/geom-untiered/src/lib.rs"
got=$(run_gate)
rm -rf "$G/geom-untiered"
if [ "$got" = RED ]; then
    printf '  %-58s %s (expected RED)  ok\n' "new crate with no tier in TIERS" "$got"
else
    printf '  %-58s %s (expected RED)  MISS\n' "new crate with no tier in TIERS" "$got"; fail=1
fi

# 5. Decoy: the violation exists only as a COMMENT. A gate that trips on this
#    is a gate nobody can write an explanatory note next to.
mutate "COMMENTED-OUT ifc-model dep (must NOT trip)" \
    "$G/geom-mesh/Cargo.toml" "# ifc-model.workspace = true" GREEN

echo "=== restored ==="
printf '  %-58s %s\n' "tree after restore" "$(run_gate)"
git -C . diff --quiet -- packages/geometry && echo "  worktree clean under packages/geometry" || { echo "  DIRTY -- restore failed"; fail=1; }

echo
[ "$fail" -eq 0 ] && echo "MUTATION MATRIX PASSED" || echo "MUTATION MATRIX FAILED"
exit "$fail"
