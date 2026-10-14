# Python cookbook

Task-sized recipes for [`openbim-ifc`](/bindings/python), the Python
binding of the IFC core. Each recipe is a region of
`crates/openbim-ifc-py/tests/python/test_cookbook.py`, which runs on every
pull request against the built wheel and checks what the recipe prints, so
the code below is the code that ran. The comments show its output for the
fixtures it reads: `test/fixtures/synthetic-properties/synthetic_properties.ifc`
(two walls on one storey, with their property sets) and
`test/fixtures/synthetic-bindings/binding_geometry.ifc` (placed, extruded
products).

For the full surface, see the [binding page](/bindings/python) and the
[Python API reference](/api/pdoc/openbim_ifc.html){target="_self"}.

## Set up

```bash
pip install openbim-ifc
```

## Open a file

<!-- SNIPPET:cookbook-py-open -->

```python
import openbim_ifc

model = openbim_ifc.open(path)  # read once, straight from disk
print((model.schema, len(model)))  # ('IFC4', 68)
header = model.header  # the STEP header, a frozen dataclass
print((header.originating_system, header.time_stamp))

with open(path, "rb") as file:  # or from bytes you already hold
    same = openbim_ifc.IfcModel.parse(file.read())
```

<!-- /SNIPPET -->

A file that does not parse raises `openbim_ifc.IfcError` with `code`
`parse`; `IfcModel.parse(data, options=ParseOptions(on_malformed="skip"))`
reads a damaged export and lists what it skipped in `model.diagnostics()`.

## Read property sets

One object's property sets, as plain values through the entity view, and
as the exact records the binding returns:

<!-- SNIPPET:cookbook-py-property-sets -->

```python
wall = next(w for w in model.by_type("IfcWall") if w.Name == "Wall A")

# Plain values: the wall's own, then those inherited from its type.
for set_name, properties in wall.psets.items():
    for name, value in properties.items():
        print(f"{set_name}.{name} = {value!r}")
# Pset_WallCommon.IsExternal = False
# Pset_WallCommon.FireRating = 'F30'   (inherited from the wall type)
width = wall.qtos["Qto_WallBaseQuantities"]["Width"]  # 200.0, in its stated unit

# The exact records: declared types, units, and where each set came from.
for pset in model.property_sets(wall.id):
    print((pset.name, pset.source, [(p.name, p.value) for p in pset.properties]))
```

<!-- /SNIPPET -->

Many objects in one call, here every wall:

<!-- SNIPPET:cookbook-py-property-sets-many -->

```python
# One pass for every wall: the property index is built once, so this
# stays linear in the model where a loop of property_sets is quadratic.
rows = {}  # id -> {"Set.Property": value}
for answer in model.property_sets_many(model.ids_of_type("IfcWall")):
    if answer.refusal is not None:
        print(answer.object, answer.refusal.code, answer.refusal.message)
        continue
    row = rows.setdefault(answer.object, {})
    for pset in answer.sets:
        for prop in pset.properties:
            row.setdefault(f"{pset.name}.{prop.name}", prop.value)

# Or as a pandas frame, one row per wall (pip install 'openbim-ifc[pandas]'):
# frame = model.to_dataframe("IfcWall")
```

<!-- /SNIPPET -->

`property_sets_many()` with no ids answers for every object definition in
the file.

## List storeys and their elements

<!-- SNIPPET:cookbook-py-storeys -->

```python
tree = model.spatial_tree()
for storey in (node for node in tree.nodes if node.kind == "storey"):
    print(f"{storey.name} (#{storey.id})")
    for element in map(model.by_id, storey.elements):
        print(f"  {element.type} #{element.id} {element.Name}")
# Level 0 (#25)
#   IFCWALL #30 Wall A
#   IFCWALL #31 Wall B
```

<!-- /SNIPPET -->

Each node also has its `parent` and `children`, so the same tree gives the
project, site and building above the storeys and the spaces below them;
`tree.orphans` lists elements no container holds.

## Validate

<!-- SNIPPET:cookbook-py-validate -->

```python
report = model.validate()  # against the schema the header declares
if not report.conformant:
    print(f"{report.errors} error(s), {report.warnings} warning(s)")
for finding in report.findings:
    # severity: "error", "evaluation-error", "warning" or "unsupported"
    at = "file" if finding.entity is None else f"#{finding.entity}"
    print(f"{finding.severity} {finding.rule} {at} {finding.attribute_name or ''}: {finding.message}")
```

