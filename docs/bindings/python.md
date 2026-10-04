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
    # Names resolve against the release the header declares.
    name = model.attribute_by_name(wall, "Name")  # Text(value='Wall')
    model.set_attribute_by_name(wall, "Name", Text(f"{name.value} (checked)"))

data = model.write()  # bytes, ready to save
```

<!-- /SNIPPET -->

Attributes are addressed by name or by position. `attribute_by_name(id,
"Name")` and `set_attribute_by_name(id, "Name", value)` resolve the name
against the release the file's header declares, so the same code reads
`IfcTask.Status` from slot 6 of an IFC2X3 file and slot 7 of an IFC4 one.
`attribute_names(id)` lists every explicit attribute in slot order,
inherited first, as frozen `AttributeInfo(name, index, type_name,
optional, aggregate, derived, declared_by)` records. Names match
case-insensitively and come back in the schema's spelling. `INVERSE`
attributes hold no slot and are unknown names (`unknown-attribute`). A
slot the entity's type derives is listed with `derived=True`, reads as
stored (`Derived()`) and refuses a write (`derived-attribute`). An entity
type the declared release does not have, or a release the build leaves
out, is `unsupported-schema`. `attribute(id, index)` and
`set_attribute(id, index, value)` stay the raw, release-independent slot
access.

Attribute values are frozen dataclasses, one per STEP form, so nothing is
lost in a round trip: `Null()` is `$`, `Derived()` is `*`, `Unknown()` is
`.U.` and is never mistaken for `Bool(False)`. Python integers are
unbounded, so 64-bit IFC integers need no special type.

Every failure raises `openbim_ifc.IfcError`, whose `code` is shared with the
JavaScript and C bindings.

## Entities, plain values and property sets

The calls above are exact and id-based. A pure-Python layer over them gives
the access Python users expect: entities as objects, plain Python values,
property sets as mappings, and a pandas export. It adds no IFC semantics:
every read and write is one of the calls above.

<!-- SNIPPET:py-entities -->

```python
import openbim_ifc

model = openbim_ifc.open(path)  # IfcModel.open(path)

for wall in model.by_type("IfcWall"):  # subtypes included
    print(wall.id, wall.type, wall.Name, wall.GlobalId)

wall = model[31]  # or model.by_id(31); 31 in model is True
history = wall.OwnerHistory  # a reference: Entity #5
placement = wall.ObjectPlacement  # $: None
exact = wall.raw("Name")  # Text('Wall B'), the lossless value
assert wall.is_a("IfcBuildingElement") and not wall.is_a("IfcSlab")
```

<!-- /SNIPPET -->

`openbim_ifc.open(path)` is `IfcModel.open(path)`. `model[id]` and
`model.by_id(id)` return an `Entity`: a light view holding the model and
the id, no data, so every read sees the model as it is then. `model[id]`
raises `KeyError` for an absent id, `by_id` raises `IfcError`
(`missing-entity`). `entity.id`, `entity.type` (upper-case, `"IFCWALL"`)
and `entity.model` describe it; two views are equal, and hash alike, when
they name the same id of the same model. `entity.is_a("IfcWall")` is true
for the type and its subtypes, per the file's schema.

`entity.Name` reads through `attribute_by_name`, so names resolve against
the declared release and match case-insensitively (`entity.name` works
too). An unknown name, an `INVERSE` attribute included, raises
`AttributeError`, chained from the `unknown-attribute` `IfcError`, so
`hasattr` and `getattr(entity, name, default)` work. `entity.get(name)`
is the same read with a precise return type (`PlainValue`; `entity.Name`
is typed `Any`), raising `IfcError` itself. `entity.raw(name)` returns the
exact tagged value.

### What a plain value keeps

| Tagged value | Plain value | Lossy? |
| --- | --- | --- |
| `Null()` (`$`) | `None` | no |
| `Derived()` (`*`) | `None` | yes: same as `$` |
| `Unknown()` (`.U.`) | `None` | yes: same as `$` |
| `Bool(b)` | `b` | no |
| `Integer(i)`, `Real(r)` | `int`, `float` | no: the Python type keeps it |
| `Text(s)` | `s` | no |
| `Enum(s)` | `s` | yes: same as text |
| `Binary(h)` | `Binary(h)`, unchanged | no: no plain form without decoding |
| `Ref(id)` | `Entity` of the same model | no |
| `List(items)` | `tuple` of plain values | no |
| `Typed(t, v)` | the plain value of `v` | yes: the type name (`IFCLABEL`, `IFCLENGTHMEASURE`) is dropped |

A plain value carries no unit: a quantity reads as the number the file
states, in the unit it states (`200.0` for a width in millimetres). Use
`entity.raw(name)` for the exact attribute value, and
`entity.property_sets()` (the records of `property_sets`) for a
property's declared type and unit, which `resolve_unit` resolves.

### Writing

<!-- SNIPPET:py-entity-write -->

```python
from openbim_ifc import Text

