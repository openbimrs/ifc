"""Geometry from Python (#328, #367): placements in every wheel, graphs and
meshes in a wheel built with the ``graph`` and ``mesh`` features, each
refusal typed per product. Run by scripts/check-python.sh, which builds that
wheel too (``--features mesh,graph``) and runs this file against it with
OPENBIM_IFC_MESH=1."""

import json
import os
import struct
import unittest
from array import array
from typing import Any, Tuple

import openbim_ifc
from openbim_ifc import (
    GEOMETRY_FORMAT,
    GEOMETRY_FORMAT_VERSION,
    IfcError,
    IfcModel,
    ProductGeometry,
    ProductMesh,
    ProductPlacement,
)

FIXTURE = os.path.join(
    os.path.dirname(__file__),
    "..", "..", "..", "..", "test", "fixtures", "synthetic-bindings", "binding_geometry.ifc",
)
MESH = os.environ.get("OPENBIM_IFC_MESH") == "1"
# The mesh wheel is built with `graph` too: one opt-in geometry wheel.
GRAPH = MESH


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


def decode_cbor(data: bytes) -> Any:
    """Decode one CBOR item (RFC 8949) of the kinds the geometry wire format
    writes: integers, text, arrays, maps, floats of every width, null and
    booleans. Test-only, so the CBOR payload can be compared with the JSON."""

    def item(at: int) -> Tuple[Any, int]:
        head = data[at]
        major, info = head >> 5, head & 0x1F
        at += 1
        if major == 7:
            simple = {20: False, 21: True, 22: None}
            if info in simple:
                return simple[info], at
            width, fmt = {25: (2, ">e"), 26: (4, ">f"), 27: (8, ">d")}[info]
            return struct.unpack_from(fmt, data, at)[0], at + width
        if info < 24:
            n = info
        else:
            width = {24: 1, 25: 2, 26: 4, 27: 8}[info]
            n = int.from_bytes(data[at : at + width], "big")
            at += width
        if major == 0:
            return n, at
        if major == 1:
            return -1 - n, at
        if major == 3:
            return data[at : at + n].decode("utf-8"), at + n
        if major == 4:
            out = []
            for _ in range(n):
                value, at = item(at)
                out.append(value)
            return out, at
        if major == 5:
            mapping = {}
            for _ in range(n):
                key, at = item(at)
                mapping[key], at = item(at)
            return mapping, at
        raise ValueError(f"CBOR major type {major} is not in the wire format")

    value, end = item(0)
    assert end == len(data), "one item, no trailing bytes"
    return value


class GraphTests(unittest.TestCase):
    @unittest.skipIf(GRAPH, "this wheel was built with the graph feature")
    def test_graphs_are_opt_in(self) -> None:
        with self.assertRaises(IfcError) as raised:
            model().product_geometry()
        self.assertEqual(raised.exception.code, "feature-disabled")

    @unittest.skipUnless(GRAPH, "needs a wheel built with --features graph")
    def test_graphs_come_in_axiolids_wire_format(self) -> None:
        printed = []
        print = printed.append  # noqa: A001
        path = FIXTURE
        # docs:snippet py-geometry-graphs
        import json

        model = openbim_ifc.open(path)
        for product in model.product_geometry():
            if product.refusal is not None:
                print((product.id, product.refusal.code))  # (65, 'unsupported')
                continue
            if product.payload is None:
                continue  # no Body: an axis-only product
            # {"format": "axiolid-geometry-graph", "version": "1.1", "graph":
            # {"nodes": [...], "roots": [...]}}, exact, in world metres.
            envelope = json.loads(product.payload)
            print((product.type_name, envelope["version"], len(envelope["graph"]["nodes"])))
        # docs:end
        # The lowest version the content needs; axiolid-model 0.3.9 labels
        # every payload 1.1 (axiolid/kernel#297).
        self.assertEqual((printed[0][0], printed[0][2]), ("IFCWALL", 3))
        self.assertIn(printed[0][1], ("1.0", "1.1"))
        self.assertEqual(printed[2], (65, "unsupported"))

        wall, _, axis_only, text = model.product_geometry()
        self.assertIsInstance(wall, ProductGeometry)
        self.assertEqual((wall.encoding, type(wall.payload)), ("json", str))
        assert isinstance(wall.payload, str)
        self.assertEqual(wall.payload_size, len(wall.payload.encode("utf-8")))
        envelope = json.loads(wall.payload)
        self.assertEqual(envelope["format"], GEOMETRY_FORMAT)
        self.assertIn(envelope["version"], ("1.0", GEOMETRY_FORMAT_VERSION))
        self.assertEqual((GEOMETRY_FORMAT, GEOMETRY_FORMAT_VERSION), ("axiolid-geometry-graph", "1.1"))
        nodes = envelope["graph"]["nodes"]
        self.assertEqual(envelope["graph"]["roots"], [len(nodes) - 1])
        self.assertEqual(list(nodes[-1]), ["Instance"])
        self.assertEqual(nodes[-1]["Instance"]["transform"][9:], [512002.0, 5403001.0, 3.0])
        self.assertEqual(wall.transform, model.product_placements([36])[0].transform)
        self.assertIsNone(axis_only.payload)
        self.assertIsNone(axis_only.refusal)
        assert text.refusal is not None
        self.assertEqual((text.refusal.code, text.refusal.entity), ("unsupported", 62))

    @unittest.skipUnless(GRAPH, "needs a wheel built with --features graph")
    def test_the_cbor_payload_decodes_to_the_json_envelope(self) -> None:
        m = model()
        for as_json, as_cbor in zip(m.product_geometry([36, 46]), m.product_geometry([36, 46], "cbor")):
            self.assertEqual(as_cbor.encoding, "cbor")
            assert isinstance(as_cbor.payload, bytes) and isinstance(as_json.payload, str)
            self.assertEqual(as_cbor.payload_size, len(as_cbor.payload))
            self.assertLess(len(as_cbor.payload), as_json.payload_size)
            self.assertEqual(decode_cbor(as_cbor.payload), json.loads(as_json.payload))
        with self.assertRaises(ValueError):
            m.product_geometry(encoding="xml")


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
