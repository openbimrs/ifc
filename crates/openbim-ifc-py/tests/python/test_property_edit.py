"""Writing property sets from Python (#123, part 2): one checked
transaction per batch, values read back identically, the shared codes, and
a refused batch that leaves the model unchanged. Run by
scripts/check-python.sh."""

import os
import unittest

from openbim_ifc import (
    IfcError,
    IfcModel,
    List,
    PropertyEdit,
    PropertyEditResult,
    Real,
    Text,
    Typed,
)

FIXTURES = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures")


def fixture():
    return IfcModel.open(
        os.path.join(FIXTURES, "synthetic-properties", "synthetic_properties.ifc")
    )


def label(text):
    return Typed("IFCLABEL", Text(text))


def values(model, object):
    return [
        (s.source, s.name, p.name, p.value)
        for s in model.property_sets(object)
        for p in s.properties
    ]


class PropertyEdits(unittest.TestCase):
    def assertRefused(self, model, code, edits):
        before = model.write()
        with self.assertRaises(IfcError) as caught:
            model.set_properties(edits)
        self.assertEqual(caught.exception.code, code, str(caught.exception))
        self.assertEqual(model.write(), before, code)

    def test_documented_example(self):
        model = fixture()
        # docs:snippet py-domain-write
        from openbim_ifc import PropertyEdit, Real, Text, Typed

        # Wall #31 inherits FireRating from its type: the write overrides it
        # on the wall and never changes the type's shared set.
        result = model.set_properties([
            PropertyEdit(31, "Pset_WallCommon", "FireRating", Typed("IFCLABEL", Text("F60"))),
            PropertyEdit(30, "Qto_WallBaseQuantities", "Width", Typed("IFCLENGTHMEASURE", Real(250.0))),
            PropertyEdit.removal(30, "Pset_WallCommon", "IsExternal"),
        ])
        # result.properties: per edit, the entity now holding the value
        own = next(s for s in model.property_sets(31) if s.source == "occurrence" and s.name == "Pset_WallCommon")
        # docs:end
        self.assertIsInstance(result, PropertyEditResult)
        self.assertEqual(len(result.properties), 3)
        self.assertIsNone(result.properties[2])
        self.assertEqual(own.properties[0].value, label("F60"))
        self.assertEqual(model.attribute(35, 2), label("F30"), "the type's value is unchanged")

    def test_values_read_back_identically_through_step_and_ifcxml(self):
        model = fixture()
        model.set_properties([
            PropertyEdit(31, "Custom", "Note", label("checked")),
            PropertyEdit(31, "Custom", "Load", Typed("IFCFORCEMEASURE", Real(1.5))),
            PropertyEdit(
                31,
                "Custom_Quantities",
                "Depth",
                Typed("IFCLENGTHMEASURE", Real(0.25)),
                set_type="IfcElementQuantity",
            ),
            PropertyEdit(31, "Pset_Families", "Colour", List((label("green"),))),
        ])
        expected = values(model, 31)
        self.assertIn(("occurrence", "Custom", "Note", label("checked")), expected)
        self.assertEqual(values(IfcModel.parse(model.write()), 31), expected)
        self.assertEqual(values(IfcModel.parse_ifcxml(model.write_ifcxml()), 31), expected)

    def test_single_edits_are_thin_wrappers(self):
        model = fixture()
        id = model.set_property(31, "Custom", "Note", label("x"))
        self.assertEqual(model.type_of(id), "IFCPROPERTYSINGLEVALUE")
        model.remove_property(31, "Custom", "Note")
        self.assertNotIn("Custom", [s.name for s in model.property_sets(31)])

    def test_every_refusal_has_its_code_and_changes_nothing(self):
        model = fixture()
        self.assertRefused(
            model,
            "template-violation",
            [PropertyEdit(31, "Pset_WallCommon", "FireRating", Typed("IFCREAL", Real(1.0)))],
        )
        self.assertRefused(
            model, "missing-property", [PropertyEdit.removal(31, "Pset_WallCommon", "IsExternal")]
        )
        self.assertRefused(model, "invalid-value", [PropertyEdit(31, "Custom", "A", Real(1.0))])
        self.assertRefused(
            model,
            "missing-entity",
            [PropertyEdit(31, "Custom", "A", label("valid")), PropertyEdit(999, "Custom", "A", label("x"))],
        )
        self.assertRefused(model, "invalid-value", [PropertyEdit(31, "Custom", "A")])
        with self.assertRaises(TypeError):
            model.set_properties([("not", "an", "edit")])
        ifc4x1 = IfcModel.parse(
            open(os.path.join(FIXTURES, "synthetic-properties", "synthetic_properties.ifc"), "rb")
            .read()
            .replace(b"('IFC4')", b"('IFC4X1')", 1)
        )
        self.assertRefused(ifc4x1, "unsupported-schema", [PropertyEdit(31, "Custom", "A", label("x"))])


if __name__ == "__main__":
    unittest.main()
