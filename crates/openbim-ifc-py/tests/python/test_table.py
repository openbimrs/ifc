"""Tables of values by path (#474), over the IFC2X3, IFC4 and IFC4X3
selector fixtures. The answers are pinned in openbim-ifc-binding-core; this
checks how they cross into Python."""

import os
import unittest

import openbim_ifc
from openbim_ifc import IfcError, IfcModel, TableChange, TableImport, TableRefusal

FIXTURES = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures", "synthetic-relations"
)
RELEASES = ("selector_ifc2x3.ifc", "selector_ifc4.ifc", "selector_ifc4x3.ifc")


def fixture(name: str) -> IfcModel:
    return openbim_ifc.open(os.path.join(FIXTURES, name))


class Tables(unittest.TestCase):
    def test_an_exported_table_imports_back_as_a_no_op(self) -> None:
        for name in RELEASES:
            for format in ("csv", "tsv", "xlsx"):
                with self.subTest(name=name, format=format):
                    model = fixture(name)
                    before = model.write()
                    table = model.export_table(
                        "IfcWall", ["Name", "type.Name", "Pset_WallCommon.FireRating"], format=format
                    )
                    self.assertIsInstance(table, bytes)
                    result = model.import_table(table, format=format)
                    self.assertEqual(result, TableImport(applied=False, changes=(), refusals=()))
                    self.assertEqual(model.write(), before)

    def test_edits_are_planned_then_applied(self) -> None:
        model = fixture("selector_ifc4.ifc")
        text = model.export_table("IfcWall", ["Name", "type.Name"], key="id").decode()
        self.assertTrue(text.startswith("id,Name,type.Name\r\n17,Wall A,WT-200\r\n"), text)
        edited = text.replace("17,Wall A,WT-200", "17,Wall A1,WT-250")
        dry = model.import_table(edited, dry_run=True)
        self.assertFalse(dry.applied)
        self.assertEqual(
            dry.changes[1],
            TableChange(
                row=2,
                column=3,
                key="17",
                path="type.Name",
                entity=17,
                target_entity=16,
                target="Name",
                before="WT-200",
                after="WT-250",
            ),
        )
        self.assertEqual(model[17].Name, "Wall A")
        applied = model.import_table(edited)
        self.assertTrue(applied.applied)
        self.assertEqual(model[17].Name, "Wall A1")
        self.assertEqual(model[16].Name, "WT-250")

    def test_refusals_are_data_and_write_nothing(self) -> None:
        model = fixture("selector_ifc4.ifc")
        before = model.write()
        result = model.import_table("id,Name,Name.\n17,X,\n99,Y,\n")
        self.assertFalse(result.applied)
        self.assertEqual(
            result.refusals[0],
            TableRefusal(
                row=1,
                column=3,
                code="invalid-header",
                message=result.refusals[0].message,
                span=(5, 5),
            ),
        )
        self.assertEqual([refusal.code for refusal in result.refusals], ["invalid-header", "unknown-key"])
        self.assertEqual(model.write(), before)
        with self.assertRaises(IfcError) as raised:
            model.export_table("IfcWall", [], format="ods")
        self.assertEqual(raised.exception.code, "invalid-value")


if __name__ == "__main__":
    unittest.main()