<!-- /SNIPPET -->

The recipe ran on the fixture with Wall A's `PredefinedType` set to an
item `IfcWallTypeEnum` does not have. `validate(max_findings)` caps the
report; `report.truncated` says when the cap was reached.

## Convert to and from ifcXML

<!-- SNIPPET:cookbook-py-ifcxml -->

```python
xml = model.write_ifcxml()  # this library's lossless layout, UTF-8 bytes
back = IfcModel.parse_ifcxml(xml)
step = back.write()  # the same entities, as STEP again

# The buildingSMART XSD layout of a release, for tools that read it.
xsd = model.write_ifcxml(xsd_profile="IFC4")
from_xsd = IfcModel.parse_ifcxml(xsd, xsd_profile="IFC4")
```

<!-- /SNIPPET -->

The lossless layout carries everything the STEP file does. An XSD layout
refuses, with `code` `write`, what it cannot carry exactly.

## Edit a property set and attributes

<!-- SNIPPET:cookbook-py-edit -->

```python
from openbim_ifc import PropertyEdit, Text, Typed

wall = next(w for w in model.by_type("IfcWall") if w.Name == "Wall B")

# Property values are exact IFC values; Pset_/Qto_ sets are checked
# against the release's PSD/QTO catalog, which the wheel embeds.
# Wall B inherits FireRating from its type: this overrides it on the wall.
wall.set_property("Pset_WallCommon", "FireRating", Typed("IFCLABEL", Text("F90")))
# Several edits as one checked transaction: all of them, or none.
model.set_properties([
    PropertyEdit(wall.id, "Checks", "Reviewer", Typed("IFCLABEL", Text("QA"))),
    PropertyEdit.removal(wall.id, "Pset_Families", "Colour"),
])

# Attributes take plain values, coerced against their declared type.
wall.Name = "Wall B (checked)"  # IfcLabel
wall.PredefinedType = "solidwall"  # IfcWallTypeEnum: .SOLIDWALL.

with open(out, "wb") as file:
    file.write(model.write())
```

<!-- /SNIPPET -->

Property values are exact values (`Typed`, `Text`, `Real`, ...), typed as
`property_sets` reports them; attributes take plain Python values coerced
against the attribute's declared type, or an exact value. A refused edit
raises `IfcError` and changes nothing: `invalid-value`,
`template-violation` (the PSD/QTO catalog disagrees), `missing-property`
(a removal of a property the object does not state), `type-mismatch` (a
plain value that does not fit).

## Read placements, meshes and exact geometry

Placements are in every wheel:

<!-- SNIPPET:cookbook-py-placements -->

```python
for product in model.product_placements():
    if product.refusal is not None:  # per product, never raised
        print((product.id, product.refusal.code))
        continue
    # A column-major 4x4 in metres; the translation is its last column.
    x, y, z = product.transform[12:15]
    body = product.representation  # None for an axis-only product
    print((product.type_name, (x, y, z), body and body.representation_type))
# ('IFCWALL', (512002.0, 5403001.0, 3.0), 'SweptSolid') ...
```

<!-- /SNIPPET -->

Meshes link a geometry kernel, which the published wheel leaves out (it
raises `feature-disabled`); build one with
`maturin build --release --features mesh` in `crates/openbim-ifc-py`:

<!-- SNIPPET:cookbook-py-meshes -->

```python
# A wheel built with `maturin build --release --features mesh`.
world = {}
for mesh in model.product_meshes():
    if mesh.refusal is not None or not mesh.indices:
        continue  # refused, or no Body
    m = mesh.transform  # column-major, metres; positions are relative to it
    p = mesh.positions  # array('f'): x y z per vertex
    world[mesh.id] = [
        tuple(m[a] * p[i] + m[4 + a] * p[i + 1] + m[8 + a] * p[i + 2] + m[12 + a] for a in range(3))
        for i in range(0, len(p), 3)
    ]
    # mesh.indices: array('I'), three per triangle
```

<!-- /SNIPPET -->

Positions are `f32` relative to each product's `f64` transform, so a site
kilometres from the origin keeps its millimetres;
`numpy.frombuffer(mesh.positions, numpy.float32)` reads them without a copy.

