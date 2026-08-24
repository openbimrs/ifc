# PSD/QTO template catalog architecture

Status: research basis for accepted ADR 0010; implementation lives in `packages/ifc-template-catalog`.

## Executive result

Nehirde already has the right instance-semantics crate, `ifc-properties`, but it does not implement PSD/QTO support. Its current `standard.rs` only announces an offline-generated catalog and its template modules are private placeholders. The adjacent plan leaves `PROP-TEMPLATE` unchecked.

The official PSD and QTO XML files are external catalog definitions. They are not `IfcPropertySetDefinition` instances in an authored model. IFC4 can represent their meaning as `IfcPropertySetTemplate` and property-template entities; `IfcRelDefinesByTemplate` can then relate a template to actual set definitions.

Recommendation: do not compile the official catalog and Nehirde policy into `ifc-properties`. Keep that crate responsible for authored property/quantity instances and in-file template views. Add a sibling, optional `ifc-template-catalog` crate with immutable official release snapshots plus explicit, provenance-carrying correction overlays. The caller must choose `Official` or `Corrected`; parsing an IFC file must never silently consult or enforce either catalog.

The environmental-impact PSDs are not safely repairable as small catalog patches. Preserve them in the official profile for interoperability, mark them as legacy/underspecified in the corrected profile, and put an EPD phase-by-indicator model in a separate optional domain package.

## Scope and evidence base

Audited 2026-08-19 against:

- nehirde `6c5baced17c51d61c8d0c8d895817bbcfc130a8c` plus the shared worktree state;
- IfcOpenShell `1a6336bd207c8cdd2dc0013263a00b7eccb2a349`;
- local official IFC2X3 TC1, IFC4 ADD2 TC1, and IFC4X3 ADD2 references.

## What the files mean

A PSD declares a named property-set template: description, applicability, property names, value forms, IFC measure types, enumerations, translations, and identifiers. A QTO file does the analogous job for an `IfcElementQuantity` template and its quantity kinds.

For example, the official IFC4 files say:

- `Pset_EnvironmentalImpactIndicators` is `PSET_TYPEDRIVENOVERRIDE`, applies to `IfcElement`, and defines 19 properties (`.../psd/Pset_EnvironmentalImpactIndicators.xml:2-13`).
- `Pset_EnvironmentalImpactValues` also applies to `IfcElement` and contains scalar impact values (`.../psd/Pset_EnvironmentalImpactValues.xml:2-12`).
- `Qto_WallBaseQuantities` applies to `IfcWall` and starts with length/width/height quantities (`.../qto/Qto_WallBaseQuantities.xml:2-15`).

The XML serialization is a publication artifact, not the domain boundary. The normalized definitions and applicability rules are the useful capability. Applications may use them to suggest, validate, or author properties, but an IFC codec should only preserve the authored graph.

The IFC4 schema's own `IfcPropertySetTemplate` documentation states that it describes the properties or quantities of a set and may be assigned to a set by `IfcRelDefinesByTemplate`. This is the in-model counterpart, not proof that every external PSD XML file is an authored `IfcPropertySetDefinition`.

## Nehirde status

`packages/ifc-properties` is the nearest crate and has the correct boundaries for authored instances:

- `AGENTS.md:3-18` assigns property, quantity, unit, in-file template, standard-library projection, and authoring seams.
- `src/lib.rs:22-25` explicitly says the crate is a scaffold.
- `src/standard.rs:1-6` promises an offline-generated official catalog but has no implementation.
- `src/template/property_set.rs` and `src/template/property.rs` contain only ownership comments.
- `PLAN.md:38-51` leaves every implementation item open, including `PROP-TEMPLATE`.

A source scan found no `IfcSimplePropertyTemplate`, `IfcComplexPropertyTemplate`, `IfcRelDefinesByTemplate`, `PropertySetDef`, or QTO implementation. Therefore: there is a planned crate and module seam, but no PSD/QTO capability yet.

The current plan puts external standard data beside instance semantics. That is the one boundary I recommend changing before implementation.

## Official-data findings

The local release artifacts contain:

