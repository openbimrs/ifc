# Plan — implementing the IFC package

Status: **M1–M7 complete.** Codec + model + one domain view are working and
verified. Remaining domain crates are still scaffolds.

## The architectural requirement (from GeneralPawz)

1. **`IfcModel` is entity-agnostic.** It must not know what a cost item, a task,
   or a wall *means*. It stores a typed entity graph and nothing more.
2. **Domain semantics live in feature-gated crates.** A thin app compiles
   `ifc-model` + one codec and nothing else.
3. **Round-trip fidelity without comprehension.** A file containing cost data
   must parse, survive, and re-export intact *even when `ifc-cost` is not
   compiled in*. Understanding is optional; preservation is not.
4. **Schema and serialization are separate concerns.** STEP/SPF is one codec.
   ifcXML and a future IFC-JSON are others. None of them own the data model.

## Delivered

```
ifc-schema  ←  ifc-model  ←  ifc-step   (codec, implements Codec)
                    ↑      ←  ifc-xml   (future codec, same trait)
                    ↑      ←  ifc-json  (future codec, same trait)
                    └───────  ifc-cost  (domain VIEW, borrows &Model)
```

| Milestone | State | Evidence |
| --- | --- | --- |
| M1 model: `Value`, `Entity`, `Model`, index, GUID | done | `model_invariants.rs` 6 tests |
| M2 `Codec` trait in the model | done | `thin_build.rs::the_model_does_not_depend_on_any_codec` |
| M3 STEP lexer + parser + writer | done | 26 unit tests in `ifc-step` |
| M4 round-trip on the fixture corpus | done | 19 files, 7,920 entities, `roundtrip.rs` |
| M5 `ifc-cost` as a view | done | 3 tests; borrows only |
| M6 `ifc` facade with per-domain features | done | 26 crates thin vs 51 full |
| M7 cost survives without `ifc-cost` compiled | done | `roundtrip.rs::cost_data_survives_...` |

## Verified numbers

- **19/19 fixtures parse**, IFC2x3 + IFC4 + IFC4X3_ADD2, 7,920 entities.
- **19/19 round-trip** structurally identical after re-parse.
- **26 crates** in a thin (`step`) build vs **51** with `full`; no `glam`, no
  geometry kernel in the thin build.
- **40 tests** workspace-wide, clippy clean (default *and* `--all-features`),
  rustdoc 0 warnings.

## Mutation testing found two real holes

Both tests passed while being wrong; both are now fixed:

1. `thin_build.rs` only checked `--no-default-features --features step`, so a
   domain leaking into `default` was invisible. Added
   `default_features_pull_in_no_domain_crate`.
2. Nothing checked `Model::insert` id reuse. Dropping the `is_none()` guard
   duplicated the id in the export order list, silently emitting an entity
   twice, with every test still green. Added `model_invariants.rs`.

An earlier version of the dependency test read `cargo metadata`'s package list
and passed vacuously — every workspace member is listed there regardless of
features. It now reads `cargo tree`, which reflects resolved edges.

## Not done yet

- **Other domain crates are still scaffolds.** `ifc-cost` is the worked example
  proving the pattern; `ifc-schedule`, `ifc-properties`, `ifc-material` and the
  rest have module trees and docs but no view implementation.
- **`ifc-schema` is not wired to the parser.** The EXPRESS `.exp` files are in
  `references/ifc-spec/`; nothing reads them yet, so there is no subtype-aware
  querying (`is_a("IfcWall", "IfcElement")`) and no attribute validation.
- **No ifcXML codec.** The trait exists and the model is codec-free, so it is
  additive work, but it is unwritten and therefore unproven.
- **Parsing is single-threaded.** `partition.rs` exists for the rayon split but
  the parser walks the token stream serially. No throughput number has been
  measured, so no performance claim is made.
- **`Value` is not interned.** Owned `Arc<str>` per attribute. Correct, but
  memory use on a 500 MB model is unmeasured.