wall = model[31]
wall.Name = Text("Wall B (checked)")  # set_attribute_by_name
wall.Description = None  # $
wall.OwnerHistory = model[5]  # a reference
# wall.Name = "x" raises TypeError: Text or Enum? The binding does not guess.
old = wall.set("Name", Text("Wall B"))  # returns the old tagged value
```

<!-- /SNIPPET -->

`entity.Name = value` writes through `set_attribute_by_name`, and
`entity.set(name, value)` does the same and returns the old tagged value.
A write takes a tagged value, an `Entity` (a reference; one from another
model is refused with `ValueError`), `None` (`$`), or a tuple or list of
these (an aggregate). A bare `str`, `int`, `float` or `bool` raises
`TypeError`: `"x"` could be a `Text` or an `Enum`, `1` an `Integer` or a
`Real`, and the binding does not guess. A refused write changes nothing;
an unknown name raises `AttributeError`, a derived one `IfcError`
(`derived-attribute`). The view's own members (`id`, `type`, ...) cannot be
assigned.

### Iteration and filtering

<!-- SNIPPET:py-iteration -->

```python
count = len(model)  # entities
present = 31 in model  # True; an Entity of this model works too
every = list(model)  # Entity views, in file order
walls = model.by_type("IfcBuildingElement")  # IfcWall #30, #31
exact = model.by_type("IfcBuildingElement", include_subtypes=False)  # []
```

<!-- /SNIPPET -->

`model.by_type(name)` returns the entities of a type and its subtypes in
file order; `include_subtypes=False` keeps the exact type. `len(model)`
counts entities, `id in model` takes an id or an `Entity`, and
`iter(model)` yields every entity. A type name the schema lacks gives
`[]` and `is_a` gives `False`; neither raises.

### Property sets as mappings

<!-- SNIPPET:py-psets -->

```python
from openbim_ifc import Text, Typed

wall = model[30]
common = wall.psets["Pset_WallCommon"]
external = common["IsExternal"]  # False: the wall's own value
rating = common["FireRating"]  # 'F30', inherited from its type
width = wall.qtos["Qto_WallBaseQuantities"]["Width"]  # 200.0, in its stated unit

