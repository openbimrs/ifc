"""The Python cookbook (docs/cookbook/python.md, #331): every recipe on that
page is a ``docs:snippet`` region below, run against the installed wheel
and checked against the fixture it reads. Run by scripts/check-python.sh,
which runs this file again against the mesh wheel with OPENBIM_IFC_MESH=1
for the mesh recipe."""

import dataclasses
import datetime
import os
import tempfile
import unittest
from typing import Any, List

import openbim_ifc
from openbim_ifc import Bool, IfcModel, Typed

FIXTURES = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures")
PROPERTIES = os.path.join(FIXTURES, "synthetic-properties", "synthetic_properties.ifc")
GEOMETRY = os.path.join(FIXTURES, "synthetic-bindings", "binding_geometry.ifc")
RELATIONS = os.path.join(FIXTURES, "synthetic-relations", "relations_ifc4.ifc")
SELECTOR = os.path.join(FIXTURES, "synthetic-relations", "selector_ifc4.ifc")
CASCADE = os.path.join(FIXTURES, "synthetic-relations", "cascade_ifc4.ifc")
MESH = os.environ.get("OPENBIM_IFC_MESH") == "1"


class Cookbook(unittest.TestCase):
    def test_open(self) -> None:
        path = PROPERTIES
        printed: List[Any] = []
        print = printed.append  # noqa: A001 - keeps the snippet's output in the test
        # docs:snippet cookbook-py-open
        import openbim_ifc

        model = openbim_ifc.open(path)  # read once, straight from disk
        print((model.schema, len(model)))  # ('IFC4', 68)
        header = model.header  # the STEP header, a frozen dataclass
        print((header.originating_system, header.time_stamp))

        with open(path, "rb") as file:  # or from bytes you already hold
            same = openbim_ifc.IfcModel.parse(file.read())
        # docs:end
        self.assertEqual(printed[0], ("IFC4", 68))
        self.assertEqual(len(same), len(model))

    def test_property_sets_of_one_object(self) -> None:
        model = openbim_ifc.open(PROPERTIES)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-property-sets
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
        # docs:end
        self.assertIn("Pset_WallCommon.IsExternal = False", printed)
        self.assertIn("Pset_WallCommon.FireRating = 'F30'", printed)
        self.assertEqual(width, 200.0)
        self.assertEqual(printed[-1][:2], ("Pset_WallCommon", "type"))

    def test_property_sets_of_many_objects(self) -> None:
        model = openbim_ifc.open(PROPERTIES)
        # docs:snippet cookbook-py-property-sets-many
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
        # docs:end
        self.assertEqual(sorted(rows), [30, 31])
        self.assertEqual(rows[30]["Pset_WallCommon.IsExternal"], Typed("IFCBOOLEAN", Bool(False)))
        self.assertEqual(rows[31]["Pset_WallCommon.IsExternal"], Typed("IFCBOOLEAN", Bool(True)))

    def test_storeys_and_their_elements(self) -> None:
        model = openbim_ifc.open(PROPERTIES)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-storeys
        tree = model.spatial_tree()
        for storey in (node for node in tree.nodes if node.kind == "storey"):
            print(f"{storey.name} (#{storey.id})")
            for element in map(model.by_id, storey.elements):
                print(f"  {element.type} #{element.id} {element.Name}")
        # Level 0 (#25)
        #   IFCWALL #30 Wall A
        #   IFCWALL #31 Wall B
        # docs:end
        self.assertEqual(printed, ["Level 0 (#25)", "  IFCWALL #30 Wall A", "  IFCWALL #31 Wall B"])

    def test_validate(self) -> None:
        with open(PROPERTIES, "rb") as handle:
            data = handle.read().replace(b"'Wall A',$,$,$,$,$,$)", b"'Wall A',$,$,$,$,$,.NOTANENUM.)")
        model = IfcModel.parse(data)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-validate
        report = model.validate()  # against the schema the header declares
        if not report.conformant:
            print(f"{report.errors} error(s), {report.warnings} warning(s)")
        for finding in report.findings:
            # severity: "error", "evaluation-error", "warning" or "unsupported"
            at = "file" if finding.entity is None else f"#{finding.entity}"
            print(f"{finding.severity} {finding.rule} {at} {finding.attribute_name or ''}: {finding.message}")
        # docs:end
        self.assertFalse(report.conformant)
        self.assertTrue(any(line.startswith("error ") and "#30" in line for line in printed), printed)
        self.assertEqual(openbim_ifc.open(PROPERTIES).validate().errors, 0)

    def test_ifcxml(self) -> None:
        model = openbim_ifc.open(GEOMETRY)
        # docs:snippet cookbook-py-ifcxml
        xml = model.write_ifcxml()  # this library's lossless layout, UTF-8 bytes
        back = IfcModel.parse_ifcxml(xml)
        step = back.write()  # the same entities, as STEP again

        # The buildingSMART XSD layout of a release, for tools that read it.
        xsd = model.write_ifcxml(xsd_profile="IFC4")
        from_xsd = IfcModel.parse_ifcxml(xsd, xsd_profile="IFC4")
        # docs:end
        self.assertEqual(back.ids(), model.ids())
        self.assertEqual(len(IfcModel.parse(step)), len(model))
        self.assertTrue(xml.startswith(b"<?xml"))
        self.assertEqual(len(from_xsd.ids_of_type("IfcWall")), 1)

    def test_edit(self) -> None:
        model = openbim_ifc.open(PROPERTIES)
        with tempfile.TemporaryDirectory() as directory:
            out = os.path.join(directory, "checked.ifc")
            # docs:snippet cookbook-py-edit
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
            # docs:end
            again = openbim_ifc.open(out)
        checked = again[31]
        self.assertEqual(checked.Name, "Wall B (checked)")
        self.assertEqual(checked.PredefinedType, "SOLIDWALL")
        self.assertEqual(checked.psets["Checks"]["Reviewer"], "QA")
        self.assertEqual(checked.psets["Pset_WallCommon"]["FireRating"], "F90")
        self.assertNotIn("Colour", checked.psets["Pset_Families"])
        self.assertEqual(again[29].psets["Pset_WallCommon"]["FireRating"], "F30", "the type is unchanged")

    def test_placements(self) -> None:
        model = openbim_ifc.open(GEOMETRY)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-placements
        for product in model.product_placements():
            if product.refusal is not None:  # per product, never raised
                print((product.id, product.refusal.code))
                continue
            # A column-major 4x4 in metres; the translation is its last column.
            x, y, z = product.transform[12:15]
            body = product.representation  # None for an axis-only product
            print((product.type_name, (x, y, z), body and body.representation_type))
        # ('IFCWALL', (512002.0, 5403001.0, 3.0), 'SweptSolid') ...
        # docs:end
        self.assertEqual(printed[0], ("IFCWALL", (512002.0, 5403001.0, 3.0), "SweptSolid"))
        self.assertEqual(len(printed), 4)

    def test_meshes(self) -> None:
        model = openbim_ifc.open(GEOMETRY)
        if not MESH:
            from openbim_ifc import IfcError

            with self.assertRaises(IfcError) as caught:
                model.product_meshes()
            self.assertEqual(caught.exception.code, "feature-disabled")
            return
        # docs:snippet cookbook-py-meshes
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
        # docs:end
        self.assertEqual(sorted(world), [36, 46])
        for x, y, _ in world[36]:
            self.assertLess(abs(x - 512002), 3)
            self.assertLess(abs(y - 5403001), 3)

    def test_graphs(self) -> None:
        model = openbim_ifc.open(GEOMETRY)
        if not MESH:
            from openbim_ifc import IfcError

            with self.assertRaises(IfcError) as caught:
                model.product_geometry()
            self.assertEqual(caught.exception.code, "feature-disabled")
            return
        # docs:snippet cookbook-py-graphs
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
        # docs:end
        self.assertEqual(sorted(kinds), [36, 46])
        self.assertEqual(kinds[36], ["Profile", "SolidOperation", "Instance"])

    def test_global_ids_types_and_openings(self) -> None:
        model = openbim_ifc.open(RELATIONS)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-global-ids
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
        # docs:end
        self.assertIsNotNone(wall)
        self.assertEqual(printed[0], ("WT-200", [17, 18]))
        self.assertEqual(printed[1:3], [("Door opening", ["Door 1"]), ("Recess", [])])
        self.assertEqual(printed[3], [])
        self.assertEqual(len(printed[4]), 22)
        self.assertEqual(printed[5], openbim_ifc.name_based_guid("my-exporter", "wall", "42"))
        self.assertIsNone(model.by_guid("3MtNd0Uaz8Ix2OPtRq_3Kk"))

    def test_convert_units(self) -> None:
        model = openbim_ifc.open(GEOMETRY)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-convert-units
        # Between units named as text: exact factors, checked dimensions.
        print(openbim_ifc.convert_unit(1.0, "foot", "MILLIMETRE"))  # 304.8
        print(openbim_ifc.format_length(444.5, "MILLIMETRE", style="feet-inches"))
        # 1' - 5 1/2"

        # The whole model to metres: geometry, quantities and properties,
        # as one checked transaction. A dry run only reports.
        report = model.convert_length_unit("METRE", dry_run=True)
        print((report.from_unit, report.to, report.length_factor))
        # ('MILLI METRE', 'METRE', 0.001)
        report = model.convert_length_unit("METRE")
        print(report.applied)  # True
        for kept in report.kept:  # values left as they are, and why
            print((kept.entity_type, kept.attribute, kept.reason))
        # docs:end
        self.assertEqual(printed[0], 304.8)
        self.assertEqual(printed[1], "1' - 5 1/2\"")
        self.assertEqual(printed[2], ("MILLI METRE", "METRE", 0.001))
        self.assertTrue(printed[3])

    def test_remove_product_and_purge(self) -> None:
        model = openbim_ifc.open(CASCADE)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-remove-product
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
        # docs:end
        self.assertEqual(printed[0], 11)
        self.assertEqual(
            printed[1:3],
            [(82, "IFCLOCALPLACEMENT", (81,)), (112, "IFCREPRESENTATIONMAP", (111,))],
        )
        self.assertEqual(model.dangling_references(), [])
        self.assertEqual(len(printed[3]), 3)

    def test_select_and_values_by_path(self) -> None:
        model = openbim_ifc.open(SELECTOR)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-selectors
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
        # docs:end
        self.assertEqual(printed[0], ["Wall A", "Wall B"])
        self.assertEqual(printed[1], [19, 28, 38, 40])
        self.assertEqual(printed[2:4], ["WT-200", ("Concrete",)])
        self.assertEqual(
            printed[4], {"IsExternal": True, "FireRating": "F90", "AcousticRating": "R45"}
        )
        self.assertEqual(printed[5], ("invalid-selector", (9, 15)))
        self.assertEqual(model.get_value(18, "type.Name"), "WT-300")

    def test_tables(self) -> None:
        model = openbim_ifc.open(SELECTOR)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-tables
        columns = ["Name", "type.Name", "Pset_WallCommon.FireRating"]
        csv = model.export_table("IfcWall", columns, key="id")  # bytes; or format="xlsx"
        print(csv.decode().splitlines()[1])  # '17,Wall A,WT-200,F90'

        # Edit the table (here, as text), then plan the import: a dry run.
        edited = csv.decode().replace("17,Wall A,WT-200,F90", "17,Wall A,WT-200,F120")
        plan = model.import_table(edited, dry_run=True)
        for change in plan.changes:
            print((change.row, change.path, change.before, change.after))
        # (2, 'Pset_WallCommon.FireRating', 'F90', 'F120')

        result = model.import_table(edited)  # one checked transaction
        print(result.applied)  # True
        # A refused cell refuses the whole table, by row and column.
        refused = model.import_table("id,Qto_WallBaseQuantities.Length\n17,long\n")
        print([(r.row, r.column, r.code) for r in refused.refusals])  # [(2, 2, 'type-mismatch')]
        # docs:end
        self.assertEqual(printed[0], "17,Wall A,WT-200,F90")
        self.assertEqual(printed[1], (2, "Pset_WallCommon.FireRating", "F90", "F120"))
        self.assertEqual(printed[2], True)
        self.assertEqual(printed[3], [(2, 2, "type-mismatch")])
        self.assertEqual(model.get_value(17, "Pset_WallCommon.FireRating"), "F120")

    def test_assign_type_with_propagation(self) -> None:
        model = openbim_ifc.open(CASCADE)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-assign-type
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
        # docs:end
        self.assertEqual(printed, ["MappedRepresentation"])
        self.assertIsNone(model[100].PredefinedType)
        self.assertEqual(model.dangling_references(), [])
        report = model.validate()
        self.assertEqual(report.errors, 0, report.findings)

    def test_representations(self) -> None:
        model = openbim_ifc.open(CASCADE)
        printed: List[Any] = []
        print = printed.append  # noqa: A001
        # docs:snippet cookbook-py-representation
        from openbim_ifc import AuthorOp, Enum, Real, Ref, handle
        from openbim_ifc import List as Aggregate  # an IFC aggregate value

        h = handle  # h(i): the entity operation i of the batch produces
        result = model.author(
            [
                # The items first: a profile swept 1 m up, through the generic create.
                AuthorOp.create("IfcCartesianPoint", {"Coordinates": Aggregate((Real(0.6), Real(0.05)))}),
                AuthorOp.create("IfcAxis2Placement2D", {"Location": Ref(h(0))}),
                AuthorOp.create(
                    "IfcRectangleProfileDef",
                    {"ProfileType": Enum("AREA"), "Position": Ref(h(1)), "XDim": Real(1.2), "YDim": Real(0.1)},
                ),
                AuthorOp.create(
                    "IfcExtrudedAreaSolid",
                    {"SweptArea": Ref(h(2)), "Position": Ref(9), "ExtrudedDirection": Ref(31), "Depth": Real(1.0)},
                ),
                # Context #11 is the 'Body' sub-context: the identifier is
                # 'Body', and the type is inferred from the item.
                AuthorOp.shape_representation(11, [h(3)]),  # 4
                AuthorOp.assign_representation(h(4), 130),  # 5: window #130's shape
                # Wall #20's body swapped for a copy of wall #40's.
                AuthorOp.copy_representation(45),  # 6
                AuthorOp.unassign_representation(25, 20),
                AuthorOp.remove_representation(25),
                AuthorOp.assign_representation(h(6), 20),
            ]
        )
        print(model.attribute_by_name(result.ids[4], "RepresentationType"))  # Text('SweptSolid')
        # One alone, with a report of what went and what stayed:
        report = model.remove_representation(105, dry_run=True)  # the door's body
        print([entity.id for entity in report.removed])  # [104, 105, 106, 107, 108]
        # docs:end
        from openbim_ifc import Text

        self.assertEqual(printed[0], Text("SweptSolid"))
        self.assertEqual(printed[1], [104, 105, 106, 107, 108])
        self.assertEqual(model.dangling_references(), [])

    def test_create(self) -> None:
        # docs:snippet cookbook-py-create
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
        # docs:end
        again = IfcModel.parse(data)
        self.assertEqual(again.type_of(wall), "IFCWALL")
        storey = next(node for node in again.spatial_tree().nodes if node.kind == "storey")
        self.assertEqual((storey.name, storey.elements), ("Level 0", (wall,)))
        self.assertEqual(again.validate().errors, 0)
        self.assertTrue(dataclasses.is_dataclass(model.header))


if __name__ == "__main__":
    unittest.main()
