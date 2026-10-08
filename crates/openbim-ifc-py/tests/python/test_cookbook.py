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
