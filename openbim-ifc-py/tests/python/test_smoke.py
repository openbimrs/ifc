"""Smoke suite for the Python bindings: parse, read, edit, write, re-parse,
and every refusal. Run by openbim-ifc-py/scripts/check-python.sh."""

import threading
import unittest

from openbim_ifc import (
    Binary, Bool, Derived, Enum, IfcError, IfcModel, Integer, List, Null,
    Real, Ref, Text, Typed, Unknown,
)
from openbim_ifc._native import NativeModel

FILE = b"""ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCWALL('0abc',$,'Wall',*,$,$,$,$,.STANDARD.);
#2=IFCPROPERTYSINGLEVALUE('P',$,IFCLOGICAL(.U.),$);
#3=IFCPROPERTYSINGLEVALUE('Q',$,IFCINTEGER(9007199254740993),$);
#4=IFCPROPERTYSINGLEVALUE('R',$,IFCBOOLEAN(.F.),$);
#5=IFCRELDEFINESBYPROPERTIES('g',$,$,$,(#1),#9);
#6=IFCPIXELTEXTURE($,$,$,$,$,1,1,1,("0A1"));
ENDSEC;
END-ISO-10303-21;
"""


class Smoke(unittest.TestCase):
    def setUp(self):
        self.model = IfcModel.parse(FILE)

    def test_parse_reads_header_and_entities_in_file_order(self):
        self.assertEqual(self.model.schema, "IFC4")
        self.assertEqual(len(self.model), 6)
        self.assertEqual(self.model.ids(), [1, 2, 3, 4, 5, 6])
        self.assertEqual(self.model.type_of(1), "IFCWALL")
        self.assertEqual(self.model.diagnostics(), [])

    def test_values_keep_the_distinctions_a_naive_binding_would_lose(self):
        wall = self.model.attributes(1)
        self.assertEqual(wall[1], Null())
        self.assertEqual(wall[3], Derived())
        self.assertNotEqual(wall[1], wall[3])
        self.assertEqual(wall[8], Enum("STANDARD"))
        self.assertEqual(self.model.attribute(2, 2), Typed("IFCLOGICAL", Unknown()))
        self.assertEqual(self.model.attribute(4, 2), Typed("IFCBOOLEAN", Bool(False)))
        self.assertNotEqual(Unknown(), Bool(False))
        # 2^53 + 1: exact, because Python ints are unbounded.
        self.assertEqual(self.model.attribute(3, 2), Typed("IFCINTEGER", Integer(9007199254740993)))
        self.assertEqual(self.model.attribute(5, 4), List((Ref(1),)))
        self.assertEqual(self.model.attribute(6, 8), List((Binary("0A1"),)))
        self.assertEqual(self.model.attribute(1, 50), Null(), "past the end is $")

    def test_an_edit_and_an_added_entity_survive_write_and_reparse(self):
        previous = self.model.set_attribute(1, 2, Text("Renamed"))
        self.assertEqual(previous, Text("Wall"))
        point = self.model.add("IfcCartesianPoint", [List([Real(1.5), Real(-2.0)])])
        self.assertEqual(point, 7)
        again = IfcModel.parse(self.model.write())
        self.assertEqual(again.attribute(1, 2), Text("Renamed"))
        self.assertEqual(again.attributes(point), [List((Real(1.5), Real(-2.0)))])
        self.assertEqual(again.type_of(point), "IFCCARTESIANPOINT")

    def test_type_queries(self):
        self.assertEqual(self.model.ids_of_type("ifcPropertySingleValue"), [2, 3, 4])
        self.assertEqual(self.model.ids_of_type_including_subtypes("IfcBuildingElement"), [1])
        self.assertEqual(self.model.ids_of_type_including_subtypes("IfcWal"), [])

    def test_removal_reports_dangling_references(self):
        self.assertEqual(self.model.dangling_references(), [(5, 9)])
        self.model.remove(1)
        self.assertEqual(sorted(self.model.dangling_references()), [(5, 1), (5, 9)])


class Refusals(unittest.TestCase):
    def setUp(self):
        self.model = IfcModel.parse(FILE)

    def assertCode(self, code, call, *args):
        with self.assertRaises(IfcError) as caught:
            call(*args)
        self.assertEqual(caught.exception.code, code, str(caught.exception))

    def test_every_failure_is_an_ifc_error_with_a_stable_code(self):
        self.assertCode("parse", IfcModel.parse, b"not a step file")
        self.assertCode("missing-entity", self.model.type_of, 99)
        self.assertCode("missing-entity", self.model.remove, 99)
        self.assertCode("missing-entity", self.model.set_attribute, 99, 0, Null())
        self.assertCode("invalid-value", self.model.set_attribute, 1, 2, Real(float("nan")))
        self.assertCode("invalid-value", self.model.set_attribute, 1, 2, Real(float("inf")))
        self.assertCode("invalid-value", self.model.set_attribute, 1, 2, Enum("NOT VALID"))
        self.assertCode("invalid-value", self.model.add, "IFC WALL", [])
        self.assertCode("out-of-range", self.model.set_attribute, 1, 2, Integer(2**63))
        self.assertCode("unsupported-schema", IfcModel().ids_of_type_including_subtypes, "IfcWall")
        self.assertTrue(issubclass(IfcError, Exception))

    def test_a_refused_edit_changes_nothing(self):
        with self.assertRaises(IfcError):
            self.model.set_attribute(1, 2, Enum("bad name"))
        self.assertEqual(self.model.attribute(1, 2), Text("Wall"))

    def test_plain_python_values_are_refused_not_guessed(self):
        for bare in (3, 2.5, "x", None, True):
            with self.assertRaises(TypeError, msg=repr(bare)):
                self.model.set_attribute(1, 2, bare)

    def test_the_native_layer_refuses_malformed_dicts(self):
        native = NativeModel.parse(FILE)
        for bad in (
            {"kind": "bogus"},
            {"kind": "bool", "value": 0},
            {"kind": "integer", "value": True},
            {"kind": "integer", "value": 1.0},
            {"kind": "real", "value": "1"},
            {"kind": "ref", "id": -1},
            {"kind": "text"},
            {"kind": "list", "items": 5},
            "not a dict",
        ):
            with self.assertRaises(IfcError, msg=repr(bad)) as caught:
                native.set_attribute(1, 2, bad)
            self.assertIn(caught.exception.code, ("invalid-value", "out-of-range"))


class Behaviour(unittest.TestCase):
    def test_values_are_frozen_and_hashable(self):
        value = List([Ref(1), Text("a")])
        self.assertIsInstance(value.items, tuple)
        self.assertEqual(hash(value), hash(List((Ref(1), Text("a")))))
        with self.assertRaises(Exception):
            value.items = ()

    def test_an_empty_model_can_be_built_and_written(self):
        model = IfcModel()
        wall = model.add("IfcWall", [Text("g"), Null(), Text("Wall")])
        again = IfcModel.parse(model.write())
        self.assertEqual(again.ids(), [wall])
        self.assertEqual(again.attribute(wall, 2), Text("Wall"))

    def test_a_model_can_be_used_from_another_thread(self):
        model = IfcModel.parse(FILE)
        seen = []

        def use_elsewhere():
            model.set_attribute(1, 2, Text("From a thread"))
            seen.append(len(model))

        thread = threading.Thread(target=use_elsewhere)
        thread.start()
        thread.join()
        self.assertEqual(seen, [6])
        self.assertEqual(model.attribute(1, 2), Text("From a thread"))


if __name__ == "__main__":
    unittest.main()
