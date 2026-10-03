# Python

`openbim-ifc` is the Python binding of the IFC core
([`openbim-ifc-py`](/reference/crates/openbim-ifc-py)), published to PyPI as
compiled wheels.

```bash
pip install openbim-ifc
```

The binding exposes the record model over STEP -- parse, read and edit
attributes, and write -- plus lenient reads, the file header, validation,
ifcXML, the reachability lint and read-only domain views (see
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

### Domain views

<!-- SNIPPET:py-domain-views -->

```python
from openbim_ifc import IfcModel

model = IfcModel.parse(data)
(wall,) = model.ids_of_type("IfcWall")

# Property sets: the wall's own first, then its type's; values typed.
for pset in model.property_sets(wall):
    for prop in pset.properties:
        print(pset.name, prop.name, prop.value)

classes = model.classifications(wall)  # (Classification(identification=...), ...)
material = model.material(wall)  # MaterialAssignment(kind="layer-set", layers=(...))
tree = model.spatial_tree()  # SpatialTree(nodes=(SpatialNode(kind=...), ...))
```

<!-- /SNIPPET -->

The domain views of the Rust facade cross as frozen dataclasses, keyed by
entity id with the `global_id` where the entity has one; lists are tuples
and IFC values the classes of `openbim_ifc.values`. A view reads the
model as it is at the call.

- **Property sets.** `property_sets(id)` returns `PropertySet`s: the
  object's own, then those its type object holds, an occurrence property
  overriding an inherited one. Each `Property` keeps its declared type
  (`Typed("IFCLENGTHMEASURE", Real(0.2))`), its `kind` and the `unit` it
  states; `resolve_unit(measure_type, unit=None)` resolves that unit, or
  the project default, exactly to SI. Read against IFC2X3, IFC4 or IFC4X3.
- **Spatial tree.** `spatial_tree()` returns a `SpatialTree` of
  `SpatialNode`s with parents, children, contained and referenced elements.
- **Classification.** `classifications(id)` returns the object's own and
  its type's `Classification`s, each with its code, name, location, the
  references above it and the `ClassificationSystem` at the top.
- **Material.** `material(id)` returns the one `MaterialAssignment` that
  applies, its own or its type's, with layers, profiles or constituents and
  a set usage's placement, or `None`.
- **Systems.** `systems()` returns every `System` with members and served
  structures, and the `SystemAnomaly`s it could not honour.
- **Cost.** `cost()` returns the `CostSchedule`s and `CostItem`s with
  nesting, `CostValue` trees (`applied_value` typed) and unit bases.
- **Georeferencing.** `georeferencing()` returns a `MapConversion` per
  coordinate operation (IFC4, IFC4X3), resolved with the project length
  unit.

Refusals raise `IfcError` with the codes every host shares:
`unsupported-schema` for a release a view does not read, `invalid-model`,
`missing-reference`, `budget-exceeded`, `unsupported` and
`wrong-entity-type`. The wheel carries every domain.

#### Writing property sets

<!-- SNIPPET:py-domain-write -->

```python
from openbim_ifc import PropertyEdit, Real, Text, Typed

# Wall #31 inherits FireRating from its type: the write overrides it
# on the wall and never changes the type's shared set.
result = model.set_properties([
    PropertyEdit(31, "Pset_WallCommon", "FireRating", Typed("IFCLABEL", Text("F60"))),
    PropertyEdit(30, "Qto_WallBaseQuantities", "Width", Typed("IFCLENGTHMEASURE", Real(250.0))),
    PropertyEdit.removal(30, "Pset_WallCommon", "IsExternal"),
])
# result.properties: per edit, the entity now holding the value
own = next(s for s in model.property_sets(31) if s.source == "occurrence" and s.name == "Pset_WallCommon")
```

<!-- /SNIPPET -->

`set_properties(edits)` writes and removes property and quantity values as
one checked transaction: every `PropertyEdit`, in order, or, when any is
refused, none, and the model is unchanged. An edit addresses a property
the way `property_sets` reports it, by object, set name and property
name, and its `value` is that property's `value`: a `Typed` `IfcValue`, a
`List` of them for an enumerated or list value, a typed measure for a
quantity. `PropertyEdit.removal(object, set, name)` removes one;
`set_property(object, set, name, value, set_type=None)` and
`remove_property(object, set, name)` are the one-edit forms, and the
result is a `PropertyEditResult` with the id now holding each value.

A missing set is created with its relationship, the object's owner history
and a name-based `GlobalId`; a removal that empties a set removes it. A
value the occurrence inherits from its type is overridden on the
occurrence and never changed on the type's shared set (pass the type
object to change that), and a set or property shared with other objects is
copied before it changes. As for the views, the edit refuses rather than
writes when a value is not admissible: `invalid-value` when the declared
release (IFC2X3, IFC4 or IFC4X3) has no such `IfcValue` member, the
payload does not fit it, a quantity's measure is another, or a stated unit
fixes another measure; `template-violation` when the release's PSD/QTO
catalog entry for a `Pset_`/`Qto_` set (data type, enumeration, form,
quantity kind, a property it does not declare) or the property's own
enumeration refuses it; `missing-property` for a removal of a property the
object does not state (one it only inherits included); `unsupported` for a
bounded, table, reference or complex value; and `wrong-entity-type` for a
set type that disagrees with the set. The wheel carries the PSD/QTO
catalog; a build without its `property-catalog` feature refuses a write to
a `Pset_`/`Qto_` set with `feature-disabled`.

Not bound yet: checked multi-edit transactions over arbitrary entities
(`Transaction`, `Applied`, `Conflict`), deferred until a host asks for
them. Use the Rust crates for those.

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
| `model.property_sets(id: int) -> List[PropertySet]` | The property sets, quantity sets and predefined property sets of object `id`: its own first, then those its type object holds, an occurrence property overriding an inherited one of the same name. |
| `model.resolve_unit(measure_type: str, unit: Optional[int] = None) -> ResolvedUnit` | The effective unit of a `measure_type` value (`"IFCAREAMEASURE"`): `unit` when given (a property's stated unit), otherwise the project default, resolved exactly to SI. |
| `model.set_properties(edits: Iterable[PropertyEdit]) -> PropertyEditResult` | Write and remove property and quantity values as one checked transaction: every edit, in order, or none. |
| `model.set_property(object: int, set: str, name: str, value: Value, *, set_type: Optional[str] = None) -> int` | Write one value (:meth:`set_properties` with one edit); returns the id of the entity holding it. |
| `model.remove_property(object: int, set: str, name: str) -> None` | Remove one property from `object`'s own set (:meth:`set_properties` with one edit). |
| `model.spatial_tree() -> SpatialTree` | The spatial containment tree: every container with its parent, sub-containers and contained elements. |
| `model.classifications(id: int) -> List[Classification]` | The classifications of object `id`: its own, then its type's. |
| `model.material(id: int) -> Optional[MaterialAssignment]` | The material association of object `id`, its own or its type's, or `None`. |
| `model.systems() -> Systems` | Every system with its members and served structures, and the memberships the reader could not honour. |
| `model.cost() -> Cost` | Every cost schedule and cost item; values as authored, typed. |
| `model.georeferencing() -> List[MapConversion]` | Every coordinate operation resolved with the project length unit; empty when the model has none. |
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