| Release | PSD XML | QTO XML | Relevant XSD situation |
| --- | ---: | ---: | --- |
| IFC2X3 TC1 | 317 | 0 in this checkout | external PSD format |
| IFC4 ADD2 TC1 | 420 | 93 | PSD and QTO use different XML shapes |
| IFC4X3 ADD2 | 0 in this checkout | 0 | EXPRESS only locally |

All 513 IFC4 catalog files parse as XML and have unique set names. They contain 2,550 property definitions and 257 quantity definitions. Their embedded version is only `IFC4`, not `IFC4 ADD2 TC1`, so the generated source manifest must carry the exact release identity that the files omit. The audit also found and corrected a stale local inventory line that had conflated the 420 PSD files with the full PSD/QTO total.

The 420 plus 93 IFC4 files are distinct from the IFC instance XSD at `annex/annex-a/general-usage/IFC4_ADD2.xsd`. That XSD serializes IFC models; it does not validate `PropertySetDef` or `QtoSetDef` publication files.

The PSD root claims `xsi:noNamespaceSchemaLocation="http://buildingSMART-tech.org/xml/psd/PSD_IFC4.xsd"` (`Pset_EnvironmentalImpactIndicators.xml:2`). That URL is currently unavailable. buildingSMART issue [IFC4.x-development#630](https://github.com/buildingSMART/IFC4.x-development/issues/630) records the broken link and a maintainer's preference to revisit and unify the PSD/QTO schemas. An archived PSD XSD remains at the IFC4 FINAL release path; the analogous tested QTO release path returned 404.

Consequences for ingestion:

1. Do not rely on remote schema resolution.
2. Treat XSD validity as only a syntax/shape gate, not semantic correctness.
3. Preserve unknown fields and emit import diagnostics.
4. Pin every generated catalog to source checksums and an IFC release identifier.

The environmental templates demonstrate a modeling limitation, not merely a typo:

- indicators store only one `LifeCyclePhase` enumeration and scalar impacts;
- the enumeration uses broad process words such as `Acquisition`, `Manufacture`, `Usage`, and `Wholelifecycle` (`Pset_EnvironmentalImpactIndicators.xml:77-106`), not an EPD module axis;
- `Pset_EnvironmentalImpactValues` contains the resulting scalars but no phase key and no machine-readable relation to a particular indicators instance;
- impact equivalence is described in prose while values use generic IFC mass/energy measures.

That representation cannot faithfully carry a phase-by-indicator EPD matrix without repetition and out-of-band conventions. It should be classified as legacy/underspecified rather than silently reinterpreted.

## How IfcOpenShell handles it

IfcOpenShell does not read hundreds of PSD/QTO XML files on every application run. It converts catalogs into IFC template-library files:

- `util/schema/Pset_IFC2X3.ifc`;
- `util/schema/Pset_IFC4_ADD2.ifc`;
- `util/schema/Pset_IFC4X3.ifc`.

At the audited commit those files contain 317, 513, and 760 `IfcPropertySetTemplate` records respectively. The IFC4 count is exactly 420 PSD plus 93 QTO definitions.

`ifcopenshell.util.pset.PsetQto` (`util/pset.py:43-189`):

- maps schema aliases to one or more packaged template files;
- performs named lookup and applicability lookup;
- resolves entity inheritance through the IFC schema;
- accepts a caller-supplied list of template IFC files;
- returns the first matching name, so ordered custom files can serve as coarse overlays.

`edit_pset` accepts an explicit template and otherwise loads the packaged catalog (`api/pset/edit_pset.py:181-186`). `edit_qto` advertises the same parameter, but the audited commit has a state-name bug: the explicit branch assigns `self.pset_template` at `api/pset/edit_qto.py:158-160`, while quantity type selection reads `self.qto_template` at `:215-229`. No class initializer supplies that attribute, and the QTO tests cover packaged templates but not this explicit-template branch. Templates otherwise primarily infer property/quantity types and enumerations during editing; these APIs are not comprehensive standards validators.

The special PSD/QTO path is Python-side. IfcOpenShell's C++ layer exposes ordinary IFC entities such as `IfcPropertySetTemplate`; it does not discover or parse external PSD/QTO XML. `PsetQto` memoizes applicability and name lookups (`util/pset.py:72-181`), so mutating or inserting template sources after a lookup can leave stale cached answers. Nehirde should therefore make resolved catalog snapshots immutable; constructing a new snapshot is the cache-invalidation mechanism.

IfcOpenShell contains pragmatic corrections, but no explicit correction ledger/profile:

- its IFC4 template still says `Qto_WallBaseQuantities` applies to `IfcWall`;
- applicability code adds type/occurrence compatibility, and the test calls `IfcWallType` support a "Backported fix for IFC4" (`test/util/test_pset.py:46-53`);
- the IFC4X3 converted catalog changes this template to `IfcWall,IfcWallType` and `QTO_TYPEDRIVENOVERRIDE`;
- both environmental-impact templates remain in IFC4X3 with the same legacy descriptions, broadened to `IfcElement,IfcElementType`.

Thus IfcOpenShell is useful prior art for normalized IFC template libraries and schema-aware applicability. It does **not** give applications a first-class `official versus corrected` policy, per-fix provenance, conflict reporting, or a semantic EPD replacement.

## Recommended package boundary

Add an optional sibling package:

```text
packages/
  ifc-properties/          # authored Psets/Qtos, units, in-file template views
  ifc-template-catalog/    # external official catalogs and correction overlays
```

Keep `ifc-template-catalog` at the metadata tier. It may depend on `ifc-schema`, but not `ifc-model`, codecs, geometry, or `ifc-properties`. Applications/facades compose the catalog with property authoring or validation. This preserves the existing rule that sibling domain crates do not depend on one another.

`ifc-properties::template` remains useful and distinct: it projects actual `IfcPropertySetTemplate`, `IfcSimplePropertyTemplate`, `IfcComplexPropertyTemplate`, and `IfcRelDefinesByTemplate` records found in a model. `ifc-template-catalog` owns definitions shipped outside a model.

Suggested feature shape:

```toml
# facade
property-catalog = ["dep:ifc-template-catalog"]
# no catalog in default, step, or properties
```

Do not hide the catalog under the existing `properties` feature. A viewer that only reads authored values should not pay for hundreds of standard definitions.

## Catalog contract

Use one normalized, lossless-enough model for PSD and QTO:

```rust
pub enum CatalogProfile { Official, Corrected }
pub enum TemplateKind { PropertySet, QuantitySet }
pub struct CatalogEdition { /* IFC release, source revision/checksum */ }
pub struct SetTemplate { /* id, name, kind, applicability, members, provenance */ }
pub struct Applicability { raw: String, selectors: Vec<EntitySelector> }
pub struct ResolvedTemplate { definition: SetTemplate, applied_fixes: Vec<FixId> }
```

Important invariants:

1. Identity is release plus stable source ID/GUID plus name, never name alone.
2. Preserve raw applicability and unknown XML fields beside normalized selectors.
3. Expand supertypes/subtypes at query time through `ifc-schema`; do not bake a guessed inheritance closure into generated data.
4. Keep Pset and QTO member kinds distinct even when one query API serves both.
5. Lookups return provenance and diagnostics, not just a definition.
6. Parsing/round-tripping a model never enforces a catalog implicitly.
7. Validation, UI suggestions, and authoring are separate consumers with separately selectable severity/policy.

## Correction model

Never edit imported official data in place. Generate an immutable `Official` snapshot and apply ordered, declarative patches to obtain `Corrected`:

```rust
pub struct CatalogPatch {
    id: FixId,
    editions: EditionRange,
    target: TemplateIdentity,
    operation: PatchOperation,
    rationale: &'static str,
    evidence: &'static [EvidenceRef],
    confidence: FixConfidence,
}
```

Operations should be narrow: change applicability, change member type, rename/alias, deprecate, suppress, or add a Nehirde extension. Detect stale targets and conflicting writes during generation/tests. Every corrected lookup exposes applied patch IDs.

Separate three classes of knowledge:

- **Correction**: high-confidence factual defect; applied by `Corrected`.
- **Advisory**: disputed or lossy modeling; definition remains but consumers get a diagnostic.
- **Extension/replacement**: a different semantic model; new namespace/identity, never masquerading as the official Pset.

Default recommendation:

- `Official` is the interoperability default for catalog lookup/authoring.
- `Corrected` is opt-in and deterministic.
- validation reports which profile was used;
- custom overlays apply after a selected built-in profile and conflicts are errors unless the caller explicitly chooses precedence.

This is stronger than IfcOpenShell's ordered first-match files: it makes every divergence inspectable and testable.

## EPD recommendation

Do not "fix" `Pset_EnvironmentalImpactIndicators` or `Pset_EnvironmentalImpactValues` by changing their meaning under the same names. In `Official`, reproduce them exactly. In `Corrected`, attach a legacy/underspecified advisory and optionally discourage new authoring while still reading existing files.

Model EPD semantics separately, preferably in an optional `packages/epd` package because EN/ISO EPD semantics sit above generic IFC property mechanics. Its internal contract should represent at least:

```text
EPD document + declared/functional unit + product/classification reference
  -> module/phase (A1-A3, A4, ..., C, D as supported by the chosen standard)
     -> indicator
        -> typed value + explicit unit + method/version + provenance
```

Lowering options into IFC should be explicit adapters, for example:

1. reference the authoritative EPD document/classification from IFC;
2. encode each indicator as an `IfcPropertyTableValue` keyed by module labels; or
3. encode one `IfcComplexProperty` per module containing typed indicator values.

The exact export mapping needs its own ADR and fixtures. It must not invent equivalence-unit semantics in prose-only `IfcMassMeasure` values. Round-trip and validation tests should prove that module, indicator, unit, and source identity survive.

## Scaffold sequence

1. **Freeze contracts only**: create `ifc-template-catalog` with edition/profile/provenance/applicability types and no embedded catalog.
2. **Offline importer**: parse local PSD and QTO XML with no network resolution; normalize both formats; preserve unknowns; emit a source manifest with checksums and diagnostics.
3. **Official IFC4 snapshot**: generate the 420 PSD plus 93 QTO definitions into committed artifacts. A normal build must not read `references/` or require an XML parser.
4. **Query layer**: exact-name and schema-aware applicability lookup, with raw selectors and deterministic ordering.
5. **Overlay engine**: patch ledger, conflict/stale-target checks, `Official`/`Corrected` golden tests. Use the IFC4 wall-QTO type applicability behavior as an initial cross-check against IfcOpenShell.
6. **Integrate consumers**: optional facade feature; property authoring accepts an explicit resolved template; validation accepts an explicit profile.
7. **Add editions**: IFC2X3 and IFC4X3 only after their source provenance and conversion route are pinned.
8. **EPD package/ADR**: separate from the generic catalog implementation.

## Required gates

- Import counts and stable checksums per release.
- Every imported applicability entity and IFC measure type resolves in the matching `ifc-schema`, or is preserved with a diagnostic.
- Official profile is byte-for-byte or field-for-field equivalent to normalized source fixtures.
- Corrected profile differs only at declared patch targets.
- Mutate a patch target, source checksum, applicability selector, and conflict rule to prove each gate fails.
- Cross-check representative applicable-template results against IfcOpenShell, including predefined types, occurrence/type pairs, and quantities.
- Measure clean-build time, incremental build time, library size, and lookup latency before choosing generated Rust tables versus an embedded compact data artifact.
- Thin facade builds without `property-catalog` contain no catalog dependency or data.

## Alternatives considered

| Alternative | Decision |
| --- | --- |
| Put generated tables in `ifc-properties::standard` | Rejected: couples authored instance semantics to optional, policy-bearing bulk data. |
| Copy IfcOpenShell's packaged `.ifc` template files and first-match list | Useful oracle, rejected as our primary contract: licensing/provenance aside, fixes are implicit and conflicts are hidden. |
| Correct official rows in generated output | Rejected: destroys reproducibility and makes upstream comparisons ambiguous. |
| Runtime-load arbitrary XML/XSD | Support later as a custom provider if needed, not the standard path; remote schemas are unreliable and runtime XML is unnecessary weight. |
| Treat every defect as an override | Rejected: some defects, especially EPD modeling, are non-isomorphic semantic replacements. |
| Put EPD logic in `ifc-properties` | Rejected: EPD phase/method semantics are a higher-level standard/domain concern. |

## Decision still required

Before code is scaffolded, accept or reject the crate split in an ADR. If accepted, update `ifc-properties/AGENTS.md` and `PLAN.md` so `standard.rs` becomes an integration/provider seam or is removed; do not leave two owners for the same catalog.
