# Changelog -- openbim-ifc-dotnet

All notable changes to the `openbim-ifc-dotnet` crate, which ships as the
`OpenBim.Ifc` NuGet package, are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#327)

- The `OpenBim.Ifc` NuGet package: C# over the versioned C ABI of
  `openbim-ifc-capi` (`openbim_ifc_v0_1_*`, ABI 0.1.5 or later), for
  `net8.0` and `netstandard2.0` (.NET Framework 4.6.2 and later), with the
  native library for `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`,
  `osx-x64` and `osx-arm64`. Build targets copy the Windows libraries into
  a .NET Framework project's output, and the assembly loads them from next
  to itself.
- `IfcModel`, an `IDisposable` over a `SafeHandle`: `Parse`, `Open`
  (owned or memory-mapped), `ParseIfcXml`, `Write`, `WriteIfcXml`,
  `Count`, `Schema`, `Diagnostics`, `Ids`, `IdsOfType`,
  `IdsOfTypeIncludingSubtypes`, `TypeOf`, `Attributes`, `Attribute`,
  `SetAttribute`, `Add`, `Remove` and `DanglingReferences`.
- Attributes by name (#326): `AttributeNames` (`AttributeInfo` records),
  `AttributeByName` and `SetAttributeByName`, with the codes
  `unknown-attribute` and `derived-attribute`.
- The #244 surface: `ParseOptions` (`Lenient`), the `Header` record, read
  and replaced; `Validate` with a `ValidationReport`; ifcXML in the native
  and XSD layouts; `UnreachableProducts`.
- The #123 domain views as C# records: `PropertySets`, `ResolveUnit`,
  `SpatialTree`, `Classifications`, `Material`, `Systems`, `Cost` and
  `Georeferencing`; and writing property sets as one checked transaction
  (`SetProperties` with `PropertyEdit`s, `SetProperty`, `RemoveProperty`),
  checked against the PSD/QTO catalog the native library embeds.
- `Value`, a closed record hierarchy keeping `$` and `*`, `.U.` and
  `.F.`, integer and real, and typed wrappers distinct; `EquatableList<T>`
  so records compare by value.
- `IfcException`, carrying the stable error `Code` every binding shares and
  the C ABI's `IfcStatus`.
- `IfcLibrary.Version` and `IfcLibrary.LiveModels`.
