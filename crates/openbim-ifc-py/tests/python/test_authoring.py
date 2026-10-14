"""Schema-checked entity creation from Python (#330): a model built from
nothing that validates clean and round-trips through STEP and ifcXML, a
refused batch that leaves the model byte-identical, and the shared codes.
Run by scripts/check-python.sh."""

import dataclasses
import unittest

from openbim_ifc import (
    AuthorOp,
    AuthoringResult,
    Enum,
    IfcError,
    IfcModel,
    Integer,
    List,
    Null,
    Real,
    Ref,
    Text,
    Typed,
    handle,
)


def empty_model(schema="IFC4"):
    """An empty model whose header names the release creation checks against."""
    model = IfcModel()
    model.set_header(dataclasses.replace(model.header, schema=(schema,)))
    return model


def reals(*values):
    return List(tuple(Real(value) for value in values))


def building(model):
    """Units, a context, project, site, building and storey, a wall type, and
    a placed wall contained in the storey and typed."""
    h = handle
    return model.author(
        [
            AuthorOp.create("IfcSIUnit", {"UnitType": Enum("LENGTHUNIT"), "Name": Enum("METRE")}),
            AuthorOp.create("IfcUnitAssignment", {"Units": List((Ref(h(0)),))}),
            AuthorOp.create("IfcCartesianPoint", {"Coordinates": reals(0.0, 0.0, 0.0)}),
            AuthorOp.create("IfcAxis2Placement3D", {"Location": Ref(h(2))}),
            AuthorOp.create(
                "IfcGeometricRepresentationContext",
                {
                    "ContextType": Text("Model"),
                    "CoordinateSpaceDimension": Integer(3),
                    "Precision": Real(1e-5),
                    "WorldCoordinateSystem": Ref(h(3)),
                },
            ),
            AuthorOp.project(
                {
                    "Name": Text("Demo"),
                    "UnitsInContext": Ref(h(1)),
                    "RepresentationContexts": List((Ref(h(4)),)),
                }
            ),
            AuthorOp.placement(),
            AuthorOp.spatial("IfcSite", h(5), placement=h(6)),
            AuthorOp.placement(relative_to=h(6)),
            AuthorOp.spatial("IfcBuilding", h(7), placement=h(8)),
            AuthorOp.placement(relative_to=h(8)),
            AuthorOp.spatial("IfcBuildingStorey", h(9), {"Name": Text("Level 0")}, placement=h(10)),
            AuthorOp.type_object("IfcWallType", {"PredefinedType": Enum("STANDARD")}),
            AuthorOp.placement(
                relative_to=h(10), location=(1, 2, 0), axis=(0, 0, 1), ref_direction=(1, 0, 0)
            ),
            AuthorOp.product(
                "IfcWall",
                {"Name": Text("Wall")},
                container=h(11),
                placement=h(13),
                type_object=h(12),
            ),
        ]
    ).ids