wall.set_property("Pset_WallCommon", "FireRating", Typed("IFCLABEL", Text("F60")))
assert wall.psets["Pset_WallCommon"]["FireRating"] == "F60"  # a fresh snapshot
```

<!-- /SNIPPET -->

`entity.psets` maps each property set (and predefined property set) name
to a read-only mapping of property name to value, built from
`property_sets`: the object's own values and those its type object holds,
an occurrence value overriding an inherited one. `entity.qtos` holds the
quantity sets (`IfcElementQuantity`) the same way. Both are snapshots
taken at access; read them again after a write. Values are plain: a
single, enumerated (a tuple of the selected values), list or reference
property gives its plain value; a complex property or quantity, a mapping
of its members; a bounded or table value, which has no single plain form,
its exact `Property` record. The mappings cannot be assigned to: write
with `entity.set_property(set, name, value, set_type=None)` and
`entity.remove_property(set, name)`, which are `set_property` and
`remove_property` on that entity, one checked transaction each.

### pandas

<!-- SNIPPET:py-dataframe -->

```python
frame = model.to_dataframe("IfcWall")  # needs openbim-ifc[pandas]
# index: id; columns: type, GlobalId, Name, then "Set.Property"
external = frame["Pset_WallCommon.IsExternal"]  # #30 False, #31 True
```

<!-- /SNIPPET -->

`model.to_dataframe(type="IfcProduct", *, psets=True, qtos=True,
attributes=("GlobalId", "Name"), include_subtypes=True)` builds one row per
entity `by_type` returns, indexed by id (`id`), with a `type` column, a
column per attribute and a `"Set.Property"` column per property and
quantity any row has; a row without one holds pandas' missing marker.
Cells are the plain values above. pandas is the optional extra
`openbim-ifc[pandas]` (`pip install 'openbim-ifc[pandas]'`), imported only
by this call; without it the call raises `ImportError` naming the extra.

### Types

The package ships inline types and `py.typed`; the snippets on this page
pass `mypy --strict`, which the Python test run checks. `entity.Name` is
`Any`, since attributes vary per entity; `entity.get(name)` is
`PlainValue`, `entity.psets` a `Mapping[str, Mapping[str,
PropertyValue]]`, and a write takes an `Assignable`.

### From IfcOpenShell

The table maps IfcOpenShell's documented Python API
([`ifcopenshell.file`](https://docs.ifcopenshell.org/autoapi/ifcopenshell/file/index.html),
[`entity_instance`](https://docs.ifcopenshell.org/autoapi/ifcopenshell/entity_instance/index.html),
[`util.element`](https://docs.ifcopenshell.org/autoapi/ifcopenshell/util/element/index.html))
to this package.

| IfcOpenShell | openbim_ifc | Differences |
| --- | --- | --- |
| `ifcopenshell.open(path)` | `openbim_ifc.open(path)` | STEP only; ifcXML is `IfcModel.parse_ifcxml(data)` |
| `model.by_type("IfcWall")` | `model.by_type("IfcWall")` | Both include subtypes; this returns a list, and `[]` for a type the schema lacks, where IfcOpenShell raises |
| `model.by_type("IfcWall", include_subtypes=False)` | the same | |
| `model.by_id(42)` | `model.by_id(42)` or `model[42]` | An absent id raises `IfcError` (`missing-entity`) or `KeyError`, not `RuntimeError` |
| `entity.Name` | `entity.Name` | A select or defined-type value comes back as its payload, not a wrapper; `.U.` reads `None`; see the table above |
| `entity.Name = "x"` | `entity.Name = Text("x")` | A bare `str` is refused: the value says what it is |
| `entity.id()`, `entity.is_a()` | `entity.id`, `entity.type` | `id` is a property; the type name is upper-case (`IFCWALL`) |
| `entity.is_a("IfcWall")` | `entity.is_a("IfcWall")` | |
| `ifcopenshell.util.element.get_psets(entity)` | `entity.psets` and `entity.qtos` | `get_psets` returns property and quantity sets together unless `psets_only` or `qtos_only` is passed; here they are separate. Sets carry no `"id"` key (`entity.property_sets()` has every id) and are read-only |
| `get_psets(entity, should_inherit=False)` | `entity.property_sets()`, keeping `source == "occurrence"` | |
| `ifcopenshell.api.pset.edit_pset(...)` | `entity.set_property(set, name, value)` | One property per call, a checked transaction; `model.set_properties` batches |

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

## Creating entities

<!-- SNIPPET:py-authoring -->

```python
from openbim_ifc import AuthorOp, Enum, Text, handle

