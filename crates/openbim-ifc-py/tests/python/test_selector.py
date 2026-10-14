"""Selectors and values by path (#464), over the IFC2X3, IFC4 and IFC4X3
selector fixtures. The answers themselves are pinned in
openbim-ifc-binding-core; this checks how they cross into Python."""

import os
import unittest
from types import MappingProxyType

import openbim_ifc
from openbim_ifc import Entity, IfcError, IfcModel, Typed, Text

FIXTURES = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures", "synthetic-relations"
)
RELEASES = ("selector_ifc2x3.ifc", "selector_ifc4.ifc", "selector_ifc4x3.ifc")


def fixture(name: str) -> IfcModel:
    return openbim_ifc.open(os.path.join(FIXTURES, name))


class Select(unittest.TestCase):
    def test_every_release_selects_the_same_ids(self) -> None:
        self.assertEqual(openbim_ifc.SELECTOR_GRAMMAR_VERSION, "1.0")
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                self.assertEqual(model.select_ids("IfcWall, type=WT-200 + IfcDoor"), [17, 18, 28])
                self.assertEqual(model.select_ids("Pset_WallCommon.IsExternal=TRUE"), [17])
                self.assertEqual(model.select_ids('location="Level 1"'), [19, 38, 40])
                walls = model.select("IfcWall")
                self.assertTrue(all(isinstance(wall, Entity) for wall in walls))
                self.assertEqual([wall.Name for wall in walls], ["Wall A", "Wall B", "Wall C"])
                self.assertEqual(model.select_ids("IfcWall", base=[model[17], 28]), [17])

    def test_a_query_that_does_not_parse_names_its_span(self) -> None:
        model = fixture("selector_ifc4.ifc")
        with self.assertRaises(IfcError) as raised:
            model.select_ids("IfcWall, colour=red")
        self.assertEqual(raised.exception.code, "invalid-selector")
        self.assertEqual(raised.exception.span, (9, 15))
        with self.assertRaises(IfcError) as raised:
            model.select_ids("IfcWal")
        self.assertEqual((raised.exception.code, raised.exception.span), ("invalid-selector", (0, 6)))
        self.assertEqual(
            openbim_ifc.canonical_selector(' IfcWall ,container = "Level 1" '),
            'IfcWall, location="Level 1"',
        )


class Values(unittest.TestCase):
    def test_paths_read_plain_values(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                self.assertEqual(model.get_value(17, "type.Name"), "WT-200")
                self.assertEqual(model.get_value(model[17], "type"), model[16])
                self.assertIsNone(model.get_value(19, "type.Name"))
                self.assertEqual(model.get_value(16, "occurrences.Name"), ("Wall A", "Wall B"))
                self.assertEqual(model.get_value(16, "occurrences.count"), 2)
                self.assertEqual(model.get_value(17, "Qto_WallBaseQuantities.Length"), 5.0)
                pset = model.get_value(17, "Pset_WallCommon")
                self.assertIsInstance(pset, MappingProxyType)
                self.assertEqual(
                    dict(pset), {"IsExternal": True, "FireRating": "F90", "AcousticRating": "R45"}
                )

    def test_paths_write_through_one_checked_transaction(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                model.set_value(17, "type.Name", "WT-300")
                self.assertEqual(model[18].get("Name"), "Wall B")
                self.assertEqual(model.get_value(18, "type.Name"), "WT-300")
                model.set_value(19, "Custom_Notes.Note", Typed("IFCLABEL", Text("checked")))
                model.set_value(model[19], "Custom_Notes.Note", "rechecked")
                self.assertEqual(model.get_value(19, "Custom_Notes.Note"), "rechecked")
                with self.assertRaises(IfcError) as raised:
                    model.set_value(17, "Name", 3)
                self.assertEqual(raised.exception.code, "type-mismatch")
                with self.assertRaises(IfcError) as raised:
                    model.set_value(17, "count", 3)
                self.assertEqual(raised.exception.code, "invalid-value")


if __name__ == "__main__":
    unittest.main()
