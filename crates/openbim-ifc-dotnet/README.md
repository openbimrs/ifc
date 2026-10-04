# OpenBim.Ifc (.NET)

Read, edit, validate and write IFC files from .NET: C#, F#, or a Revit,
Navisworks, Tekla or Dynamo add-in. A thin binding over the
[`openbim-ifc`](https://github.com/openbimrs/ifc) Rust crates, through
their versioned C ABI; the native library ships inside the package.

```sh
dotnet add package OpenBim.Ifc
```

Documentation: [.NET guide](https://openbimrs.github.io/ifc/bindings/dotnet)
· [source and issues](https://github.com/openbimrs/ifc)

```csharp
using OpenBim.Ifc;

using var model = IfcModel.Open("model.ifc");
foreach (var wall in model.IdsOfTypeIncludingSubtypes("IfcWall"))
{
    Console.WriteLine($"#{wall} {model.Attribute(wall, 2)}"); // 'Wall'
    model.SetAttribute(wall, 2, new Value.Text("Renamed"));
}
File.WriteAllBytes("out.ifc", model.Write());
```

## Platforms

| Target framework | Hosts |
| --- | --- |
| `net8.0` | .NET 8 and later |
| `netstandard2.0` | .NET Framework 4.6.2 and later (Revit 2024 and older), Mono, older .NET |

The package carries the native library for `win-x64`, `win-arm64`,
`linux-x64`, `linux-arm64`, `osx-x64` and `osx-arm64`, all 64-bit. A .NET
application picks its runtime's copy itself. A .NET Framework project gets
the Windows libraries copied into its output by the package's build
targets, and the assembly loads them from next to itself, so an add-in
needs no `RuntimeIdentifier` and no copy step.

## Values

Attribute values are a closed record hierarchy, one case per STEP form:
`Value.Null` (`$`), `Value.Derived` (`*`), `Value.Bool`, `Value.Unknown`
(`.U.`), `Value.Integer` (64-bit), `Value.Real`, `Value.Text`,
`Value.Binary`, `Value.Enum`, `Value.Ref` (`#42`), `Value.List` and
`Value.Typed` (`IFCLENGTHMEASURE(2.5)`). They keep every distinction IFC
makes, so a file read and written back through .NET is unchanged; they
compare by value, lists included.

## Errors

Every refusal throws `IfcException` with a stable `Code` (`parse`,
`missing-entity`, `invalid-value`, `unsupported-schema`, ...) shared with
the JavaScript, Python and C bindings, and the C ABI's `Status`. A refused
edit leaves the model unchanged.

## Models and threads

`IfcModel` is `IDisposable`: dispose it to release the native model, or
the finalizer will. Calls on one model are serialised by the library, so a
model may be shared between threads; calls on different models run in
parallel.

## Scope

The record model -- entities, attributes, the STEP codec, exact and
subtype queries -- plus lenient reads (`ParseOptions`), the STEP header,
validation, ifcXML, the reachability lint, the read-only domain views
(property sets and quantities, the spatial tree, classification,
materials, systems, cost, georeferencing) as C# records, and writing
property sets and quantities as one checked transaction
(`SetProperties`), checked against the embedded PSD/QTO catalog. Geometry
and checked multi-edit transactions over arbitrary entities are not bound
yet.

## Build from source

The package is C# in `dotnet/`; the Cargo crate around it holds its
version, changelog and the tests that hold the C# declarations to the C
header and the shared records. With the .NET 8 SDK and Rust:

```sh
python3 crates/openbim-ifc-dotnet/scripts/check-dotnet.py
```

builds the C ABI library for this machine, packs the `.nupkg` and runs the
C# suite against it from a fresh project.

## License

`AGPL-3.0-or-later`; the package carries `LICENSE` and `NOTICE`. See the
repository's `LICENSING.md`.
