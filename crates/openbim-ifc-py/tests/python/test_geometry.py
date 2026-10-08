"""Geometry from Python (#328): placements in every wheel, meshes in a wheel
built with the ``mesh`` feature, each refusal typed per product. Run by
scripts/check-python.sh, which builds the mesh wheel too and runs this file
against it with OPENBIM_IFC_MESH=1."""

import os
import unittest
from array import array

import openbim_ifc
from openbim_ifc import IfcError, IfcModel, ProductMesh, ProductPlacement

FIXTURE = os.path.join(
    os.path.dirname(__file__),
    "..", "..", "..", "..", "test", "fixtures", "synthetic-bindings", "binding_geometry.ifc",
)
MESH = os.environ.get("OPENBIM_IFC_MESH") == "1"


def model() -> IfcModel:
    with open(FIXTURE, "rb") as handle:
        return IfcModel.parse(handle.read())


class PlacementTests(unittest.TestCase):
    def test_every_product_with_a_shape_is_placed_and_its_body_selected(self) -> None:
        printed = []
        print = printed.append  # noqa: A001 - keeps the snippet's output in the test
        path = FIXTURE
        # docs:snippet py-geometry-placements
        model = openbim_ifc.open(path)
        for product in model.product_placements():
            if product.refusal is not None:
                print((product.id, product.refusal.code, product.refusal.message))
                continue
            # A column-major 4x4 in metres: the origin is its last column.
            x, y, z = product.transform[12:15]
            body = product.representation  # None for an axis-only product
            print((product.type_name, (x, y, z), body and body.representation_type))
        # docs:end
        self.assertEqual(len(printed), 4)

        placements = model.product_placements()
        self.assertEqual([p.id for p in placements], [36, 46, 53, 65])
        wall, _, axis_only, _ = placements
        self.assertIsInstance(wall, ProductPlacement)
        self.assertEqual(wall.type_name, "IFCWALL")
        self.assertEqual(wall.global_id, "2nR5uK8Lw3eT6yH1aJ9sD0")
        expected = (0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 512002, 5403001, 3, 1)
        self.assertIsNotNone(wall.transform)
        for got, want in zip(wall.transform or (), expected):
            self.assertAlmostEqual(got, want, places=6)
        body = wall.representation
        assert body is not None
        self.assertEqual((body.id, body.identifier, body.representation_type), (30, "Body", "SweptSolid"))
        self.assertEqual((body.context, body.context_type, body.target_view), (7, "Model", "MODEL_VIEW"))
        self.assertIsNone(axis_only.representation)
        self.assertIsNone(axis_only.refusal)

    def test_a_selection_and_a_missing_id(self) -> None:
        text, missing = model().product_placements([65, 9999])
        assert text.representation is not None
        self.assertEqual(text.representation.id, 63)
        assert missing.refusal is not None
        self.assertEqual(missing.refusal.code, "missing-reference")


class MeshTests(unittest.TestCase):
    @unittest.skipIf(MESH, "this wheel was built with the mesh feature")
    def test_meshes_are_opt_in(self) -> None:
        with self.assertRaises(IfcError) as raised:
            model().product_meshes()
        self.assertEqual(raised.exception.code, "feature-disabled")

    @unittest.skipUnless(MESH, "needs a wheel built with --features mesh")
    def test_meshes_come_as_arrays_relative_to_each_product(self) -> None:
        printed = []
        print = printed.append  # noqa: A001
        path = FIXTURE
        # docs:snippet py-geometry-meshes
        model = openbim_ifc.open(path)
        for mesh in model.product_meshes():
            if mesh.refusal is not None:
                print((mesh.id, mesh.refusal.code))  # (65, 'unsupported')
                continue
            # positions: array('f'), x y z per vertex in metres relative to
            # mesh.transform; indices: array('I'), three per triangle.
            print((mesh.type_name, len(mesh.positions) // 3, len(mesh.indices) // 3))
        # docs:end
        self.assertEqual(printed[3], (65, "unsupported"))
        self.assertEqual(printed[2], ("IFCBUILDINGELEMENTPROXY", 0, 0))

        (wall,) = model.product_meshes([36])
        self.assertIsInstance(wall, ProductMesh)
        self.assertIsInstance(wall.positions, array)
        self.assertEqual(wall.positions.typecode, "f")
        self.assertEqual(wall.indices.typecode, "I")
        self.assertEqual(len(wall.positions), wall.vertex_count * 3)
        self.assertEqual(len(wall.indices), wall.triangle_count * 3)
        self.assertGreaterEqual(wall.triangle_count, 12)
        extent = [max(wall.positions[axis::3]) for axis in range(3)]
        for got, want in zip(extent, (2.0, 0.1, 2.8)):
            self.assertAlmostEqual(got, want, places=4)
        self.assertEqual(wall.transform, model.product_placements([36])[0].transform)


if __name__ == "__main__":
    unittest.main()
