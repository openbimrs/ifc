"""Shape representations (#481) from Python: assemble, assign, copy,
unassign and remove over the IFC2X3, IFC4 and IFC4X3 cascade fixtures. The
operations are pinned in openbim-ifc; this checks how they cross."""

import os
import unittest

import openbim_ifc
from openbim_ifc import AuthorOp, Enum, IfcError, IfcModel, List, Null, Real, Ref, RemovalReport, Text, handle

FIXTURES = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures", "synthetic-relations"
)
RELEASES = ("cascade_ifc2x3.ifc", "cascade_ifc4.ifc", "cascade_ifc4x3.ifc")


def fixture(name: str) -> IfcModel:
    return openbim_ifc.open(os.path.join(FIXTURES, name))


def reals(*values):
    return List(tuple(Real(value) for value in values))


def extrusion():
    """Ops 0..=3; op 3 is a 1.2 x 0.1 x 1.0 extrusion."""
    h = handle
    return [
        AuthorOp.create("IfcCartesianPoint", {"Coordinates": reals(0.6, 0.05)}),
        AuthorOp.create("IfcAxis2Placement2D", {"Location": Ref(h(0))}),
        AuthorOp.create(
            "IfcRectangleProfileDef",
            {"ProfileType": Enum("AREA"), "Position": Ref(h(1)), "XDim": Real(1.2), "YDim": Real(0.1)},
        ),
        AuthorOp.create(
            "IfcExtrudedAreaSolid",
            {"SweptArea": Ref(h(2)), "Position": Ref(9), "ExtrudedDirection": Ref(31), "Depth": Real(1.0)},
        ),
    ]


class Representation(unittest.TestCase):
    def test_assemble_and_assign_infers_the_type(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                result = model.author(
                    extrusion()
                    + [
                        AuthorOp.shape_representation(11, [handle(3)]),
                        AuthorOp.assign_representation(handle(4), 130),
                    ]
                )
                representation, shape = result.ids[4], result.ids[5]
                self.assertEqual(model.attribute_by_name(representation, "RepresentationType"), Text("SweptSolid"))
                self.assertEqual(model.attribute_by_name(representation, "RepresentationIdentifier"), Text("Body"))
                self.assertEqual(model.attribute_by_name(130, "Representation"), Ref(shape))
                self.assertEqual(model.dangling_references(), [])

    def test_copy_unassign_and_remove_in_one_batch(self) -> None:
        model = fixture("cascade_ifc4.ifc")
        result = model.author(
            [
                AuthorOp.copy_representation(25),
                AuthorOp.unassign_representation(25, 20),
                AuthorOp.remove_representation(25),
                AuthorOp.assign_representation(handle(0), 20),
            ]
        )
        copy = result.ids[0]
        self.assertIn(25, result.removed)
        shape = model.attribute_by_name(20, "Representation")
        self.assertEqual(model.attribute_by_name(shape.id, "Representations"), List((Ref(copy),)))

    def test_refusals_have_their_codes(self) -> None:
        model = fixture("cascade_ifc4.ifc")
        cases = (
            (extrusion() + [AuthorOp.shape_representation(11, [handle(3)], representation_type="Brep")], "invalid-value"),
            ([AuthorOp.assign_representation(25, 130)], "invalid-model"),
            ([AuthorOp.unassign_representation(45, 20)], "invalid-model"),
        )
        for ops, code in cases:
            with self.assertRaises(IfcError) as raised:
                model.author(ops)
            self.assertEqual(raised.exception.code, code)

    def test_remove_representation_reports(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                before = len(model)
                dry = model.remove_representation(105, dry_run=True)
                self.assertIsInstance(dry, RemovalReport)
                self.assertFalse(dry.applied)
                self.assertEqual([e.id for e in dry.removed], [104, 105, 106, 107, 108])
                done = model.remove_representation(105)
                self.assertTrue(done.applied)
                self.assertEqual(len(model), before - 5)
                self.assertEqual(model.attribute_by_name(100, "Representation"), Null())
                with self.assertRaises(IfcError) as raised:
                    model.remove_representation(114)
                self.assertEqual(raised.exception.code, "still-referenced")


if __name__ == "__main__":
    unittest.main()
