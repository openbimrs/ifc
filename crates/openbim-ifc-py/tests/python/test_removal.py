"""remove_product and purge (#478), over the IFC2X3, IFC4 and IFC4X3 cascade
fixtures. The cascade itself is pinned in openbim-ifc and
openbim-ifc-binding-core; this checks how the reports cross into Python."""

import os
import unittest

import openbim_ifc
from openbim_ifc import IfcError, IfcModel, KeptEntity, RemovalReport, RemovedEntity

FIXTURES = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures", "synthetic-relations"
)
RELEASES = ("cascade_ifc2x3.ifc", "cascade_ifc4.ifc", "cascade_ifc4x3.ifc")


def fixture(name: str) -> IfcModel:
    return openbim_ifc.open(os.path.join(FIXTURES, name))


class RemoveProduct(unittest.TestCase):
    def test_a_dry_run_reports_and_the_removal_applies(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                before = len(model)
                dry = model.remove_product(100, dry_run=True)
                self.assertIsInstance(dry, RemovalReport)
                self.assertFalse(dry.applied)
                self.assertEqual(len(model), before)
                self.assertEqual(dry.removed[0], RemovedEntity(100, "IFCDOOR"))
                self.assertIn(KeptEntity(112, "IFCREPRESENTATIONMAP", "shared", (111,)), dry.kept)

                done = model.remove_product(100)
                self.assertTrue(done.applied)
                self.assertEqual(done.removed, dry.removed)
                self.assertEqual(len(model), before - 11)
                self.assertEqual(model.dangling_references(), [])

    def test_refusals_and_options(self) -> None:
        model = fixture("cascade_ifc4.ifc")
        for id, code in ((20, "still-referenced"), (66, "still-referenced"), (61, "wrong-entity-type")):
            with self.assertRaises(IfcError) as raised:
                model.remove_product(id)
            self.assertEqual(raised.exception.code, code)
        self.assertEqual(model.remove_product(20, rebase_placements=True).rebased, (101, 131))
        untyped = model.remove_product(66, untype_occurrences=True)
        self.assertEqual([entity.id for entity in untyped.removed], [66, 67, 68, 69])


class Purge(unittest.TestCase):
    def test_by_class_then_by_default(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                skipped = model.purge(["IfcPropertySet"], dry_run=True)
                self.assertEqual(skipped.removed, ())
                self.assertEqual(skipped.kept[0].reason, "protected")
                self.assertEqual(
                    [e.id for e in model.purge(["IfcPropertySet"], include_roots=True).removed], [173]
                )
                self.assertEqual(
                    [e.id for e in model.purge().removed], [170, 171, 172, 174]
                )

    def test_an_unknown_class_is_refused(self) -> None:
        with self.assertRaises(IfcError) as raised:
            fixture("cascade_ifc4.ifc").purge(["IfcNoSuchThing"])
        self.assertEqual(raised.exception.code, "unsupported-schema")


if __name__ == "__main__":
    unittest.main()
