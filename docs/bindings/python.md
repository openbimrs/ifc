# Python

`openbim-ifc` is the Python binding of the IFC core
([`openbim-ifc-py`](/reference/crates/openbim-ifc-py)), published to PyPI as
compiled wheels.

```bash
pip install openbim-ifc
```

Task-sized recipes are in the [Python cookbook](/cookbook/python).

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
case-insensitively and come back in the schema's spelling. An `INVERSE`
name (#462) reads through the same call: `attribute_by_name(wall,
"IsDefinedBy")` follows the declared release's `FOR` clause (IFC2X3's
`IsDefinedBy` holds type and property relationships, IFC4's only property
ones beside `IsTypedBy`) and answers a `List` of `Ref`s in ascending id
order for a `SET` or `BAG`; `SET [0:1]` inverses such as
`ContainedInStructure` are a list of zero or one, never unwrapped. A
single-valued inverse (`VoidsElements`) is a `Ref` or `Null`; two
referrers raise `invalid-model`. A write to one raises
`inverse-attribute`. `inverse_attribute_names(id)` lists the declarations
as `InverseAttributeInfo(name, entity, for_attribute, aggregate, lower,
upper, declared_by)`; `inverse_references(id)` lists every referrer as
`InverseReference(id, index)`, the attribute index included, and
`total_inverses(id)` counts the distinct referrers. The reverse index
behind them is built on the first such call and again only after an
edit. A
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
too), and so do `INVERSE` names: `wall.IsDefinedBy` is a tuple of the
relationship entities, `opening.VoidsElements` an `Entity` or `None`.
`entity.inverse_references()` gives `(Entity, index)` pairs and
`entity.total_inverses()` the count. An unknown name raises
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
wall.Name = "Wall B (checked)"  # IfcLabel: written 'Wall B (checked)'
wall.PredefinedType = "standard"  # IfcWallTypeEnum: written .STANDARD.
wall.Description = None  # $
wall.OwnerHistory = model[5]  # a reference, checked to be an IfcOwnerHistory
old = wall.set("Name", Text("Wall B"))  # exact; returns the old tagged value
```

<!-- /SNIPPET -->

`entity.Name = value` writes through `set_attribute_by_name_plain`
(#342), and `entity.set(name, value)` does the same and returns the old
tagged value. A plain value is coerced against the attribute's declared
type in the release the header declares:

| Plain value | Declared type | Written |
| --- | --- | --- |
| `str` | `STRING`, through any defined type (`IfcLabel`) | text, bare: `'x'` |
| `str` | an enumeration | the item it names, in any case: `.STANDARD.` |
| `int` | `INTEGER` / `REAL` | an integer / a real (exact within 2^53) |
| `float` | `REAL` | a real |
| `bool` | `BOOLEAN`, `LOGICAL` | `.T.` / `.F.` |
| tuple or list | an aggregate | each element coerced against the element type |
| `Entity` | an entity type, or a SELECT of them | a reference, once it exists and is of an accepted type |
| `None` | any | `$` |
| any of the above | a SELECT | the one member that takes it, as a typed parameter: `IFCDESCRIPTIVEMEASURE('by layer')` |

A value that does not fit raises `IfcError` with `type-mismatch` (a
number for a label, a name no enumeration item has, a wall where a
placement is declared), one that several SELECT members take
`ambiguous-value` naming them (`"x"` for `IfcValue` is a label, a text, an
identifier, ...): pass the exact value then, `Typed("IFCLABEL",
Text("x"))`. A tagged value is always written exactly as given, nested in a
list too, and an `Entity` from another model is refused with `ValueError`.
A refused write changes nothing; an unknown name raises `AttributeError`, a
derived one `IfcError` (`derived-attribute`). The view's own members (`id`,
`type`, ...) cannot be assigned.

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
| `ifcopenshell.open(path)` | `openbim_ifc.open(path)` | STEP and ifcZIP; ifcXML is `IfcModel.parse_ifcxml(data)` |
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
| `model.by_guid(guid)` | `model.by_guid(guid)` | `None` when absent, where IfcOpenShell raises; a `GlobalId` several entities carry raises `IfcError` (`invalid-model`) naming them all, and `model.duplicate_global_ids()` lists every one |
| `ifcopenshell.guid.new()` | `openbim_ifc.new_guid()` | |
| (no equivalent) | `openbim_ifc.name_based_guid(*parts)` | Reproducible: the same parts give the same `GlobalId` |
| `util.element.get_type(entity)` | `model.type_object(entity.id)` | An id or `None`; two types for one object raise `invalid-model` |
| `util.element.get_types(type)` | `model.occurrences_of(type.id)` | Ids |
| `entity.HasOpenings`, `opening.HasFillings`, `opening.VoidsElements` | the same, or `model.openings_of(id)`, `model.fillings_of(id)`, `model.voided_element_of(id)` | The inverse reads relationship entities; the queries return the elements' ids |
| `entity.IsDefinedBy`, any `INVERSE` name | `entity.IsDefinedBy` | Resolved against the file's release; a single-valued inverse (`VoidsElements`) is an `Entity` or `None`, not a tuple (IfcOpenShell's `unpack_non_aggregate_inverses`) |
| `file.get_inverse(entity, True, True)` | `entity.inverse_references()` | `(Entity, index)` pairs in ascending order; `model.inverse_references(id)` gives `InverseReference(id, index)` |
| `file.get_total_inverses(entity)` | `entity.total_inverses()` | Distinct referencing entities |

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
- **ifcZIP.** `IfcModel.parse` and `IfcModel.open` read an ifcZIP archive,
  recognized by its ZIP signature: its one STEP or ifcXML model (either
  layout), within a decompression budget. `write_ifczip()` writes STEP
  inside as `model.ifc`; `inner="ifcxml"` writes ifcXML as `model.ifcXML`,
  in the layout `xsd_profile` names. An archive is refused with `parse`
  when it holds no `.ifc` or `.ifcXML` entry at its root or several,
  `unsupported` for an encrypted entry or a compression other than Stored
  and Deflate, and `budget-exceeded` for an entry that would inflate past
  4 GiB or 200 times its compressed size.
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

