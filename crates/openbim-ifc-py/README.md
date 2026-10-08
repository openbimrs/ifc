# openbim-ifc (Python)

Read, edit and write IFC STEP files from Python. A thin binding over the
[`openbim-ifc`](https://github.com/openbimrs/ifc) Rust crates: one abi3
wheel for CPython 3.9 and later.

```sh
pip install openbim-ifc
```

Documentation: [Python guide](https://openbimrs.github.io/ifc/bindings/python)
· [source and issues](https://github.com/openbimrs/ifc)

```python
from openbim_ifc import IfcModel, Text

with open("model.ifc", "rb") as f:
    model = IfcModel.parse(f.read())

print(model.schema, len(model))
for id in model.ids_of_type_including_subtypes("IfcWall"):
    print(id, model.attribute_by_name(id, "Name"))  # e.g. Text('Wall')

model.set_attribute_by_name(id, "Name", Text("Renamed"))
with open("out.ifc", "wb") as f:
    f.write(model.write())
```

Names resolve against the release the file's header declares, inherited
attributes included; `attribute_names(id)` lists them in slot order with
their `optional` and `derived` flags. `attribute(id, index)` and
`set_attribute(id, index, value)` address a slot by position.

## Entities, plain values, property sets

A pure-Python layer over those calls reads like IfcOpenShell:

```python
import openbim_ifc
from openbim_ifc import Text

model = openbim_ifc.open("model.ifc")
for wall in model.by_type("IfcWall"):          # subtypes included
    print(wall.Name, wall.psets["Pset_WallCommon"]["IsExternal"])
wall.Name = Text("Renamed")                     # set_attribute_by_name
frame = model.to_dataframe("IfcWall")           # pip install 'openbim-ifc[pandas]'
```

Reads give plain Python values; a typed wrapper's type name, `*` and `.U.`
(both `None`) and the text/enum distinction are lost there, and
`wall.raw("Name")` keeps them. `wall.psets` and `wall.qtos` are read-only
mappings with type values inherited; write with `wall.set_property(...)`.
The package is typed (`py.typed`) and checked with `mypy --strict`.

## Values

Attribute values are frozen dataclasses, one per STEP form: `Null` (`$`),
`Derived` (`*`), `Bool`, `Unknown` (`.U.`), `Integer`, `Real`, `Text`,
`Binary`, `Enum`, `Ref` (`#42`), `List`, and `Typed`
(`IFCLENGTHMEASURE(2.5)`). They keep every distinction IFC makes, so a
file read and written back through Python is unchanged.

Bare Python values are refused with `TypeError`: `3` could be an `Integer`
or a `Real`, `"x"` a `Text` or an `Enum`, and the binding does not guess.

## Errors

Every failure raises `IfcError` with a stable `code` (`parse`,
`missing-entity`, `invalid-value`, `unsupported-schema`, ...; the Python
guide lists them all). The codes are shared with the JavaScript and C
bindings; a code is never renamed or reused.

## Threads

`parse` releases the GIL while it reads. A model can be passed between
threads; the GIL serialises calls on it.

## Scope

The record model -- entities, attributes, the STEP codec, exact and
subtype queries -- plus lenient reads (`ParseOptions`), the STEP header,
validation, ifcXML, the reachability lint and the read-only domain views
(property sets and quantities, the spatial tree, classification, materials,
systems, cost, georeferencing; #123), each as frozen dataclasses,
writing property sets and quantities as one checked transaction
(`set_properties`), the Pythonic layer over them (#332), and creating
entities as one checked transaction (`author` with `AuthorOp`s: entities
by type and named attributes, the spatial structure, placed, contained and
typed products, removal with relationships; #330), and each product's
world placement and Body (`product_placements`; meshes from a
`--features mesh` wheel, #328).

## Build from source

```sh
uv venv && . .venv/bin/activate
uv pip install maturin
maturin develop --release     # or: scripts/check-python.sh to build + test
```

### Faster reading (opt-in)

```sh
maturin build --release --features rusty_alloc
```

Uses the pure-Rust rusty_alloc allocator for the extension's own memory:
reading STEP into a model takes 18-35% less CPU time on seven real IFC
files, at 1-7% less peak memory (#49). Python's own allocator is not
replaced. Off by default because a global allocator is a build-time
choice; the version is pinned exactly.
