#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/ifc.
#
# Usage: scripts/gate.sh [section...]
#
# With no argument every section runs, in order: this is the full gate, and
# the one to run before a merge. CI runs each section as its own parallel job
# and passes only when all of them pass, so the union is identical:
#
#   lint      formatting, clippy, rustdoc, documentation and licensing gates
#   test      workspace build and tests, architecture and context gates
#   features  feature-column builds: kernel-free, compile, facade, browser WASM
#   bindings  JavaScript, C (and its CMake package) and Python bindings
set -euo pipefail

cd "$(dirname "$0")/.."

# Shared fleet workspaces reuse a global target directory. Isolate this
# standalone repository so compile-time manifest paths cannot leak in from the
# superproject build of the same package names.
if [[ -z "${CARGO_TARGET_DIR:-}" && -d /mnt/backup/build-cache ]]; then
    export CARGO_TARGET_DIR=/mnt/backup/build-cache/openbim-ifc-standalone
fi

# The normative EXPRESS schemas are CC BY-ND 4.0 and therefore not committed.
# Without them every schema-backed test silently skips -- which is how an
# IFC2X3 slot-name bug reached main with CI green. Fetch them (cached by
# checksum), then require them: IFC_SPEC_REQUIRED turns a skip into a failure.
if [[ -z "${IFC_SPEC_SKIP_FETCH:-}" ]]; then
    scripts/fetch-ifc-schemas.sh
fi
export IFC_SPEC_REQUIRED=1

gate_lint() {
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

    # Documentation gates. Every committed generated docs file and region (crate
    # reference, install table, binding APIs, capability and coverage
    # tables, facts.json) and every test-sourced snippet must match what
    # `cargo run -p xtask -- docs` would write, so drift is a build failure
    # rather than a silent inconsistency the reader has to notice. The same
    # check rejects hand-written code fences, git dependencies, pinned TOML
    # versions, typed crate counts and home paths on any page, and a
    # publishable crate with no changelog. It also writes the gitignored
    # changelog page the site build below needs.
    cargo run --quiet -p xtask -- docs --check

    # Open work lives in GitHub issues: a code marker names its issue as
    # `TODO(#N)`, and no checked-in PLAN.md may return as a second backlog.
    cargo run --quiet -p xtask -- todo --check
    cargo run --quiet -p xtask -- plans --check
    python3 scripts/check-inline-html.py

    # Build the docs site when its toolchain is installed. The checks above
    # validate content; only the real build resolves every link, and a dead
    # link fails Pages *after* a push. Catching it here keeps that failure
    # local. Skipped when node_modules is absent so the gate still runs on a
    # machine without the docs toolchain.
    if [ -d node_modules ]; then
      npm run docs:build --silent > /tmp/docs-build.log 2>&1 \
        || { echo "docs build failed:"; tail -20 /tmp/docs-build.log; exit 1; }
      echo "docs build ok"
    else
      echo "docs build skipped (no node_modules)"
    fi

    # Public API docs. Library crates enforce missing_docs through
    # [workspace.lints]; this fails for one that neither opts in nor carries a
    # measured budget, and for a budget that grew or shrank without being
    # updated.
    python3 scripts/check-missing-docs.py

    # Licensing gate. The IFC schemas are CC BY-ND 4.0 and must never reach
    # the published tree; this rejects XSD/PDF payloads and any `references/`
    # or `schemas/` path. It ran only by hand until now, which is how a
    # tracked `ifc-geometry/references/` survived undetected.
    python3 scripts/check-leakage.py
}

gate_test() {
    cargo build --workspace --all-targets
    # The workspace test run doubles as the authored-coverage measurement:
    # with `--all-features` the `ifc-model/authored-dump` hook is compiled in,
    # and AUTHORED_DUMP makes every test process record the entity types it
    # created.
    authored_dump="$(mktemp -d)"
    AUTHORED_DUMP="$authored_dump" cargo test --workspace --all-features
    python3 scripts/authored-coverage.py "$authored_dump" --json > "$authored_dump/report.json"
    committed=docs/.vitepress/data/authored-coverage.json
    if ! cmp -s "$authored_dump/report.json" "$committed"; then
        if [[ -n "${UPDATE_AUTHORED_COVERAGE:-}" ]]; then
            cp "$authored_dump/report.json" "$committed"
            echo "updated $committed; regenerate the docs with: cargo run -p xtask -- docs"
        else
            diff "$committed" "$authored_dump/report.json" || true
            echo "authored coverage changed; rerun with UPDATE_AUTHORED_COVERAGE=1 scripts/gate.sh test" >&2
            rm -rf "$authored_dump"
            exit 1
        fi
    fi
    rm -rf "$authored_dump"

    cargo test -p ifc-model --test package_architecture
    cargo test -p ifc-model --test progressive_context
    cargo test -p ifc-model --test module_reachability
    cargo test -p ifc-model --test no_monolithic_files
    cargo test -p ifc-geometry --test declaration_manifest
    cargo test -p ifc-geometry --test no_backend_dependency
    cargo test -p ifc-geometry --test kernel_free_build
    cargo test -p ifc-georef --test kernel_free_build
}