result = model.author(
    [
        AuthorOp.project({"Name": Text("Demo")}),  # 0
        AuthorOp.placement(),  # 1: at the origin
        AuthorOp.spatial("IfcSite", handle(0), placement=handle(1)),  # 2
        AuthorOp.spatial("IfcBuilding", handle(2)),  # 3
        AuthorOp.placement(relative_to=handle(1)),  # 4
        AuthorOp.spatial("IfcBuildingStorey", handle(3), placement=handle(4)),  # 5
        AuthorOp.type_object("IfcWallType", {"PredefinedType": Enum("STANDARD")}),  # 6
        AuthorOp.placement(relative_to=handle(4), location=(1, 2, 0)),  # 7
        AuthorOp.product(
            "IfcWall",
            {"Name": Text("Wall")},
            container=handle(5),  # IfcRelContainedInSpatialStructure
            placement=handle(7),
            type_object=handle(6),  # IfcRelDefinesByType
        ),
    ]
)
wall = result.ids[8]  # every IfcRoot got a GlobalId
```

<!-- /SNIPPET -->

`model.author(ops)`, with `AuthorOp` built by its class methods, creates and
edits entities as one checked transaction against the release the header
declares: every operation, in order, or, when any is refused, none, and the
model is unchanged. An operation names the entity an earlier operation of
the same batch produced by `openbim_ifc.handle(index)`, anywhere an id goes,
attribute values included, and the result holds per operation the id its
entity received. `create_entity(type_name, attributes)` and
`remove_with_relationships(id)` are the one-operation forms. A model built
from nothing needs a header naming its release first (`set_header`).

| Operation | What it writes |
| --- | --- |
| `create` | one entity by `type` and named `attributes` |
| `edit` | named `attributes` of `entity`; the whole entity is checked again |
| `remove` | removes `entity` and takes it out of every relationship; a relationship left without an end goes too |
| `project` | the model's one `Ifc_project` |
| `spatial` | a spatial element of `type` and its `Ifc_rel_aggregates` under `parent` |
| `product` | a product, its `Ifc_rel_contained_in_spatial_structure` in `container` and its `Ifc_rel_defines_by_type` by `type_object` |
| `type_object` | a type object (`Ifc_wall_type`, ...) |
| `assign_type`, `contain`, `aggregate` | one relationship; an object already related is refused |
| `placement` | an `Ifc_local_placement` over an `Ifc_axis2Placement3D` at `location`, relative to `relative_to`; `axis` and `ref_direction` both or neither |
| `owner_history` | an `Ifc_owner_history` with its person, organization and application |

Every record is built by attribute name through `ifc-author` against the
declared release, and refused with the shared codes: a type the release does
not declare (`unsupported-schema`) or an abstract one (`wrong-entity-type`);
an unknown name (`unknown-attribute`); a value of the wrong type or form, or
an aggregate outside its declared bounds (`invalid-value`); a required
attribute left unset (`missing-attribute`); a derived one set
(`derived-attribute`); a reference to an entity that does not exist
(`missing-reference`) or of a type the attribute does not accept
(`wrong-entity-type`). An object is contained, aggregated and typed once and
a model holds one `IfcProject` (`invalid-model`); a removal an entity other
than a relationship still needs is `still-referenced`.

An `IfcRoot` created without a `GlobalId` gets a fresh one. `OwnerHistory`
is never invented: a builder writes the one it is given on every record it
creates, and IFC2X3, which requires it, refuses a record without
(`missing-attribute`). The ids the batch returns are the ones `model[id]`
wraps as an `Entity`.

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
| `model.attribute_names(id: int) -> List[AttributeInfo]` | Every explicit attribute of entity `id` in slot order, inherited first, as the release the header declares defines them. `INVERSE` attributes hold no slot and are not listed. |
| `model.attribute_by_name(id: int, name: str) -> Value` | Attribute `name` of entity `id`, matched case-insensitively (`"Name"`) and resolved against the declared release; `Null` when the record stops before its slot. |
| `model.set_attribute_by_name(id: int, name: str, value: Value) -> Value` | Set attribute `name` of entity `id`; returns the old value. A derived attribute raises `derived-attribute`, an unknown name `unknown-attribute`, and a refused write changes nothing. |
| `model.add(type_name: str, attributes: Iterable[Value]) -> int` | Append an entity; returns its new id. |
| `model.remove(id: int) -> None` | Remove entity `id`, leaving references to it dangling. |
| `model.author(ops: Iterable[AuthorOp]) -> AuthoringResult` | Apply authoring operations as one checked transaction against the release the header declares: every operation, in order, or none. |
| `model.create_entity(type_name: str, attributes: Optional[Mapping[str, Value]] = None) -> int` | Create one entity of `type_name` from named attributes, checked against the declared release (:meth:`author` with one :meth:`AuthorOp.create`); returns its id. |
| `model.remove_with_relationships(id: int) -> None` | Remove entity `id` with the relationships that reference it, leaving nothing dangling (:meth:`author` with one :meth:`AuthorOp.remove`). Raises `still-referenced` while an entity other than a relationship needs it. |
| `model.dangling_references() -> List[Tuple[int, int]]` | Every `(from, to)` pair where `to` does not exist. |
| `model.by_id(id: int) -> Entity` | Entity `#id` as an :class:`Entity`; raises :class:`openbim_ifc.IfcError` (`missing-entity`) when absent. |
| `model[id] -> Entity` | `model[id]`: :meth:`by_id`, raising `KeyError` when absent. |
| `id in model -> bool` | `id in model` (an `int`) or `entity in model`. |
| `iter(model) -> Iterator[Entity]` | `iter(model)`: every entity as an :class:`Entity`, in file order. |
| `model.by_type(type_name: str, *, include_subtypes: bool = True) -> list[Entity]` | Every entity of `type_name` (case-insensitive) as an :class:`Entity`, in file order; with `include_subtypes=False` only those of exactly that type. A name the schema lacks gives `[]`. |
| `model.to_dataframe(type: str = 'IfcProduct', *, psets: bool = True, qtos: bool = True, attributes: Sequence[str] = ('GlobalId', 'Name'), include_subtypes: bool = True) -> pandas.DataFrame` | A pandas `DataFrame` of the entities :meth:`by_type` returns: one row per entity, indexed by id, with a `type` column, one column per name in `attributes`, and one `"Set.Property"` column per property (`psets`) and quantity (`qtos`) any row has. |

