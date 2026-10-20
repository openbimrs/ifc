"""Copying products and entities, and appending from another model (#479),
from Python, over the cascade fixtures. The copies are pinned in
openbim-ifc; this checks how they cross."""

import os
import unittest

import openbim_ifc
from openbim_ifc import AuthorOp, CopiedEntity, CopyReport, IfcError, IfcModel, Ref, Text

FIXTURES = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures", "synthetic-relations"
)
RELEASES = ("cascade_ifc2x3.ifc", "cascade_ifc4.ifc", "cascade_ifc4x3.ifc")


def fixture(name: str) -> IfcModel:
    return openbim_ifc.open(os.path.join(FIXTURES, name))


def originals(report: CopyReport) -> set:
    return {entity.original for entity in report.copied}


class CopyProduct(unittest.TestCase):
    def test_a_wall_copies_what_it_owns_and_joins_what_it_shares(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                planned = model.copy_product(20, dry_run=True)
                self.assertIsInstance(planned, CopyReport)
                self.assertFalse(planned.applied)
                report = model.copy_product(20)
                self.assertTrue(report.applied)
                self.assertIsInstance(report.copied[0], CopiedEntity)
                self.assertEqual(model.attribute_by_name(report.copy, "Name"), Text("Wall A"))
                # Own property set, openings and their door copied; the
                # type assignment and the container joined.
                self.assertTrue({64, 81, 100} <= originals(report))
                self.assertIn(67, report.joined)
                self.assertIn(19, report.joined)
                validation = model.validate()
                self.assertEqual(validation.errors, 0, validation.findings)

    def test_options_cross(self) -> None:
        model = fixture("cascade_ifc4.ifc")
        report = model.copy_product(20, offset=(0.0, 4.0, 0.0), openings=False, container=False)
        self.assertNotIn(81, originals(report))
        self.assertIn(19, report.skipped)
        with self.assertRaises(IfcError) as raised:
            model.copy_product(20, offset=(1.0, 0.0, 0.0), placement=18)
        self.assertEqual(raised.exception.code, "invalid-value")
        with self.assertRaises(IfcError) as raised:
            model.copy_product(61)
        self.assertEqual(raised.exception.code, "wrong-entity-type")

    def test_copy_operations_in_a_batch(self) -> None:
        model = fixture("cascade_ifc4.ifc")
        result = model.author(
            [
                AuthorOp.copy_product(100, fillings=False),
                AuthorOp.copy_entity(26, share=["IfcProfileDef"]),
            ]
        )
        door, solid = result.ids
        self.assertEqual(model.attribute_by_name(door, "Name"), Text("Door 1"))
        self.assertEqual(model.attribute_by_name(solid, "SweptArea"), Ref(27))


class Append(unittest.TestCase):
    def test_modes_reuse_and_refusals(self) -> None:
        source = fixture("cascade_ifc4.ifc")
        target = fixture("cascade_ifc4.ifc")
        with self.assertRaises(IfcError) as raised:
            target.append(source, [66])
        self.assertEqual(raised.exception.code, "invalid-value")
        merged = target.append(source, [66], global_ids="merge")
        self.assertEqual(merged.copied, ())
        self.assertEqual(merged.reused[0].copy, 66)
        report = target.append(source, [20], global_ids="regenerate")
        self.assertIn(19, report.skipped)
        self.assertTrue(report.regenerated)
        self.assertIn(5, {entity.original for entity in report.reused})
        copied_everything = target.append(source, [111], global_ids="regenerate", reuse=())
        self.assertNotIn(11, {entity.original for entity in copied_everything.reused})
        with self.assertRaises(IfcError) as raised:
            target.append(source, [20], reuse=["paint"])
        self.assertEqual(raised.exception.code, "invalid-value")
        old = fixture("cascade_ifc2x3.ifc")
        with self.assertRaises(IfcError) as raised:
            old.append(source, [111], global_ids="regenerate")
        self.assertEqual(raised.exception.code, "unsupported-schema")


if __name__ == "__main__":
    unittest.main()