The exact representation instead of triangles
([#367](https://github.com/openbimrs/ifc/issues/367)): each Body as
Axiolid's neutral geometry graph in its wire format, for your own
kernel. It needs a wheel built with the `graph` feature:

<!-- SNIPPET:cookbook-py-graphs -->

```python
# A wheel built with `maturin build --release --features graph`.
import json

kinds = {}
for product in model.product_geometry():  # or encoding="cbor": bytes
    if product.refusal is not None or product.payload is None:
        continue  # refused, or no Body
    envelope = json.loads(product.payload)  # Axiolid's wire format
    # Each node is tagged by its kind; a reference is an earlier node's index.
    kinds[product.id] = [next(iter(node)) for node in envelope["graph"]["nodes"]]
# {36: ['Profile', 'SolidOperation', 'Instance'], 46: [...]}
```

<!-- /SNIPPET -->

Each `payload` is the envelope `{"format": "axiolid-geometry-graph",
"version": "1.0", "graph": {...}}`, exact, in world coordinates, metres;
`encoding="cbor"` gives it as `bytes`. The
[binding page](/bindings/python#geometry) describes it.

## Find entities by GlobalId, types and openings

An entity by its `GlobalId`, an occurrence's type object and the type's
other occurrences, and a wall's openings with what fills them, read from
`test/fixtures/synthetic-relations/relations_ifc4.ifc`:

<!-- SNIPPET:cookbook-py-global-ids -->

```python
wall = model.by_guid("0LwOtle8vEaxZPGHkxQdYT")  # an Entity, or None
# The first lookup indexes every GlobalId once; the rest reuse it.

wall_type = model.type_object(wall.id)  # IfcRelDefinesByType
print((model[wall_type].Name, model.occurrences_of(wall_type)))  # ('WT-200', [17, 18])
for opening in model.openings_of(wall.id):  # IfcRelVoidsElement
    fillings = [model[id].Name for id in model.fillings_of(opening)]
    print((model[opening].Name, fillings))
# ('Door opening', ['Door 1'])
# ('Recess', [])

print(model.duplicate_global_ids())  # [] -- a copied GlobalId is listed here
print(openbim_ifc.new_guid())  # random, e.g. '2Vq9nF0yL5yQ$1x_4kZb3T'
print(openbim_ifc.name_based_guid("my-exporter", "wall", "42"))  # same every run
```

<!-- /SNIPPET -->

A `GlobalId` that several entities carry raises `IfcError` with `code`
`invalid-model` naming every holder; `model.duplicate_global_ids()` lists
them all. `new_guid()` draws an RFC 9562 version-4 UUID from the operating
system; `name_based_guid(*parts)` is version 8, the same for the same parts
in every binding. Both are IFC's 22-character form.

## Select entities and read values by path

A [selector](/guide/selectors) picks entities by class, attribute,
property, type, material, classification, location and group; a value path
reads (and writes) one value of an entity. Read from
`test/fixtures/synthetic-relations/selector_ifc4.ifc`:

<!-- SNIPPET:cookbook-py-selectors -->

```python
walls = model.select("IfcWall, Pset_WallCommon.FireRating=F90")  # Entity views
print([wall.Name for wall in walls])  # ['Wall A', 'Wall B']
print(model.select_ids('IfcDoor + location="Level 1"'))  # [19, 28, 38, 40]

print(model.get_value(walls[0], "type.Name"))  # 'WT-200'
print(model.get_value(walls[0], "materials.Name"))  # ('Concrete',)
print(dict(model.get_value(walls[0], "Pset_WallCommon")))
# {'IsExternal': True, 'FireRating': 'F90', 'AcousticRating': 'R45'}

model.set_value(walls[0], "type.Name", "WT-300")  # one checked transaction
try:
    model.select("IfcWall, colour=red")
except openbim_ifc.IfcError as error:
    print((error.code, error.span))  # ('invalid-selector', (9, 15))
```

<!-- /SNIPPET -->

`select` returns `Entity` views and `select_ids` the ids, in ascending id
order; `base=` restricts both to given entities. A property set reads as a
read-only mapping, several values as a tuple, a reference as an `Entity`.
`set_value` writes an attribute coerced as `wall.Name = ...` is, or a
property in the type the property already states. A selector that does not
parse, or names a class or attribute the release does not declare, raises
`IfcError` with `code` `invalid-selector` and `span`, the byte range it
refuses.

## Remove a product and purge

A door with what it alone owns, a wall whose openings place their fillings,
then what nothing uses any more, from
`test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

<!-- SNIPPET:cookbook-py-remove-product -->

```python
# Door #100 with what it alone owns: placement, representation, its
# filling and type relationships. A dry run changes nothing.
report = model.remove_product(100, dry_run=True)
print(len(report.removed))  # 11
for kept in report.kept:
    if kept.reason == "shared":  # still used by something that stays
        print((kept.id, kept.type_name, kept.users))
# (82, 'IFCLOCALPLACEMENT', (81,))     the opening's placement
# (112, 'IFCREPRESENTATIONMAP', (111,))  the door type's geometry

model.remove_product(100)  # applied: one checked transaction
# A wall whose openings place their fillings: re-base those
# placements onto the storey so nothing moves, or it is refused.
model.remove_product(20, rebase_placements=True)

# Then purge what nothing uses any more: no classes means every
# resource entity no IfcRoot reaches.
print([entity.type_name for entity in model.purge(dry_run=True).removed][:3])
```

<!-- /SNIPPET -->

`rebase_placements=True` re-bases placements relative to a removed one onto
its parent, composing the transforms; `untype_occurrences=True` lets a type
object with occurrences go. Without them those removals raise
`still-referenced`. `purge(["IfcPropertySet"], include_roots=True)` purges
unreferenced entities of the named classes only. The
[binding page](/bindings/python#removing-products-and-purging) lists what
the cascade removes and keeps.

## Assign a type and propagate it

Door 1 typed again by its door type, with the type's representation map
mapped onto it, and Wall B moved into the building, from
`test/fixtures/synthetic-relations/cascade_ifc4.ifc`:

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

The type assignment takes `map_representations`, `mapping_target`, `material_usage`, `layer_set_direction`, `predefined_type` and `replace`. Every option is off by default: the
operation then writes only the `IfcRelDefinesByType` and refuses an object
that already has a type (`invalid-model`). With `replace=True` the previous
type is unassigned and the representations and material usage it
propagated are removed with what only they use; `AuthorOp.contain` and `AuthorOp.aggregate` take the
same flag. `material_usage` derives an `IfcMaterialLayerSetUsage` (walls
`AXIS2`, slabs and plates `AXIS3`, `POSITIVE`, offset 0) or an
`IfcMaterialProfileSetUsage` from the type's set; `predefined_type="validate"` refuses an
object whose own `PredefinedType` contradicts the type's. The
[binding page](/bindings/python#assigning-types) lists the rules.

## Create a model

<!-- SNIPPET:cookbook-py-create -->

```python
from openbim_ifc import AuthorOp, Header, Text, handle

model = IfcModel()
model.set_header(Header(
    description=("ViewDefinition [DesignTransferView]",),
    implementation_level="2;1",
    name="new.ifc",
    time_stamp=datetime.datetime.now().strftime("%Y-%m-%dT%H:%M:%S"),
    author=("",),
    organization=("",),
    preprocessor_version="openbim-ifc",
    originating_system="cookbook",
    authorization="",
    schema=("IFC4",),  # the release every operation is checked against
))
result = model.author([  # handle(i): the entity operation i of this batch creates
    AuthorOp.project({"Name": Text("Demo")}),  # 0
    AuthorOp.placement(),  # 1
    AuthorOp.spatial("IfcSite", handle(0), placement=handle(1)),  # 2
    AuthorOp.spatial("IfcBuilding", handle(2)),  # 3
    AuthorOp.spatial("IfcBuildingStorey", handle(3), {"Name": Text("Level 0")}),  # 4
    AuthorOp.placement(relative_to=handle(1), location=(4.0, 0.0, 0.0)),  # 5
    AuthorOp.product("IfcWall", {"Name": Text("Wall")}, container=handle(4), placement=handle(5)),  # 6
])
wall = result.ids[6]  # every IfcRoot got a GlobalId
data = model.write()
```

<!-- /SNIPPET -->

Every operation is checked against the release the header declares, and a
batch is one transaction. The [binding page](/bindings/python#creating-entities)
lists every operation.
