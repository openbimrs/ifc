# ifc-template-catalog implementation plan

Status: implemented and verified.
Last updated: 2026-08-19

This is opt-in task state. Follow `AGENTS.md`; claim one task ID and update proof here. Keep stable boundaries in `AGENTS.md`.

## Established boundary

Accepted by ADR 0010. This crate is external schema metadata, separate from authored instance semantics in `ifc-properties`. Runtime catalogs are immutable and version/profile explicit. Reference XML is generation input only.

## Confirmed evidence

- IFC4 ADD2 TC1: 420 PSD sets with 2,550 properties; 93 QTO sets with 257 quantities.
- The XML embeds only `IFC4`, so exact edition and checksums must come from the source manifest.
- PSD property forms present: single, bounded, enumerated, list, reference, table, and nested complex.
- QTO forms present: weight, length, area, volume, count, and time.
- IfcOpenShell generates runtime IFC template libraries and has implicit fixes but no correction ledger.
- `Qto_WallBaseQuantities` type applicability is the first evidence-backed corrected-profile patch.
- The legacy environmental-impact Psets require advisories, not silent structural replacement.

## Planned file map

- `src/definition/`: edition, source, applicability, set, property, quantity.
- `src/catalog.rs`: construction, duplicate checks, immutable indices, embedded profiles.
- `src/query.rs`: hierarchy-injected applicability matching.
- `src/overlay/`: patch contracts, application, conflict checks, built-in ledger.
- `src/xml/`: small DOM reader plus PSD/QTO decoding.
- `src/archive.rs` and `src/embedded.rs`: versioned binary artifact and cached profile loaders.
- `tools/generate.rs`: deterministic generator entry point.
- `data/`: committed artifact, provenance, and generation policy; no copied XML.

## Work queue

- [x] `CAT-CONTEXT` - ADR, crate scaffold, lean progressive context pairs.
- [x] `CAT-CONTRACT` - typed definitions, provenance, editions, diagnostics, immutable catalog.
- [x] `CAT-XML` - parse every IFC4 PSD/QTO shape with explicit errors.
- [x] `CAT-QUERY` - exact, predefined-type, and subtype-aware applicability lookup.
- [x] `CAT-OVERLAY` - official/corrected profiles, patch IDs, conflicts, advisories, custom overlays.
- [x] `CAT-GEN` - deterministic normalized IFC4 artifact and drift gate.
- [x] `CAT-INTEGRATE` - workspace/facade feature and dependency-boundary gates.
- [x] `CAT-VERIFY` - focused/full gates, mutation checks, size/timing measurements, review.

- [x] `CAT-REVIEW-FIX` - reconcile late review findings, fix confirmed invariants, regenerate, and gate.

## Late review follow-up

- Classify both asynchronous reviews against commit `e3ff514`.
- Fix confirmed profile-transition and cross-call overlay conflicts with regression tests.
- Make absent/published QTO template classification explicit in the typed model and queries.
- Regenerate the archive, rerun corpus-fidelity assertions, and land a separate fix-forward commit.

## Decisions during implementation

- The attributed official artifact preserves published names, descriptions, aliases, GUIDs, applicability, units, and type declarations; 16 empty official `DataType` declarations remain empty and diagnostic.
- A magic/version-bearing bincode artifact keeps generated data out of Rust source and decodes once into an immutable snapshot.
- Custom XML import is optional and byte/node/depth bounded; default embedded lookup has no XML dependency.
- Applicability remains a vector of entity/predefined-type clauses, not independent lists; callers can supply occurrence/type/performance context and use the structured query to retain unknown-schema outcomes.
- The exhaustive grammar audit found set-definition and enumeration-constant aliases that the first importer flattened; those are now typed and corpus-count gated.

## Completion log

Append entries as `TASK-ID - command/result - material decision`. Do not paste full logs.

- `CAT-GEN` - generator imported 513/513 XML files and reproduced byte-identical 1,537,256-byte format-v2 artifacts; source digest `57227d...36e3`, artifact digest `fe5567...8363`.
- `CAT-VERIFY` - all-feature tests/clippy and IFC architecture/context/reachability/monolith gates passed; architecture, context, and artifact corruption mutations each failed as expected. Full `scripts/gate.sh` passed after removing an unnecessary `quick-xml/encoding` feature that broke sibling `ifc-xml` through Cargo feature unification. Release measurements across three processes: official first load 3.27-3.77 ms, 100k exact-name lookups 5.35-5.69 ms, corrected first load 2.05-2.67 ms.
- `CAT-REVIEW-FIX` - late reviews reconciled; QTO/profile/cross-call findings fixed, stale findings pinned by regressions, format-v2 artifact reproduced, corruption mutation exited 101, and exact scoped gates passed. Independent follow-up review passed; its three non-blocking coverage suggestions were added and mutation-verified. The full gate is currently blocked by the pre-existing `ifc-geometry::no_backend_dependency` macro fixture on HEAD.
