# Plan — implementing the IFC package

Status: **contract satisfied.** Codecs, schema, and the entity-agnostic model
are working and verified. Most domain views are still scaffolds.

## The architectural requirement (from GeneralPawz)

1. **`IfcModel` is entity-agnostic.** It must not know what costing is.
2. **Domain semantics live in feature-gated crates.** A thin app compiles none.
3. **Costing data must survive** a read/write cycle in a build that cannot
   interpret it.
4. **Schema and serialization are separate.** STEP, ifcXML, and prospectively
   IFC-JSON are interchangeable backends.

## Design (implemented)

```
ifc-step  --+                            +-- ifc-cost
ifc-xml   --+-- Codec --> ifc-model <----+-- ifc-schedule    (views)
ifc-json  --+             (entities)     +-- ifc-properties
                              ^
                              |  optional, never required to read
                          ifc-schema  (parses the official .exp files)
```

- `Codec` is a trait **in `ifc-model`**, so the model depends on no codec.
- Domain crates borrow `&Model`; they own no storage.
- `Model` stores `(id, type_name, attributes)`. Unknown entity types are
  ordinary rows.

## Milestones

| # | Milestone | State |
| --- | --- | --- |
| M1 | Value model + `Entity` + `Model` (no domain knowledge) | done |
| M2 | `Codec` trait in `ifc-model` | done |
| M3 | STEP lexer / parser / writer | done |
| M4 | GUID codec (IFC base-64) | done |
| M5 | `ifc-cost` as a borrowed view | done |
| M6 | `ifc` facade with per-domain features | done |
| M7 | Round-trip proof on 19 real fixtures | done |
| M8 | EXPRESS schema parser + `is_a` + attribute names | done |
| M9 | ifcXML codec (schema-aware, schema optional) | done |
| M10 | Contract test: costing survives a no-domain build | done |
| M11 | Remaining 12 domain views | **not started** |
| M12 | IFC-JSON codec | not started |
| M13 | Parallel parsing (partition.rs is unused) | not started |

## Verification (all commands re-runnable)

```bash
cargo test -p ifc --all-features                       # 12 tests
cargo build -p ifc --no-default-features               # ok
cargo build -p ifc --features step                     # ok
cargo build -p ifc --features ifcxml                   # ok
cargo build -p ifc --all-features                      # ok
cargo clippy -p ifc <each of the above> -- -D warnings  # 0 warnings
cargo test --workspace --all-features                  # 47 tests
```

Schema parser verified against the real specs:
IFC2x3 653 entities · IFC4 776 entities / 397 types · IFC4x3 876 entities.

Mutation-verified (5/5 defects caught by `costing_roundtrip.rs`): dropping
unknown entities, truncating reals, collapsing `.U.`→`.F.`, losing the XML enum
kind, emitting numeric-looking strings raw.

## Known limitations — do not overclaim

- **Round-trip fidelity is semantic, not byte-identical.** Real lexemes
  normalize (`1.0` → `1.`) and comments are dropped. Tests compare entity
  graphs.
- **12 of 16 domain crates are scaffolds.** Only `ifc-cost` is implemented; the
  pattern is proven once, not thirteen times.
- **Parsing is single-threaded.** `ifc-step/src/partition.rs` exists for a
  rayon split but is unused. **No throughput has been measured, so no
  performance claim is made.**
- **ifcXML is structurally valid, not schema-validated.** It is not checked
  against `IFC4.xsd`, and namespace handling is minimal.
- **`ifc-xml`'s reader and writer are coupled**: `looks_numeric` must mirror
  `infer_scalar`. A third codec should extract a shared helper.