`property_sets(id)` resolves one object. For many, or all, ask once:
`property_sets_many(ids)` (#358) validates the file's property
relationships once for the call instead of once per object, so a pass over
every object is linear in the model where a loop of `property_sets` is
quadratic. Each answer is exactly the per-object call's, or its refusal
as `code` and `message`; no index outlives the call, so a call after an
edit sees the edit. `to_dataframe` uses it.

<!-- SNIPPET:py-property-sets-many -->

```python
every = model.property_sets_many()  # every object definition, one pass
walls = model.property_sets_many([30, 31])  # or the ids given, in order
external = {
    answer.object: [p.value for s in answer.sets for p in s.properties if p.name == "IsExternal"]
    for answer in walls
}
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

## GlobalIds, types and openings

`model.by_guid(global_id)` finds an entity by `GlobalId` through an index
built on the first lookup and kept until the model changes; a `GlobalId`
several entities carry raises `invalid-model` naming them all, and
`model.duplicate_global_ids()` lists every one. `model.type_object(id)` and
`model.occurrences_of(type)` read `IfcRelDefinesByType` in every release
(IFC2X3's `IsDefinedBy` included); `openings_of`, `fillings_of` and
`voided_element_of` read `IfcRelVoidsElement` and `IfcRelFillsElement`.
`openbim_ifc.new_guid()` draws a random version-4 `GlobalId` from the
operating system and `openbim_ifc.name_based_guid(*parts)` derives a
reproducible version-8 one. The
[cookbook](/cookbook/python#find-entities-by-globalid-types-and-openings)
has a recipe.

## Selectors and values by path

`model.select(query, base=None)` returns the entities a
[selector](/guide/selectors) selects (`select_ids` their ids), in ascending
id order: `"IfcWall, Pset_WallCommon.IsExternal=TRUE + IfcSlab"`.
`model.get_value(entity, path)` reads a value by path (`type.Name`,
`Pset_WallCommon.FireRating`, `materials.Name`) as plain Python values, a
property set as a read-only mapping; `model.set_value(entity, path, value)`
writes one as a single checked transaction.
`openbim_ifc.canonical_selector(query)` prints a selector in canonical form
and `openbim_ifc.SELECTOR_GRAMMAR_VERSION` names the grammar. A selector
that does not parse raises `IfcError` with `code` `invalid-selector` and
`span`. The
[cookbook](/cookbook/python#select-entities-and-read-values-by-path) has a
recipe.

## Tables

`model.export_table(query, columns, format="csv", key=None, null="")`
returns the entities a selector selects as a [table](/guide/tables), bytes
in `csv`, `tsv` or `xlsx`: the key column first (`GlobalId`, or `id` with
`key="id"`), then one column per value path. `model.import_table(data,
format="csv", key=None, null="", dry_run=False)` applies a table (bytes,
or a `str` of CSV or TSV) back by its key as one checked transaction and
returns a `TableImport`: `applied`, every `TableChange` (row, column, key,
path, entity, target entity and attribute or property, value before and
after) and every `TableRefusal` (row, column, `code`, `message`, `span`).
Refusals are data, not exceptions: any refusal means nothing was written,
as does `dry_run`. An unchanged table changes nothing. The
[cookbook](/cookbook/python#export-a-table-and-import-it-back) has a
recipe.

## Geometry

Geometry crosses at three levels ([ADR 0021](/adr/0021-bindings-carry-placements-and-opt-in-meshes)):
placements, the exact neutral representation and meshes.
`model.product_placements(ids=None)` is in every wheel: for each product
with a shape (or for the ids given), its world placement and the Body
representation a viewer draws, as frozen dataclasses.

<!-- SNIPPET:py-geometry-placements -->

```python
model = openbim_ifc.open(path)
for product in model.product_placements():
    if product.refusal is not None:
        print((product.id, product.refusal.code, product.refusal.message))
        continue
    # A column-major 4x4 in metres: the origin is its last column.
    x, y, z = product.transform[12:15]
    body = product.representation  # None for an axis-only product
    print((product.type_name, (x, y, z), body and body.representation_type))
```

<!-- /SNIPPET -->

`ProductPlacement.transform` is a 4x4 column-major matrix in metres, a
tuple of 16 floats; `representation` is a `SelectedRepresentation`
(`identifier`, `representation_type`, `context`, `context_type`,
`context_identifier`, `target_view`), or `None` for a product with an
axis or footprint only. A product that cannot be placed or selected does
not raise: its `refusal` is a `GeometryRefusal` with a `code` --
`unsupported`, `invalid-model`, `missing-reference` or `budget-exceeded`
-- the `entity` at fault and a `message`. The call raises only
`unsupported-schema`.

`model.product_meshes(ids=None)` adds triangles, compiled by the reference
backend of `ifc-geometry` with a one-millimetre tolerance while the GIL is
released. It links a geometry kernel, so it is the cargo feature `mesh`,
which the published wheel leaves out (it raises `feature-disabled`); build
a wheel with `maturin build --release --features mesh` to use it:

<!-- SNIPPET:py-geometry-meshes -->

```python
model = openbim_ifc.open(path)
for mesh in model.product_meshes():
    if mesh.refusal is not None:
        print((mesh.id, mesh.refusal.code))  # (65, 'unsupported')
        continue
    # positions: array('f'), x y z per vertex in metres relative to
    # mesh.transform; indices: array('I'), three per triangle.
    print((mesh.type_name, len(mesh.positions) // 3, len(mesh.indices) // 3))
```

<!-- /SNIPPET -->

`ProductMesh.positions` is an `array('f')` (`x, y, z` per vertex) and
`indices` an `array('I')` (three per triangle);
`numpy.frombuffer(mesh.positions, numpy.float32)` reads them without a
copy. Positions are relative to the product's `transform`, which keeps
the large offsets of a georeferenced site in `f64`. A product with no
Body has empty arrays and no refusal; one whose lowering or compilation
is refused carries the typed `refusal`.

Each mesh also carries its `colour`, a `SurfaceColour`
([#449](https://github.com/openbimrs/ifc/issues/449)): `red`, `green`,
`blue` and `transparency` (each in `[0, 1]`; `transparency` is `None`
when the file states none, which reads as opaque), the `source` that
supplied it, `item`, `layer` or `material` in that precedence, the
`source_entity` and the `surface_style`. The colour comes from the Body
item's `IfcStyledItem`, else its styled presentation layer, else the
`IfcMaterialDefinitionRepresentation` of the material the product (or its
type) is made of: an `IfcMaterial`, the one material of a layer or
profile set, or, for a constituent set, the constituent whose
`IfcShapeAspect` names the item. `colour` is `None` when nothing colours
the product, and `colour_refusal` says why it is not resolved, with code
`ambiguous` for a set of several materials or Body items of different
colours. A colour never refuses the mesh.

`model.product_curves(ids=None, representation="Plan")` gives the curves
of a curve representation as polylines
([#458](https://github.com/openbimrs/ifc/issues/458)), from the same
`mesh` wheel: an `Axis`, a `FootPrint`, the curves of an `IfcAnnotation`,
which have no area and so no triangles. `representation` picks the
representation as the context selector does: `"Plan"` (the default) takes
the best drawing curves, `Plan`, then `Annotation`, `FootPrint` and
`Axis`, preferring one in a `PLAN_VIEW` context; `"Axis"`,
`"FootPrint"`, `"Annotation"`, `"Body"` or `"Surface"` name one
`RepresentationIdentifier`; any other name raises `invalid-value`:

<!-- SNIPPET:py-geometry-curves -->

```python
model = openbim_ifc.open(path)
for product in model.product_curves(representation="Axis"):
    for line in product.polylines:
        # positions: array('f'), x y z per vertex in metres relative
        # to product.transform; this curve is a slice of them.
        start = 3 * line.first_vertex
        xyz = product.positions[start : start + 3 * line.vertex_count]
        print((product.id, len(xyz) // 3, line.closed, line.chord_bound))
```

<!-- /SNIPPET -->

Each `ProductCurves` has one `positions` `array('f')`, relative to its
`transform` as a mesh's are, holding every polyline's vertices in turn,
and a `ProductPolyline` per curve: its slice (`first_vertex`,
`vertex_count`), `closed` (the last vertex repeats the first), the IFC
`curve` it came from, and `chord_bound`, the reference backend's
certified bound in metres on how far the exact curve lies from the
polyline (`0.0` for a polyline the file authored), against
`chord_requested`, one millimetre. Trimmed, composite, offset and
intrinsic curves (an `IfcTrimmedCurve` axis, an `IfcCompositeCurve`
footprint) compile ([#506](https://github.com/openbimrs/ifc/issues/506));
a curve the reference compiler has no certified polyline for (an implicit
curve, an offset of an offset) is refused `unsupported`, naming the
product. A representation that also holds text or a fill area (a dimension
and its value) gives its curves; each `IfcTextLiteral`,
`IfcTextLiteralWithExtent` and `IfcAnnotationFillArea` of it is listed
by entity id in `skipped`, not drawn and not a refusal
([#512](https://github.com/openbimrs/ifc/issues/512)).

`product_meshes(ids, representation=...)` meshes a representation other
than the Body the same way, such as `"Surface"`: its surfaces and solids
as one mesh, its curves left to `product_curves`. Every mesh says what it
bounds in `closure`: `"solid"`, `"surface"` (a surface model, a face, a
rectangular trimmed surface: an area and no volume) or `"unknown"` for an
empty or refused mesh. Only a Body mesh carries a colour.

`model.product_geometry(ids=None, encoding="json")` hands over each
product's Body as Axiolid's neutral geometry graph instead
([#367](https://github.com/openbimrs/ifc/issues/367)): exact extrusions,
sweeps, B-splines and unevaluated booleans, for your own kernel. It links
the lowering, so it is the cargo feature `graph`, which the published
wheel leaves out (it raises `feature-disabled`); build a wheel with
`maturin build --release --features graph` (or `mesh,graph`):

<!-- SNIPPET:py-geometry-graphs -->

```python
import json

model = openbim_ifc.open(path)
for product in model.product_geometry():
    if product.refusal is not None:
        print((product.id, product.refusal.code))  # (65, 'unsupported')
        continue
    if product.payload is None:
        continue  # no Body: an axis-only product
    # {"format": "axiolid-geometry-graph", "version": "1.0", "graph":
    # {"nodes": [...], "roots": [...]}}, exact, in world metres.
    envelope = json.loads(product.payload)
    print((product.type_name, envelope["version"], len(envelope["graph"]["nodes"])))
```

<!-- /SNIPPET -->

`ProductGeometry.payload` is Axiolid's versioned wire format (Axiolid
ADR 0085), `{"format": "axiolid-geometry-graph", "version": "1.0",
"graph": {"nodes": [...], "roots": [...]}}`: a `str` for `encoding="json"`,
`bytes` (CBOR, RFC 8949) for `encoding="cbor"`; any other encoding raises
`ValueError`. Its `version` is the lowest the content needs: `1.0` unless a
station carries a seam-snapping window
([#423](https://github.com/openbimrs/ifc/issues/423)), then `1.1`, or a
run's sections do ([#429](https://github.com/openbimrs/ifc/issues/429),
a sectioned solid or surface along such an offset), then `1.2`, or an
offset run continues to a basis end the kernel measures
([#433](https://github.com/openbimrs/ifc/issues/433), an
`IfcOffsetCurveByDistances` along such an offset), then `1.3`, or a
polygonal bounded half-space clips
([#515](https://github.com/openbimrs/ifc/issues/515), an
`IfcPolygonalBoundedHalfSpace` bounded along its `Position`'s Z), then
`1.4` (axiolid/kernel#297, #301, #303, #321). A reader on `axiolid-model` 0.3.8 or
older reads a `1.0` payload and refuses `1.1`; one on 0.3.10 or older
refuses `1.2`, one on 0.3.11 or older `1.3`, one on 0.3.13 or older
`1.4`. `GEOMETRY_FORMAT` and `GEOMETRY_FORMAT_VERSION`
name the format and the newest version the wheel writes, `"1.4"`. Nodes come in insertion order; each is tagged by its
kind, and a reference to another node is its index. The graph is in world
coordinates, metres, with the product's `transform` already applied. A
product with no Body has no payload and no refusal; a refused one carries
the typed `refusal`. A major version of the wire format would be a
breaking release of the wheel (ADR 0021).

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
| `assign_type`, `contain`, `aggregate` | one relationship; an object already related is refused unless `replace` (see [Assigning types](#assigning-types)) |
| `placement` | an `Ifc_local_placement` over an `Ifc_axis2Placement3D` at `location`, relative to `relative_to`; `axis` and `ref_direction` both or neither |
| `owner_history` | an `Ifc_owner_history` with its person, organization and application |
| `shape_representation` | an `IfcShapeRepresentation` of `items` in `context`, its type checked or inferred ([below](#shape-representations)) |
| `assign_representation`, `unassign_representation` | a shape representation on or off a product (its `IfcProductDefinitionShape`) or a type (an `IfcRepresentationMap`) |
| `copy_representation`, `remove_representation` | a copy of a shape representation; its removal with what only it uses |

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

## Assigning types

The type assignment propagates the type to each object when asked (#480);
every option is off by default, and the operation then writes only the
`IfcRelDefinesByType`.

| Option | What it does |
| --- | --- |
| `map_representations=True` | maps each of the type's `RepresentationMaps` onto the object: an `IfcShapeRepresentation` in the map's context with its `RepresentationIdentifier`, `RepresentationType` `MappedRepresentation` and one `IfcMappedItem`, added to the object's `IfcProductDefinitionShape` (created when it has none) |
| `mapping_target` | the `IfcCartesianTransformationOperator` every mapped item uses; by default a fresh identity operator (3D, or 2D for a 2D `MappingOrigin`) per object, so the geometry sits at the object's placement |
| `material_usage=True` | derives the object's material usage from the type's set: an `IfcMaterialLayerSetUsage` (`DirectionSense` `POSITIVE`, `OffsetFromReferenceLine` 0) for a layer set, an `IfcMaterialProfileSetUsage` (no `CardinalPoint`) for a profile set, one per object in its own `IfcRelAssociatesMaterial`; any other material applies through the type |
| `layer_set_direction` | the layer direction, `"AXIS1"` to `"AXIS3"`; by default the one the IFC documentation fixes for the class: `AXIS2` for `IfcWall`, `AXIS3` for `IfcSlab` and `IfcPlate` |
| `predefined_type` | "ignore" (default) leaves `PredefinedType`; "validate" refuses an object whose own value is set, not `NOTDEFINED`, and differs from the one the type provides (for `USERDEFINED`, `ObjectType` against the type's `ElementType`); "sync" unsets the object's (and its `USERDEFINED` `ObjectType`) where the type provides one, as IFC asks |
| `replace=True` | unassigns the previous type instead of refusing: the object leaves its `IfcRelDefinesByType` (removed when left relating nothing), and the representations mapped from the previous type's maps and the usage derived from its set are removed with what only they use |

A type provides a `PredefinedType` when its own is set and not `NOTDEFINED`;
nothing is synchronised where the object or the type has no such attribute
in the declared release (IFC2X3 `IfcWall`, `IfcDoorStyle`). The containment
and aggregation take `replace` too: the object leaves its previous container
or whole, whose relationship goes when it is left relating nothing.

Refused, with the model unchanged: a representation that clashes with one
the object has (same identifier, or unnamed in the same context) or a
product definition shape another product shares, a contradicting
`PredefinedType`, and an object that already has a material
(`invalid-model`); a map whose representation is not an
`IfcShapeRepresentation`, a type with several material associations or a
usage instead of a set, and a layer set usage for a class with no documented
direction when none is given (`unsupported`).

<!-- SNIPPET:cookbook-py-assign-type -->

```python
from openbim_ifc import AuthorOp, Text, handle

# Door 1 (#100) typed again by D-90 (#111) with replace: what the
# old assignment mapped is taken back, the type's representation
# map is mapped afresh, and PredefinedType is left to the type.
model.author([
    AuthorOp.assign_type(
        111, [100], map_representations=True, predefined_type="sync", replace=True
    )
])
# A new door, typed the same way in the batch that creates it.
result = model.author([
    AuthorOp.product("IfcDoor", {"Name": Text("Door 2")}, container=14),
    AuthorOp.assign_type(111, [handle(0)], map_representations=True),
])
body = model[result.ids[0]].Representation.Representations[0]
print(body.RepresentationType)  # 'MappedRepresentation'
# Wall B (#40) moves into the building (#13); without replace the
# second containment is refused with invalid-model.
model.author([AuthorOp.contain(13, [40], replace=True)])
```

<!-- /SNIPPET -->

## Shape representations

`AuthorOp.shape_representation(context, items)` assembles an
`IfcShapeRepresentation` of existing items (built in the same batch with
`create`) in a geometric representation context or sub-context. Its
`representation_type` is checked against the items by the release's
`IfcShapeRepresentationTypes` (`CorrectItemsForType`): a mismatch, items no
listed type admits, a topological item other than a vertex point, edge curve
or face surface, and a missing identifier are `invalid-value`; a label the
release does not list is written as given. Without a type the most specific
one admitting every item is inferred: `SweptSolid` for extrusions and
revolutions, `AdvancedSweptSolid` for other sweeps, `Brep`, `Clipping`,
`CSG`, `Tessellation`, `Curve2D` or `Curve3D` (by the curve's dimension),
`MappedRepresentation`, and so on (IFC2X3 lists fewer types). The
`identifier` defaults to the context's `ContextIdentifier` (`Body` for a
`Body` sub-context); IFC4 and IFC4X3 require one.

A shape representation belongs to exactly one product definition shape,
representation map or shape aspect (`IfcShapeModel.WR11`):

- `assign_representation(representation, to)` adds it to a product's
  `IfcProductDefinitionShape` (created when missing; one several products
  share is refused) or wraps it in a new `IfcRepresentationMap` of a type,
  at the origin. One already used is refused with `invalid-model`.
- `unassign_representation(representation, from_)` takes it off again; an
  emptied product shape, or the type's map with its origin, goes. The
  representation stays for the batch to assign elsewhere or remove.
- `copy_representation(representation)` copies it with every item, profile,
  point, direction and placement it reaches, and the styled items of the
  copied items; the context, styles, colours and the type map a mapped item
  maps are shared. The copy joins the layers of the original.
- `remove_representation(representation)`, and `model.remove_representation(id,
  dry_run=False)` with its `RemovalReport`, take it off what uses it and
  remove it with its items down to what nothing else uses, by the rules
  below; a type map an occurrence still maps is `still-referenced`.

## Removing products and purging

`model.remove_product(id)` removes a product, or a type object, with
everything it solely owns, as one checked transaction, and returns a
`RemovalReport`: `removed`, `kept` (each `KeptEntity` with its `reason`
and `users`), `detached`, `rebased` and `applied`. With `dry_run=True` it
only reports.

The cascade decides "solely owned" by counting references within the
same transaction: an entity reached from a removed one goes when nothing
that stays needs it, a relationship that only loses a list member or a
presentation layer assignment not counting. It removes the product's
`ObjectPlacement`, its representation down to the items no one else uses,
the property and quantity sets only it is defined by with their
properties, its material usage (not the material definitions), its
openings (each with the same cascade), its ports, and every relationship
left without an end. It keeps, and reports with a reason, what other
entities still use (`shared`, with the users: a property set another wall
uses, a door type's representation maps, a style or point used elsewhere),
objects of their own (`related-object`: the type, the container, the
fillings of a removed opening) and definitions it never deletes
(`definition`: material definitions, classification references, units,
contexts, the owner history).

It raises `still-referenced` while other placements that stay are relative
to the product's placement (a window placed relative to the opening of the
wall being removed); `rebase_placements=True` re-bases them onto the
removed placement's parent instead, composing the transforms so nothing
moves. A type object that still types objects raises `still-referenced`
unless `untype_occurrences=True`.

`model.purge()` removes every resource entity no `IfcRoot` reaches, nor
annotates something reached (a styled item, a layer assignment, a shape
aspect); `model.purge(["IfcPropertySet"], include_roots=True)` removes the
unreferenced entities of the named classes instead. `IfcRoot`s and the
owner history with its actors stay unless asked (`include_roots`,
`include_owner_history`); `dry_run=True` reports without removing. The
[cookbook](/cookbook/python#remove-a-product-and-purge) has a recipe.

`model.remove(id)` is deprecated: it leaves references dangling and warns
with a `DeprecationWarning`. Use `remove_with_relationships(id)` for one
entity or `remove_product(id)` for a product.

## API

Generated from the `openbim_ifc` package source. The
[pdoc reference](/api/pdoc/openbim_ifc.html){target="_self"} documents every
class, record and function of the package with its docstring and types.

<!-- API:PYTHON:BEGIN -->

| Member | Description |
| --- | --- |
| `IfcModel()` | An empty model. |
| `IfcModel.parse(data: bytes, *, options: Optional[ParseOptions] = None) -> IfcModel` | Parse a STEP (`.ifc`) file from its bytes. |
| `IfcModel.parse_ifcxml(data: bytes, *, xsd_profile: Optional[str] = None) -> IfcModel` | Parse an ifcXML document. |
| `IfcModel.open(path: Union[str, os.PathLike[str]], *, mapped: bool = False, options: Optional[ParseOptions] = None) -> IfcModel` | Read a STEP (`.ifc`) file, or an ifcZIP archive as :meth:`parse` reads one, from disk. |
| `model.write() -> bytes` | Serialize as STEP bytes. |
| `model.write_ifcxml(*, xsd_profile: Optional[str] = None) -> bytes` | Serialize as ifcXML bytes, in the layout `parse_ifcxml` reads. |
| `model.write_ifczip(*, inner: str = 'step', xsd_profile: Optional[str] = None) -> bytes` | Serialize as ifcZIP bytes: one Deflate entry holding STEP (`model.ifc`), or, with `inner="ifcxml"`, ifcXML (`model.ifcXML`) in the layout `xsd_profile` names, as for :meth:`write_ifcxml`. :meth:`parse` and :meth:`open` read it back. |
| `model.header: Header` | The STEP file header: description, name, time stamp, author, ... |
| `model.set_header(header: Header) -> None` | Replace the STEP file header, e.g. with `dataclasses.replace`. |
| `model.validate(max_findings: Optional[int] = None) -> ValidationReport` | Validate against the schema the header declares. |
| `model.unreachable_products() -> List[UnreachableProduct]` | Products no viewer will draw, with a stable `reason`, in id order. |
| `model.property_sets(id: int) -> List[PropertySet]` | The property sets, quantity sets and predefined property sets of object `id`: its own first, then those its type object holds, an occurrence property overriding an inherited one of the same name. |
| `model.property_sets_many(ids: Optional[Iterable[int]] = None) -> List[ObjectPropertySets]` | :meth:`property_sets` of each of `ids`, in that order, or, with `None`, of every object definition (`IfcObjectDefinition` and its subtypes) in file order, in one pass. |
| `model.resolve_unit(measure_type: str, unit: Optional[int] = None) -> ResolvedUnit` | The effective unit of a `measure_type` value (`"IFCAREAMEASURE"`): `unit` when given (a property's stated unit), otherwise the project default, resolved exactly to SI. |
| `model.set_properties(edits: Iterable[PropertyEdit]) -> PropertyEditResult` | Write and remove property and quantity values as one checked transaction: every edit, in order, or none. |
| `model.set_property(object: int, set: str, name: str, value: Value, *, set_type: Optional[str] = None) -> int` | Write one value (:meth:`set_properties` with one edit); returns the id of the entity holding it. |
| `model.remove_property(object: int, set: str, name: str) -> None` | Remove one property from `object`'s own set (:meth:`set_properties` with one edit). |
| `model.spatial_tree() -> SpatialTree` | The spatial containment tree: every container with its parent, sub-containers and contained elements. |
| `model.classifications(id: int) -> List[Classification]` | The classifications of object `id`: its own, then its type's. |
| `model.material(id: int) -> Optional[MaterialAssignment]` | The material association of object `id`, its own or its type's, or `None`. |
| `model.product_placements(ids: Optional[Iterable[int]] = None) -> List[ProductPlacement]` | Each product's world placement (a column-major 4x4 in metres) and the Body representation a viewer draws, for `ids` or every product with a shape. A product that cannot be placed carries a typed `refusal`; the call raises only `unsupported-schema` or `feature-disabled`. |
| `model.product_geometry(ids: Optional[Iterable[int]] = None, encoding: str = 'json') -> List[ProductGeometry]` | Each product's Body as Axiolid's neutral geometry graph, exact, for `ids` or every product with a shape, in Axiolid's versioned wire format: `{"format": "axiolid-geometry-graph", "version": "1.0", "graph": {"nodes": [...], "roots": [...]}}` (the lowest version the content needs, `"1.0"` to `"1.4"`), in world coordinates, metres. `encoding` `"json"` gives the payload as a `str`, `"cbor"` as `bytes`; any other raises `ValueError`. A product that cannot be lowered carries a typed `refusal`; the call raises only `unsupported-schema` or `feature-disabled`. |
| `model.product_meshes(ids: Optional[Iterable[int]] = None, representation: str = 'Body') -> List[ProductMesh]` | Each product's Body as triangles from the reference backend, for `ids` or every product with a shape; a product that cannot be meshed carries a typed `refusal`. `representation` meshes another one instead (#458): `"Surface"`, or any name :meth:`product_curves` takes; its curves are left to that call, and `closure` says `"solid"` or `"surface"`. Only a Body mesh carries a colour. Needs a wheel built with the `mesh` feature; the published wheel raises `feature-disabled`. |
| `model.product_curves(ids: Optional[Iterable[int]] = None, representation: str = 'Plan') -> List[ProductCurves]` | Each product's curves as polylines from the reference backend (#458), for `ids` or every product with a shape. `representation` is `"Plan"` (the best drawing curves: `Plan`, `Annotation`, `FootPrint`, then `Axis`, preferring a `PLAN_VIEW` context), or `"Axis"`, `"FootPrint"`, `"Annotation"`, `"Body"` or `"Surface"`; another name raises `invalid-value`. A product whose curves cannot be compiled carries a typed `refusal`: a curve the reference compiler has no certified polyline for (an implicit curve, an offset of an offset) is `unsupported`; trimmed, composite, offset and intrinsic curves compile (#506). Text literals and annotation fill areas beside the curves (a dimension's value) are listed in `skipped`, not refused (#512). Needs a wheel built with the `mesh` feature. |
| `model.systems() -> Systems` | Every system with its members and served structures, and the memberships the reader could not honour. |
| `model.cost() -> Cost` | Every cost schedule and cost item; values as authored, typed. |
| `model.georeferencing() -> List[MapConversion]` | Every coordinate operation resolved with the project length unit; empty when the model has none. IFC2X3 is read from its `ePSet_MapConversion` and `ePSet_ProjectedCRS` property sets. |
| `len(model) -> int` | Number of entities. |
| `model.schema: Optional[str]` | The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `None`. |
| `model.diagnostics() -> List[str]` | Non-fatal problems found while reading. |
| `model.ids() -> List[int]` | Every entity id, in file order. |
| `model.ids_of_type(type_name: str) -> List[int]` | Ids of every entity of exactly `type_name` (case-insensitive). |
| `model.ids_of_type_including_subtypes(type_name: str) -> List[int]` | Ids of `type_name` or any subtype, per the file's schema. |
| `model.select_ids(query: str, base: Optional[Iterable[Union[int, Entity]]] = None) -> List[int]` | The ids selector `query` selects, ascending (#464). |
| `model.select(query: str, base: Optional[Iterable[Union[int, Entity]]] = None) -> List[Entity]` | :meth:`select_ids` as :class:`Entity` views. |
| `model.get_value(entity: Union[int, Entity], path: str) -> SelectedValue` | The value `path` reads from `entity` (#464): `"Name"`, `"type.Name"`, `"Pset_WallCommon.FireRating"`, `"materials.Name"`, `"storey.Name"`, `"occurrences.count"`. |
| `model.set_value(entity: Union[int, Entity], path: str, value: Assignable) -> None` | Write `value` at `path` of `entity` (#464), as one checked transaction: an attribute (`"Name"`, `"type.Name"`) coerced as :meth:`set_attribute_by_name_plain` coerces it, or a property (`"Pset_WallCommon.FireRating"`) from a typed value, or a plain one when the property already states its type. A refused write changes nothing; a path naming no single value raises `invalid-value`. |
| `model.export_table(query: str, columns: Sequence[str], *, format: str = 'csv', key: Optional[str] = None, null: str = '') -> bytes` | The entities selector `query` selects as a table (#474): `format` `"csv"`, `"tsv"` or `"xlsx"`, the key column first (`"GlobalId"`, or `"id"` with `key="id"`), then one column per value path of `columns` (`"Name"`, `"type.Name"`, `"Pset_WallCommon.FireRating"`), each cell the value the path reads as text and `NULL` as `null`. |
| `model.import_table(data: Union[bytes, str], *, format: str = 'csv', key: Optional[str] = None, null: str = '', dry_run: bool = False) -> TableImport` | Apply a table (`bytes`, or `str` for CSV and TSV) back to the model by its key column (#474), as one checked transaction: every other cell is written by its header's value path, typed by the attribute's declared type or the property's stated one, and a cell that reads what the model holds changes nothing, so an exported table imported unchanged is a no-op. |
| `model.id_by_guid(global_id: str) -> Optional[int]` | The id of the entity whose `GlobalId` is `global_id`, or `None`. The first lookup indexes every `GlobalId` once; later ones reuse it until the model changes. Raises `invalid-value` for text that is not a `GlobalId` and `invalid-model` when several entities carry it. :meth:`by_guid` returns the :class:`Entity`. |
| `model.duplicate_global_ids() -> List[DuplicateGlobalId]` | Every `GlobalId` several entities carry, with its holders; empty for a conformant file. |
| `model.type_object(id: int) -> Optional[int]` | The type object of occurrence `id` (`IfcRelDefinesByType`), or `None`. Raises `wrong-entity-type` for a non-`IfcObject` and `invalid-model` when the file states two types. |
| `model.occurrences_of(id: int) -> List[int]` | Every object type object `id` types, in file order. |
| `model.openings_of(id: int) -> List[int]` | Every opening voiding element `id` (`IfcRelVoidsElement`). |
| `model.fillings_of(id: int) -> List[int]` | Every element filling opening `id` (`IfcRelFillsElement`). |
| `model.voided_element_of(id: int) -> Optional[int]` | The element opening `id` voids, or `None`. |
| `model.type_of(id: int) -> str` | The upper-case type name of entity `id`. |
| `model.attributes(id: int) -> List[Value]` | Every attribute of entity `id`, in declaration order. |
| `model.attribute(id: int, index: int) -> Value` | Attribute `index` of entity `id`; `Null` past the end. |
| `model.set_attribute(id: int, index: int, value: Value) -> Value` | Set attribute `index` of entity `id`; returns the old value. |
| `model.attribute_names(id: int) -> List[AttributeInfo]` | Every explicit attribute of entity `id` in slot order, inherited first, as the release the header declares defines them. `INVERSE` attributes hold no slot and are listed by :meth:`inverse_attribute_names`. |
| `model.attribute_by_name(id: int, name: str) -> Value` | Attribute `name` of entity `id`, matched case-insensitively (`"Name"`) and resolved against the declared release; `Null` when the record stops before its slot. |
| `model.inverse_attribute_names(id: int) -> List[InverseAttributeInfo]` | Every `INVERSE` attribute of entity `id`, inherited ones first, with its `FOR` clause and bounds, as the declared release defines them. |
| `model.inverse_references(id: int) -> List[InverseReference]` | Every reference to entity `id`: the referencing entity and the attribute index holding the reference, in ascending order, one per entity and attribute. IfcOpenShell's `file.get_inverse(entity, allow_duplicate=True, with_attribute_indices=True)`. |
| `model.total_inverses(id: int) -> int` | How many distinct entities reference entity `id`, through any attribute. IfcOpenShell's `file.get_total_inverses`. |
| `model.set_attribute_by_name(id: int, name: str, value: Value) -> Value` | Set attribute `name` of entity `id`; returns the old value. A derived attribute raises `derived-attribute`, an `INVERSE` one `inverse-attribute`, an unknown name `unknown-attribute`, and a refused write changes nothing. |
| `model.set_attribute_by_name_plain(id: int, name: str, value: Assignable) -> Value` | Set attribute `name` of entity `id` from a plain Python value, coerced against the attribute's declared type in the declared release; returns the old tagged value. `wall.Name = "x"` calls this. |
| `model.add(type_name: str, attributes: Iterable[Value]) -> int` | Append an entity; returns its new id. |
| `model.remove(id: int) -> None` | Remove entity `id`, leaving references to it dangling. |
| `model.author(ops: Iterable[AuthorOp]) -> AuthoringResult` | Apply authoring operations as one checked transaction against the release the header declares: every operation, in order, or none. |
| `model.create_entity(type_name: str, attributes: Optional[Mapping[str, Value]] = None) -> int` | Create one entity of `type_name` from named attributes, checked against the declared release (:meth:`author` with one :meth:`AuthorOp.create`); returns its id. |
| `model.remove_with_relationships(id: int) -> None` | Remove entity `id` with the relationships that reference it, leaving nothing dangling (:meth:`author` with one :meth:`AuthorOp.remove`). Raises `still-referenced` while an entity other than a relationship needs it. |
| `model.remove_product(id: int, *, rebase_placements: bool = False, untype_occurrences: bool = False, dry_run: bool = False) -> RemovalReport` | Remove product (or type object) `id` with everything it solely owns -- its placement, representation, own property and quantity sets, material usage, openings and ports, and the relationships left without an end -- as one checked transaction. Shared data stays: a property set another object uses, a type's representation maps, a style used elsewhere. The report says what went and what stayed, and why; with `dry_run` nothing changes. |
| `model.purge(classes: Sequence[str] = (), *, include_roots: bool = False, include_owner_history: bool = False, dry_run: bool = False) -> RemovalReport` | Remove unreferenced entities as one checked transaction; with `dry_run`, only report. |
| `model.remove_representation(id: int, *, dry_run: bool = False) -> RemovalReport` | Remove shape representation `id` with what only it uses, as one checked transaction: it is taken off its product definition shape (which goes when left empty), representation map or shape aspects, then removed with its items down to what nothing else uses. Shared items, profiles, points and styles stay, and the report says why; with `dry_run` nothing changes. |
| `model.dangling_references() -> List[Tuple[int, int]]` | Every `(from, to)` pair where `to` does not exist. |
| `model.by_id(id: int) -> Entity` | Entity `#id` as an :class:`Entity`; raises :class:`openbim_ifc.IfcError` (`missing-entity`) when absent. |
| `model.by_guid(global_id: str) -> Optional[Entity]` | The entity whose `GlobalId` is `global_id` as an :class:`Entity`, or `None`: :meth:`IfcModel.id_by_guid`. |
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
| `entity.set(name: str, value: Assignable) -> Value` | Write attribute `name` (`entity.Name = value`); returns the old tagged value. A plain value is coerced against the declared type (:meth:`openbim_ifc.IfcModel.set_attribute_by_name_plain`); a tagged one is written as given. A refused write changes nothing. |
| `entity.attribute_names() -> list[AttributeInfo]` | Every explicit attribute, in slot order, as :meth:`IfcModel.attribute_names` lists them. |
| `entity.inverse_attribute_names() -> list[InverseAttributeInfo]` | Every `INVERSE` attribute, as :meth:`IfcModel.inverse_attribute_names` lists them. |
| `entity.inverse_references() -> list[Tuple[Entity, int]]` | Every entity referencing this one, with the attribute index holding the reference: :meth:`IfcModel.inverse_references` as `(Entity, index)` pairs. |
| `entity.total_inverses() -> int` | How many distinct entities reference this one: :meth:`IfcModel.total_inverses`. |
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
