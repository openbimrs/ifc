"""Attributes by name (#326), resolved against the declared release."""

import unittest

from openbim_ifc import AttributeInfo, Derived, Enum, IfcError, IfcModel, Null, Ref, Text


def file(schema: str, data: str) -> bytes:
    return f"""ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('t.ifc','2026-10-03T00:00:00',('a'),('o'),'p','s','');
FILE_SCHEMA(('{schema}'));
ENDSEC;
DATA;
{data}
ENDSEC;
END-ISO-10303-21;
""".encode()


IFC4 = file(
    "IFC4",
    """#1=IFCTASK('0YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1',$,'Planned','Crane',.F.,1,$,$);
#2=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);""",
)
IFC2X3 = file(
    "IFC2X3",
    "#1=IFCTASK('0YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1','Planned','Crane',.F.,1);",
)


class ByName(unittest.TestCase):
    def code(self, call) -> str:
        with self.assertRaises(IfcError) as caught:
            call()
        return caught.exception.code

    def test_names_are_listed_in_slot_order_with_their_flags(self):
        model = IfcModel.parse(IFC4)
        names = model.attribute_names(2)
        self.assertIsInstance(names[0], AttributeInfo)
        self.assertEqual(
            names[0],
            AttributeInfo("GlobalId", 0, "IfcGloballyUniqueId", False, False, False, "IfcRoot"),
        )
        self.assertEqual([info.index for info in names], list(range(len(names))))
        self.assertTrue(model.attribute_names(3)[0].derived)

    def test_a_name_resolves_to_the_slot_of_the_declared_release(self):
        for data, slot in ((IFC2X3, 6), (IFC4, 7)):
            model = IfcModel.parse(data)
            self.assertEqual(model.attribute_by_name(1, "status"), Text("Planned"))
            self.assertEqual(model.set_attribute_by_name(1, "Status", Text("Done")), Text("Planned"))
            self.assertEqual(model.attribute(1, slot), Text("Done"))
        self.assertEqual(self.code(lambda: IfcModel.parse(IFC4).attribute_by_name(1, "TaskId")), "unknown-attribute")

    def test_every_refusal_has_its_code_and_writes_nothing(self):
        model = IfcModel.parse(IFC4)
        before = model.write()
        self.assertEqual(model.attribute_by_name(2, "PredefinedType"), Enum("STANDARD"))
        self.assertEqual(model.attribute_by_name(3, "Dimensions"), Derived())
        self.assertEqual(self.code(lambda: model.set_attribute_by_name(3, "Dimensions", Null())), "derived-attribute")
        self.assertEqual(self.code(lambda: model.attribute_by_name(2, "IsDefinedBy")), "unknown-attribute")
        self.assertEqual(self.code(lambda: model.set_attribute_by_name(2, "Nmae", Ref(1))), "unknown-attribute")
        self.assertEqual(self.code(lambda: model.attribute_names(99)), "missing-entity")
        unknown = IfcModel.parse(IFC4.replace(b"'IFC4'", b"'IFC9'"))
        self.assertEqual(self.code(lambda: unknown.attribute_by_name(2, "Name")), "unsupported-schema")
        self.assertEqual(model.write(), before)


if __name__ == "__main__":
    unittest.main()
