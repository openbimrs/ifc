"""Undo and redo (#465). Run by scripts/check-python.sh."""

import unittest

from openbim_ifc import HistoryChange, HistoryState, IfcError, IfcModel, Text

FILE = b"""ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('py.ifc','2026-10-03T00:00:00',('Ann'),('Org'),'pre','sys','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCWALL('0abc',$,'Wall',$,$,$,$,$,.STANDARD.);
#2=IFCCARTESIANPOINT((0.,0.,0.));
#3=IFCCARTESIANPOINT((1.,0.,0.));
ENDSEC;
END-ISO-10303-21;
"""


class HistoryTests(unittest.TestCase):
    def test_history_is_off_until_a_limit_is_set(self):
        model = IfcModel.parse(FILE)
        self.assertEqual(model.history(), HistoryState(limit=0, undo=0, redo=0))
        model.remove(2)
        with self.assertRaises(IfcError) as raised:
            model.undo()
        self.assertEqual(raised.exception.code, "history-empty")

    def test_undo_and_redo_restore_the_exact_file(self):
        model = IfcModel.parse(FILE)
        model.set_history_limit(4)
        before = model.write()
        model.remove(2)
        after = model.write()
        self.assertNotEqual(before, after)
        self.assertEqual(model.history(), HistoryState(limit=4, undo=1, redo=0))

        change = model.undo()
        self.assertEqual(
            change, HistoryChange(created=(2,), removed=(), modified=(), header=False)
        )
        self.assertEqual(model.write(), before)
        self.assertEqual(model.history(), HistoryState(limit=4, undo=0, redo=1))

        change = model.redo()
        self.assertEqual(change.removed, (2,))
        self.assertEqual(model.write(), after)
        with self.assertRaises(IfcError) as raised:
            model.redo()
        self.assertEqual(raised.exception.code, "history-empty")

    def test_a_batch_is_one_entry_and_clear_keeps_the_limit(self):
        model = IfcModel.parse(FILE)
        model.set_history_limit(2)
        model.create_entity("IfcBuildingElementProxy", {"Name": Text("Proxy")})
        self.assertEqual(model.history().undo, 1)
        model.clear_history()
        self.assertEqual(model.history(), HistoryState(limit=2, undo=0, redo=0))


if __name__ == "__main__":
    unittest.main()
