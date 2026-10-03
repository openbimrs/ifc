# Python

`openbim-ifc` is the Python binding of the IFC core
([`openbim-ifc-py`](/reference/crates/openbim-ifc-py)), published to PyPI as
compiled wheels.

```bash
pip install openbim-ifc
```

The binding exposes the record model over STEP -- parse, read and edit
attributes, and write -- plus lenient reads, the file header, validation,
ifcXML and the reachability lint (see
[Beyond the record model](#beyond-the-record-model)).

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

## Beyond the record model

<!-- SNIPPET:py-beyond-records -->

```python
import dataclasses

from openbim_ifc import IfcModel, ParseOptions

# A damaged export: skip what cannot be read, and say what was skipped.
model = IfcModel.parse(data, options=ParseOptions.lenient())
skipped = model.diagnostics()  # one message per recovery

header = model.header  # Header(name=..., author=(...), schema=(...), ...)
model.set_header(dataclasses.replace(header, author=("Reviewer",)))

report = model.validate()  # ValidationReport(conformant=..., findings=(...))
errors = [f for f in report.findings if f.severity == "error"]

xml = model.write_ifcxml()  # lossless ifcXML; or xsd_profile="IFC4"
from_xml = IfcModel.parse_ifcxml(xml)
```

<!-- /SNIPPET -->

- **Lenient reads.** `IfcModel.parse` and `IfcModel.open` take
  `options=ParseOptions(...)`: `on_malformed="skip"`, `check_references`,
  `accept_real_without_point`, or the `ParseOptions.lenient()` preset.
  Every recovery is listed by `diagnostics()`.
- **Header.** `model.header` is a frozen `Header` with every
  `FILE_DESCRIPTION`, `FILE_NAME` and `FILE_SCHEMA` field;
  `set_header()` replaces it.
- **Validation.** `validate(max_findings=None)` checks the model against
  the schema its header declares and returns a frozen `ValidationReport`:
  counts by severity, `conformant`, `truncated`, and `ValidationFinding`s
  sorted by severity, rule, entity and slot.
- **ifcXML.** `write_ifcxml()` and `IfcModel.parse_ifcxml(data)` use this
  library's lossless layout; `xsd_profile="IFC4"` or `"IFC4X3_ADD2"`
  selects the buildingSMART XSD layout, which refuses with `write` what it
  cannot carry exactly.
- **Reachability.** `unreachable_products()` lists products no viewer will
  draw as `UnreachableProduct`s with a stable `reason`.

Not bound yet: checked multi-edit transactions (`Transaction`, `Applied`,
`Conflict`), deferred until a host asks for them, and the domain views
such as property sets or the spatial tree
([#123](https://github.com/openbimrs/ifc/issues/123)), which come next as
opt-in features. Use the Rust crates for those.

## API

Generated from the `openbim_ifc` package source.

<!-- API:PYTHON:BEGIN -->

| Member | Description |
| --- | --- |
| `IfcModel()` | An empty model. |
| `IfcModel.parse(data: bytes, *, options: Optional[ParseOptions] = None) -> IfcModel` | Parse a STEP (`.ifc`) file from its bytes. |
| `IfcModel.parse_ifcxml(data: bytes, *, xsd_profile: Optional[str] = None) -> IfcModel` | Parse an ifcXML document. |
| `IfcModel.open(path: Union[str, os.PathLike[str]], *, mapped: bool = False, options: Optional[ParseOptions] = None) -> IfcModel` | Read a STEP (`.ifc`) file from disk. |
| `model.write() -> bytes` | Serialize as STEP bytes. |
| `model.write_ifcxml(*, xsd_profile: Optional[str] = None) -> bytes` | Serialize as ifcXML bytes, in the layout `parse_ifcxml` reads. |
| `model.header: Header` | The STEP file header: description, name, time stamp, author, ... |
| `model.set_header(header: Header) -> None` | Replace the STEP file header, e.g. with `dataclasses.replace`. |
| `model.validate(max_findings: Optional[int] = None) -> ValidationReport` | Validate against the schema the header declares. |
| `model.unreachable_products() -> List[UnreachableProduct]` | Products no viewer will draw, with a stable `reason`, in id order. |
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
