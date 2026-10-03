# Changelog -- openbim-ifc-binding-core

All notable changes to the `openbim-ifc-binding-core` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#123, domain views: read side)

- Read-only domain views, each an owned snapshot keyed by entity id with
  the `GlobalId` where present: `IfcModel::property_sets` (exact,
  release-bound property sets, quantity sets and predefined sets, type
  inheritance applied, values typed in the tagged encoding) and
  `resolve_unit`; `spatial_tree`; `classifications`; `material`;
  `systems`; `cost`; `georeferencing`. Records live in the new
  `properties`, `spatial`, `classification`, `material`, `systems`, `cost`
  and `georef` modules.
- The `record` module: `Record`, `Field` and `ToRecord`, the
  host-independent shape every domain record crosses in (a named, ordered
  list of fields; `Record::to_tagged` is the C tape form).
- Default features `properties`, `spatial`, `classification`, `material`,
  `systems`, `cost` and `georef` (which implies `properties`). With one
  left out, its operations refuse with `FeatureDisabled`.
- `BindingError::InvalidModel` (`invalid-model`), `MissingReference`
  (`missing-reference`), `BudgetExceeded` (`budget-exceeded`),
  `Unsupported` (`unsupported`) and `WrongEntityType`
  (`wrong-entity-type`). `BindingError` is exhaustive, so the new variants
  are a breaking change for a Rust matcher: a minor release under 0.x. The
  crate is internal and unreleased, and every host here is updated with it.

### Changed (#123)

- The `unsupported-schema` message reads `unsupported schema: <detail>`
  (was `no bundled schema for <token> in this build`), since a domain view
  refuses a bundled release it does not read with the same code. The code
  and the payload of the existing refusals are unchanged.

### Added (#244)

- Lenient STEP reads: `ParseOptions` (`on_malformed`, `check_references`,
  `accept_real_without_point`; `strict()` and `lenient()` presets) and
  `IfcModel::parse_with`, `parse_owned_with`, `open_with` and the unsafe
  `open_mapped_with`. Recoveries are reported by `diagnostics()`.
- The STEP header: `IfcModel::header` and `set_header`, with the
  `header` module's tagged form (`to_tagged`, `from_tagged`) for the C tape.
- `IfcModel::validate(max_findings)`: validation against the declared
  schema as a `ValidationReport` of `ValidationFinding` records, sorted.
- `IfcModel::parse_ifcxml` and `write_ifcxml`: the native ifcXML layout,
  or the XSD layout of the `IFC4` / `IFC4X3_ADD2` profile.
- `IfcModel::unreachable_products`: `UnreachableProduct` records with a
  stable `reason` code.
- Default features `ifcxml`, `validate` and `unreachable`. With one left
  out, its operation refuses with `FeatureDisabled`.
- `BindingError::UnsupportedProfile` (`unsupported-profile`) and
  `BindingError::FeatureDisabled` (`feature-disabled`). `BindingError` is
  exhaustive, so for a Rust matcher the new variants are a breaking
  (minor, under 0.x) change; the crate is internal (`publish = false`) and
  unreleased, and every host in this workspace is updated with it.
- Parse and write errors name their format in the detail (`STEP: ...`,
  `ifcXML: ...`); the messages of STEP errors are unchanged.

### Added (lazy loading)

- `IfcModel::parse_owned(Vec<u8>)`, `IfcModel::open(path)` and the unsafe
  `IfcModel::open_mapped(path)`. A parsed model keeps its source and
  decodes entities on access (ADR 0015); `parse` copies the input once,
  `parse_owned` and `open` not at all beyond the file read.
- `BindingError::Io`, stable code `io`, for a file that cannot be read.

### Added

- Release features `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`, `ifc4x3` (all
  default), passed to `openbim-ifc`. With a release left out,
  `ids_of_type_including_subtypes` refuses a file declaring it with
  `UnsupportedSchema`, whose message says the release is not compiled in.
- The host-independent half of the language bindings (ADR 0013): `IfcModel`
  operations, the lossless `Tagged` value encoding and `BindingError` with
  stable codes, shared by the WASM, C and Python bindings. Extracted from
  `openbim-ifc-wasm`.
- Non-finite reals (NaN, infinity) are refused for every host; STEP has no
  form for them.

### Changed

- `ids_of_type_including_subtypes` resolves IFC4X1 and IFC4X2 files through
  their own bundled tables; `UnsupportedSchema` no longer lists the
  releases in its message.