gate_features() {
    # The kernel-free column. `--all-features` cannot see a boundary that
    # only exists when a feature is OFF, so a 2D consumer's build is verified
    # explicitly: it must compile, pass its tests, and link no geometry crate.
    cargo build -p ifc-geometry --no-default-features
    cargo test -p ifc-geometry --no-default-features
    cargo clippy -p ifc-geometry --no-default-features --all-targets -- -D warnings
    # Intra-doc links to feature-gated items resolve under `--all-features` and
    # break here, so rustdoc gets its own kernel-free run.
    RUSTDOCFLAGS="-D warnings" cargo doc -p ifc-geometry --no-default-features --no-deps

    # The semantic georeferencing column (#268): without `transform`,
    # ifc-georef reads CRS, map-conversion parameters and north with no
    # geometry crate linked. kernel_free_build.rs asserts the resolved graph
    # and reads an IFC4 map conversion and true north in this column.
    cargo build -p ifc-georef --no-default-features
    cargo test -p ifc-georef --no-default-features
    cargo clippy -p ifc-georef --no-default-features --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc -p ifc-georef --no-default-features --no-deps

    # The compile column. `--all-features` builds it but cannot prove it is
    # OPTIONAL: a default-enabled feature edge would satisfy an --all-features
    # run while shipping an execution provider to every consumer. The
    # dependency-graph assertions live in kernel_free_build.rs; this runs the
    # pairing corpus, which needs the feature explicitly.
    cargo test -p ifc-geometry --features compile
    cargo clippy -p ifc-geometry --features compile --all-targets -- -D warnings

    # `spatial` and `properties` need a release to read through (#306), so
    # their combinations name one; both crates refuse to compile without.
    for features in "--no-default-features" "--features step" "--features ifcxml" "--features step,ifc4" "--features step,schema-api" "--features step,geometry-select" "--features step,ifc4,validate" "--features step,ifc4,spatial,geometry-select" "--features step,ifc4,properties,geometry-select" "--features step,ifc4,spatial,properties" "--features step,ifc4,georef,properties" "--features step,ifc4,properties,property-catalog-runtime" "--all-features"; do
        # shellcheck disable=SC2086
        cargo build -p openbim-ifc $features
        # shellcheck disable=SC2086
        cargo clippy -p openbim-ifc $features --all-targets -- -D warnings
    done

    # The unreachable-product lint spans two sibling domains, so it exists
    # only when both are on. `--all-features` would hide a break in that exact
    # pairing.
    cargo test -p openbim-ifc --features step,schema,spatial,geometry-select --test unreachable_corpus
    # Door and window operation geometry (#148, #170) join placement and
    # panel properties, so they exist only with both `geometry-select` and
    # `properties`.
    cargo test -p openbim-ifc --features step,schema,properties,geometry-select --lib \
        --test door_operation --test door_operation_refusals \
        --test window_operation --test window_operation_refusals
    # Element properties by spatial container (#121) join the spatial tree and
    # exact property resolution, so they exist only with both `spatial` and
    # `properties`.
    cargo test -p openbim-ifc --features step,schema,spatial,properties --test spatial_properties
    # Every coordinate operation scaled by the project length unit (#123) joins
    # `ifc-georef` and exact unit resolution, so it exists only with both
    # `georef` and `properties`.
    cargo test -p openbim-ifc --features step,schema,georef,properties --test georeferencing
    # Property edits (#123) check `Pset_`/`Qto_` sets against the PSD/QTO
    # catalog when `property-catalog` is on; `--all-features` hides the
    # refusal a build without it gives.
    cargo test -p openbim-ifc --features step,schema,properties --test property_edit
    # With the catalog supplied at runtime (#318), a `Pset_`/`Qto_` edit
    # waits for its edition to be installed; `--all-features` embeds it.
    cargo test -p openbim-ifc --features step,schema,properties,property-catalog-runtime \
        --test property_edit --test property_catalog_runtime
    # The catalog crate on its own: the runtime column without the embedded
    # container, and its wasm32 build (sha2 for the pins must build there).
    cargo clippy -p ifc-template-catalog --no-default-features --features runtime --lib -- -D warnings
    cargo build -p ifc-template-catalog --no-default-features --features runtime --target wasm32-unknown-unknown

    # Per-release schema column (#112). `--all-features` always bundles every
    # release, so a single-release build is the only place the `NotBundled`
    # refusal is reachable: the schema crate and the binding core each test
    # it with one release compiled in, and clippy checks the release-free
    # build.
    cargo test -p ifc-schema --no-default-features --features ifc4
    cargo clippy -p ifc-schema --no-default-features --all-targets -- -D warnings
    cargo clippy -p ifc-schema --no-default-features --features ifc4 --all-targets -- -D warnings
    cargo test -p openbim-ifc-binding-core --no-default-features --features ifc4
    # The binding capabilities (#244) are features too: the line above runs
    # with all three left out (their `feature-disabled` refusals), this one
    # reaches the XSD-profile refusal for a release left out of the build.
    cargo test -p openbim-ifc-binding-core --no-default-features --features ifc4,ifcxml
    # The domain views (#123) are features too: the `ifc4` run above refuses
    # all seven with `feature-disabled`; this one binds georeferencing alone,
    # which brings property sets with it and leaves the other five out.
    cargo test -p openbim-ifc-binding-core --no-default-features --features ifc4,georef
    # Writing property sets (#123) is a feature of its own, and the catalog
    # another: this run writes without the catalog, with the IFC4 table
    # alone, so an IFC2X3 file is refused as unbundled.
    cargo test -p openbim-ifc-binding-core --no-default-features --features ifc4,properties-write
    # The npm package's catalog (#318): loaded at runtime, refused with
    # `catalog-not-loaded` before; tests/catalog.rs loads it.
    cargo test -p openbim-ifc-binding-core --no-default-features --features ifc4,property-catalog-runtime

    # Every crate the bindings reach takes its releases from the build too
    # (#306): each builds and tests with one release; the binding core with
    # every capability and domain on still refuses a release left out, and
    # links the IFC4 table alone. `cargo tree -e normal` leaves out the
    # dev-dependencies, whose `ifc-schema/default` would bundle every release
    # into test builds and hide a leak.
    all_bound="ifc4,ifcxml,validate,unreachable,properties,spatial,classification,material,systems,cost,georef"
    for crate in ifc-validate ifc-spatial ifc-geometry ifc-properties ifc-classification \
        ifc-material ifc-systems ifc-cost ifc-georef; do
        cargo test -p "$crate" --no-default-features --features ifc4 --lib
        cargo clippy -p "$crate" --no-default-features --features ifc4 --all-targets -- -D warnings
    done
    cargo clippy -p ifc-validate --no-default-features --all-targets -- -D warnings
    cargo test -p openbim-ifc-binding-core --no-default-features --features "$all_bound"
    releases="$(cargo tree -p openbim-ifc-binding-core --no-default-features \
        --features "$all_bound" -e features,normal -i ifc-schema |
        grep -oE 'ifc-schema feature "ifc[0-9x]+"' | sort -u || true)"
    if [[ "$releases" != 'ifc-schema feature "ifc4"' ]]; then
        echo "error: an IFC4-only binding core links other releases' tables:" >&2
        echo "$releases" >&2
        exit 1
    fi

    # Browser WASM column (#34). The facade must build for
    # wasm32-unknown-unknown with its default and widest pure-Rust feature
    # sets; a native-only dependency (getrandom via ahash was the first)
    # breaks every JS consumer silently.
    for features in "" "--features schema,ifcxml,author,domains,spatial" "--no-default-features --features step,ifc4"; do
        # shellcheck disable=SC2086
        cargo build -p openbim-ifc --target wasm32-unknown-unknown $features
    done
    # The browser package with one bundled release (#112), without and with
    # the capabilities (#244, #306).
    cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown --no-default-features --features ifc4
    cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown --no-default-features --features "$all_bound"
}

