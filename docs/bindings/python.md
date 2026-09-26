# Python

`openbim-ifc` is the Python binding of the IFC core
([`openbim-ifc-py`](/reference/crates/openbim-ifc-py)), published to PyPI as
compiled wheels.

```bash
pip install openbim-ifc
```

## Read, edit and write

<!-- SNIPPET:py-read-edit-write -->

```python
from openbim_ifc import IfcModel, Text

model = IfcModel.parse(data)  # or IfcModel.open("model.ifc")
schema = model.schema  # "IFC4"

for wall in model.ids_of_type("IfcWall"):
    name = model.attribute(wall, 2)  # Text(value='Wall')
    model.set_attribute(wall, 2, Text(f"{name.value} (checked)"))

data = model.write()  # bytes, ready to save
```

<!-- /SNIPPET -->

Attribute values are frozen dataclasses, one per STEP form, so nothing is
lost in a round trip: `Null()` is `$`, `Derived()` is `*`, `Unknown()` is
`.U.` and is never mistaken for `Bool(False)`. Python integers are
unbounded, so 64-bit IFC integers need no special type.

Every failure raises `openbim_ifc.IfcError`, whose `code` is shared with the
JavaScript and C bindings.

## API

Generated from the `openbim_ifc` package source.

<!-- API:PYTHON:BEGIN -->

| Member | Description |
| --- | --- |
| `IfcModel()` | An empty model. |
| `IfcModel.parse(data: bytes) -> IfcModel` | Parse a STEP (`.ifc`) file from its bytes. |
| `IfcModel.open(path: Union[str, os.PathLike[str]], *, mapped: bool = False) -> IfcModel` | Read a STEP (`.ifc`) file from disk. |
| `model.write() -> bytes` | Serialize as STEP bytes. |
| `len(model) -> int` | Number of entities. |
| `model.schema: Optional[str]` | The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `None`. |
| `model.diagnostics() -> List[str]` | Non-fatal problems found while reading. |
| `model.ids() -> List[int]` | Every entity id, in file order. |
| `model.ids_of_type(type_name: str) -> List[int]` | Ids of every entity of exactly `type_name` (case-insensitive). |
| `model.ids_of_type_including_subtypes(type_name: str) -> List[int]` | Ids of `type_name` or any subtype, per the file's schema. |
| `model.type_of(id: int) -> str` | The upper-case type name of entity `id`. |
| `model.attributes(id: int) -> List[Value]` | Every attribute of entity `id`, in declaration order. |
| `model.attribute(id: int, index: int) -> Value` | Attribute `index` of entity `id`; `Null` past the end. |
| `model.set_attribute(id: int, index: int, value: Value) -> Value` | Set attribute `index` of entity `id`; returns the old value. |
| `model.add(type_name: str, attributes: Iterable[Value]) -> int` | Append an entity; returns its new id. |
| `model.remove(id: int) -> None` | Remove entity `id`, leaving references to it dangling. |
| `model.dangling_references() -> List[Tuple[int, int]]` | Every `(from, to)` pair where `to` does not exist. |

Attribute values are frozen dataclasses in `openbim_ifc` (from `openbim_ifc.values`):

| Value | Meaning |
| --- | --- |
| `Null()` | `$` -- the attribute is not set. |
| `Derived()` | `*` -- derived in a supertype; distinct from `$`. |
| `Bool(value: bool)` | `.T.` or `.F.`. |
| `Unknown()` | `.U.` -- the third logical state. |
| `Integer(value: int)` | An integer literal (64-bit). |
| `Real(value: float)` | A real literal; must be finite. |
| `Text(value: str)` | A string. |
| `Binary(value: str)` | A binary literal, as its hex digits. |
| `Enum(value: str)` | An enumeration constant, without dots: `Enum("ELEMENT")`. |
| `Ref(id: int)` | A reference to entity `#id`. |
| `List(items: Tuple[Value, ...])` | An aggregate (list, set, array or bag). |
| `Typed(type: str, value: Value)` | A typed wrapper such as `IFCLENGTHMEASURE(2.5)`. |

<!-- API:PYTHON:END -->