An `Entity` (from `model[id]`, `by_id`, `by_type` or `iter(model)`) also reads and writes every IFC attribute by name, `wall.Name`:

| Member | Description |
| --- | --- |
| `entity.id: int` | The entity's `#id`. |
| `entity.model: IfcModel` | The model the entity belongs to. |
| `entity.type: str` | The upper-case type name, e.g. `"IFCWALL"`. |
| `entity.is_a(type_name: str) -> bool` | Whether the entity is a `type_name` or a subtype of it, per the file's schema; case-insensitive. A name the schema lacks is `False`. |
| `entity.get(name: str) -> PlainValue` | Attribute `name` as a plain value; `entity.Name` with the precise type. Raises :class:`openbim_ifc.IfcError` as :meth:`IfcModel.attribute_by_name` does. |
| `entity.raw(name: str) -> Value` | Attribute `name` as the exact tagged value, e.g. `Typed("IFCLABEL", Text("W1"))` or `Derived()`. |
| `entity.set(name: str, value: Assignable) -> Value` | Write attribute `name` (`entity.Name = value`); returns the old tagged value. A refused write changes nothing. |
| `entity.attribute_names() -> list[AttributeInfo]` | Every explicit attribute, in slot order, as :meth:`IfcModel.attribute_names` lists them. |
| `entity.property_sets() -> list[PropertySet]` | The exact records :meth:`IfcModel.property_sets` returns. |
| `entity.psets: Mapping[str, Mapping[str, PropertyValue]]` | Property sets and predefined property sets by name, each a read-only mapping of property name to :data:`PropertyValue`. |
| `entity.qtos: Mapping[str, Mapping[str, PropertyValue]]` | Quantity sets (`IfcElementQuantity`) by name, as :attr:`psets`. |
| `entity.set_property(set: str, name: str, value: Value, *, set_type: Optional[str] = None) -> int` | Write one property or quantity value: :meth:`IfcModel.set_property` on this entity. `value` is exact, e.g. `Typed("IFCLABEL", Text("F60"))`. Returns the id of the entity holding it. |
| `entity.remove_property(set: str, name: str) -> None` | Remove one property from the entity's own set: :meth:`IfcModel.remove_property`. |

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