gate_bindings() {
    # JavaScript bindings (#34, #40, ADR 0013): build the npm package's three
    # targets with the pinned wasm-bindgen CLI, run the Node
    # smoke and corpus suites, then check the packed tarball from Node, a
    # webpack bundle and headless Chrome, so the binding is proven to work
    # from JS, not just to compile.
    if [[ -n "${IFC_SKIP_JS:-}" ]]; then
        echo "warning: IFC_SKIP_JS set; JS binding suites NOT run" >&2
    else
        crates/openbim-ifc-wasm/scripts/build-npm-pkg.sh
    fi

    # C ABI (#38, ADR 0013): the committed header must match the exports (the
    # `header` test in the `test` section), and a C program compiled as strict
    # C11 and as C++17 must parse, read, edit, write and re-parse through the
    # real library.
    crates/openbim-ifc-capi/scripts/check-c.sh
    # The CMake package (#41): source-tree, installed and packed-archive
    # consumers, shared and static, run the same smoke test. macOS and
    # Windows run it in .github/workflows/native.yml.
    python3 crates/openbim-ifc-capi/scripts/check-cmake.py

    # Python (#39, ADR 0013): build the abi3 wheel with maturin, install it
    # into a throwaway uv venv, and run the Python smoke and corpus suites
    # against it.
    if [[ -n "${IFC_SKIP_PYTHON:-}" ]]; then
        echo "warning: IFC_SKIP_PYTHON set; Python binding suites NOT run" >&2
    else
        crates/openbim-ifc-py/scripts/check-python.sh
    fi
}

sections=("$@")
if [[ ${#sections[@]} -eq 0 ]]; then
    sections=(lint test features bindings)
fi
for section in "${sections[@]}"; do
    case "$section" in
        lint | test | features | bindings) ;;
        *) echo "error: unknown gate section '$section' (lint, test, features, bindings)" >&2; exit 2 ;;
    esac
done
for section in "${sections[@]}"; do
    echo "== gate: $section"
    "gate_$section"
done
