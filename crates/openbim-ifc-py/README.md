# openbim-ifc (Python)

Read, edit and write IFC STEP files from Python. A thin binding over the
[`openbim-ifc`](https://github.com/openbimrs/ifc) Rust crates: one abi3
wheel for CPython 3.9 and later.

```python
from openbim_ifc import IfcModel, Text

with open("model.ifc", "rb") as f:
    model = IfcModel.parse(f.read())

print(model.schema, len(model))
for id in model.ids_of_type_including_subtypes("IfcWall"):
    print(id, model.attribute(id, 2))      # Name, e.g. Text('Wall')

model.set_attribute(id, 2, Text("Renamed"))
with open("out.ifc", "wb") as f:
    f.write(model.write())
```

## Values

Attribute values are frozen dataclasses, one per STEP form: `Null` (`$`),
`Derived` (`*`), `Bool`, `Unknown` (`.U.`), `Integer`, `Real`, `Text`,
`Binary`, `Enum`, `Ref` (`#42`), `List`, and `Typed`
(`IFCLENGTHMEASURE(2.5)`). They keep every distinction IFC makes, so a
file read and written back through Python is unchanged.

Bare Python values are refused with `TypeError`: `3` could be an `Integer`
or a `Real`, `"x"` a `Text` or an `Enum`, and the binding does not guess.

## Errors

Every failure raises `IfcError` with a stable `code`: `parse`, `write`,
`missing-entity`, `invalid-value`, `out-of-range` or `unsupported-schema`.
The codes are shared with the JavaScript and C bindings.

## Threads

`parse` releases the GIL while it reads. A model can be passed between
threads; the GIL serialises calls on it.

## Scope

The record model only: entities, attributes, the STEP codec, exact and
subtype queries. Domain views (properties, quantities, geometry) are not
bound yet. The wheel is built and tested in CI; it is not on PyPI yet.

## Build

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