class Authoring(unittest.TestCase):
    def assert_code(self, code, call, *args):
        with self.assertRaises(IfcError) as raised:
            call(*args)
        self.assertEqual(raised.exception.code, code, str(raised.exception))

    def test_documented_example(self):
        model = empty_model("IFC4")
        # docs:snippet py-authoring
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
        # docs:end
        self.assertIsInstance(result, AuthoringResult)
        self.assertEqual(model.type_of(wall), "IFCWALL")
        # The ids a batch returns are what the entity layer (#332) wraps.
        self.assertEqual(model[wall].Name, "Wall")
        self.assertTrue(model[wall].is_a("IfcBuildingElement"))
        self.assertEqual(len(model.attribute_by_name(wall, "GlobalId").value), 22)
        report = model.validate()
        self.assertEqual(report.errors + report.evaluation_errors, 0, report.findings)

    def test_a_model_built_from_nothing_validates_and_round_trips(self):
        model = empty_model()
        wall = building(model)[14]
        # A property set through the #316 call.
        model.set_property(wall, "ACME_WallData", "Mark", Typed("IFCLABEL", Text("W-01")))
        report = model.validate()
        self.assertEqual(report.errors + report.evaluation_errors, 0, report.findings)
        step = model.write()
        self.assertEqual(IfcModel.parse(step).write(), step)
        for profile in (None, "IFC4"):
            back = IfcModel.parse_ifcxml(model.write_ifcxml(xsd_profile=profile), xsd_profile=profile)
            self.assertEqual(back.write(), step, profile)

    def test_a_refused_batch_leaves_the_model_byte_identical(self):
        model = empty_model()
        ids = building(model)
        before = model.write()
        self.assert_code(
            "invalid-model",
            model.author,
            [
                AuthorOp.product("IfcWall", container=ids[11]),
                AuthorOp.contain(ids[11], [ids[14]]),
            ],
        )
        self.assertEqual(model.write(), before)

    def test_single_calls(self):
        model = empty_model()
        ids = building(model)
        proxy = model.create_entity("IfcBuildingElementProxy", {"Name": Text("Proxy")})
        self.assertEqual(model.type_of(proxy), "IFCBUILDINGELEMENTPROXY")
        self.assert_code("still-referenced", model.remove_with_relationships, ids[10])
        model.remove_with_relationships(ids[14])
        self.assertEqual(model.ids_of_type("IfcRelContainedInSpatialStructure"), [])
        self.assertEqual(model.dangling_references(), [])

    def test_refusals_carry_the_shared_codes(self):
        model = empty_model()
        ids = building(model)
        before = model.write()
        cases = [
            ("unsupported-schema", [AuthorOp.create("IfcWal")]),
            ("wrong-entity-type", [AuthorOp.create("IfcElement")]),
            ("unknown-attribute", [AuthorOp.create("IfcWall", {"Nmae": Text("x")})]),
            ("missing-attribute", [AuthorOp.create("IfcWallType")]),
            ("invalid-value", [AuthorOp.create("IfcWall", {"Name": Integer(3)})]),
            ("missing-reference", [AuthorOp.create("IfcWall", {"ObjectPlacement": Ref(99999)})]),
            ("missing-entity", [AuthorOp.edit(99999, {})]),
            ("derived-attribute", [AuthorOp.edit(ids[0], {"Dimensions": Null()})]),
            ("invalid-model", [AuthorOp.project()]),
            ("still-referenced", [AuthorOp.remove(ids[10])]),
            ("invalid-value", [AuthorOp.placement(axis=(0, 0, 1))]),
            ("invalid-value", [AuthorOp.placement(location=(1, 2))]),
            ("invalid-value", [AuthorOp("build")]),
            ("invalid-value", [AuthorOp("create", (("type", "IfcWall"), ("colour", "red")))]),
            ("invalid-value", [AuthorOp.create("IfcWall", {"ObjectPlacement": Ref(handle(0))})]),
        ]
        for code, ops in cases:
            with self.subTest(code=code, ops=ops):
                self.assert_code(code, model.author, ops)
                self.assertEqual(model.write(), before)
        self.assert_code("missing-attribute", empty_model("IFC2X3").author, [AuthorOp.project()])
        self.assert_code("unsupported-schema", IfcModel().author, [AuthorOp.project()])
        with self.assertRaises(TypeError):
            model.author(["project"])
        with self.assertRaises(ValueError):
            handle(-1)


    def test_assign_type_propagates_and_replaces(self):
        # #480: map the type's representation, derive the layer set usage,
        # sync PredefinedType, replace the previous type and container.
        model = empty_model()
        ids = building(model)
        context, building_id, storey, wall_type, wall = ids[4], ids[9], ids[11], ids[12], ids[14]
        h = handle
        ops = [
            AuthorOp.create("IfcCartesianPoint", {"Coordinates": reals(0.0, 0.0, 0.0)}),  # 0
            AuthorOp.create("IfcAxis2Placement3D", {"Location": Ref(h(0))}),  # 1
            AuthorOp.create(
                "IfcBoundingBox",
                {"Corner": Ref(h(0)), "XDim": Real(5.0), "YDim": Real(0.2), "ZDim": Real(3.0)},
            ),  # 2
            AuthorOp.create(
                "IfcShapeRepresentation",
                {
                    "ContextOfItems": Ref(context),
                    "RepresentationIdentifier": Text("Body"),
                    "RepresentationType": Text("BoundingBox"),
                    "Items": List((Ref(h(2)),)),
                },
            ),  # 3
            AuthorOp.create(
                "IfcRepresentationMap", {"MappingOrigin": Ref(h(1)), "MappedRepresentation": Ref(h(3))}
            ),  # 4
            AuthorOp.create("IfcMaterial", {"Name": Text("Concrete")}),  # 5
            AuthorOp.create("IfcMaterialLayer", {"Material": Ref(h(5)), "LayerThickness": Real(0.2)}),  # 6
            AuthorOp.create("IfcMaterialLayerSet", {"MaterialLayers": List((Ref(h(6)),))}),  # 7
            AuthorOp.type_object(
                "IfcWallType",
                {"PredefinedType": Enum("SOLIDWALL"), "RepresentationMaps": List((Ref(h(4)),))},
            ),  # 8
            AuthorOp.create(
                "IfcRelAssociatesMaterial",
                {"RelatedObjects": List((Ref(h(8)),)), "RelatingMaterial": Ref(h(7))},
            ),  # 9
            AuthorOp.spatial("IfcBuildingStorey", building_id, {"Name": Text("Level 1")}),  # 10
        ]
        added = model.author(ops).ids
        new_type, level_1 = added[8], added[10]
        # Refused by default: the wall is already typed and contained.
        self.assert_code("invalid-model", model.author, [AuthorOp.assign_type(new_type, [wall])])
        self.assert_code("invalid-model", model.author, [AuthorOp.contain(level_1, [wall])])
        self.assert_code(
            "invalid-value",
            model.author,
            [AuthorOp.assign_type(new_type, [wall], replace=True, predefined_type="always")],
        )
        model.author(
            [
                AuthorOp.assign_type(
                    new_type,
                    [wall],
                    map_representations=True,
                    material_usage=True,
                    predefined_type="sync",
                    replace=True,
                ),
                AuthorOp.contain(level_1, [wall], replace=True),
            ]
        )
        self.assertEqual(model.type_object(wall), new_type)
        self.assertNotEqual(wall_type, new_type)
        representation = model[wall].Representation.Representations[0]
        self.assertEqual(representation.RepresentationType, "MappedRepresentation")
        self.assertEqual(representation.Items[0].MappingSource.id, added[4])
        report = model.validate()
        self.assertEqual(report.errors + report.evaluation_errors, 0, report.findings)
        self.assertNotEqual(storey, level_1)


if __name__ == "__main__":
    unittest.main()
