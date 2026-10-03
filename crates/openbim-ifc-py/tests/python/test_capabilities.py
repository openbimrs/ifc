"""The #244 surface from Python: lenient reads, the STEP header, validation,
ifcXML and unreachable products. Run by scripts/check-python.sh."""

import dataclasses
import unittest

from openbim_ifc import (
    Header,
    IfcError,
    IfcModel,
    ParseOptions,
    Real,
    Typed,
    UnreachableProduct,
    ValidationReport,
)

FILE = b"""ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('py.ifc','2026-10-03T00:00:00',('Ann'),('Org'),'pre','sys','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPERSON($,'Doe',$,$,$,$,$,$);
#2=IFCPROPERTYSINGLEVALUE('Length',$,IFCLENGTHMEASURE(1E-05),$);
#3=IFCWALL('x',,;
#4=IFCRELDEFINESBYPROPERTIES('0def',$,$,$,(#9),#8);
ENDSEC;
END-ISO-10303-21;
"""

LENIENT = ParseOptions.lenient()


class Capabilities(unittest.TestCase):
    def assertCode(self, code, call, *args, **kwargs):
        with self.assertRaises(IfcError) as caught:
            call(*args, **kwargs)
        self.assertEqual(caught.exception.code, code, str(caught.exception))

    def test_a_lenient_read_recovers_and_reports(self):
        self.assertCode("parse", IfcModel.parse, FILE)
        model = IfcModel.parse(FILE, options=LENIENT)
        self.assertEqual(model.ids(), [1, 2, 4])
        self.assertEqual(len(model.diagnostics()), 2)
        self.assertEqual(model.attribute(2, 2), Typed("IFCLENGTHMEASURE", Real(1e-05)))
        self.assertEqual(LENIENT, ParseOptions("skip", False, True))
        self.assertCode("invalid-value", IfcModel.parse, FILE, options=ParseOptions("drop"))
        self.assertCode(
            "invalid-value", IfcModel.parse, FILE, options=ParseOptions("skip", check_references=1)
        )

    def test_a_lenient_open_reads_from_disk(self):
        import os
        import tempfile

        handle, path = tempfile.mkstemp(suffix=".ifc")
        with os.fdopen(handle, "wb") as file:
            file.write(FILE)
        self.addCleanup(os.remove, path)
        self.assertCode("parse", IfcModel.open, path)
        for mapped in (False, True):
            self.assertEqual(IfcModel.open(path, mapped=mapped, options=LENIENT).ids(), [1, 2, 4])

    def test_the_header_reads_and_a_replacement_is_written(self):
        model = IfcModel.parse(FILE, options=LENIENT)
        header = model.header
        self.assertIsInstance(header, Header)
        self.assertEqual(header.name, "py.ifc")
        self.assertEqual(header.author, ("Ann",))
        self.assertEqual(header.schema, ("IFC4",))
        edited = dataclasses.replace(header, name="edited.ifc", author=("Zoë",))
        model.set_header(edited)
        self.assertEqual(IfcModel.parse(model.write()).header, edited)
        with self.assertRaises(TypeError):
            model.set_header({"name": "x"})
        self.assertCode("invalid-value", model._native.set_header, {"name": "x"})
        self.assertEqual(model.header, edited, "a refused header changes nothing")

    def test_validation_returns_frozen_records(self):
        model = IfcModel.parse(FILE, options=LENIENT)
        report = model.validate()
        self.assertIsInstance(report, ValidationReport)
        self.assertFalse(report.conformant)
        on_four = [f for f in report.findings if f.entity == 4 and f.severity == "error"]
        self.assertTrue(on_four, [f.path for f in report.findings])
        self.assertEqual(report.errors, sum(f.severity == "error" for f in report.findings))
        capped = model.validate(max_findings=1)
        self.assertEqual(len(capped.findings), 1)
        self.assertTrue(capped.truncated)
        with self.assertRaises(dataclasses.FrozenInstanceError):
            report.conformant = True
        self.assertCode("unsupported-schema", IfcModel().validate)

    def test_ifcxml_round_trips_in_both_layouts(self):
        model = IfcModel.parse(FILE, options=LENIENT)
        native = IfcModel.parse_ifcxml(model.write_ifcxml())
        self.assertEqual(native.ids(), model.ids())
        for id in model.ids():
            self.assertEqual(native.attributes(id), model.attributes(id))

        person = IfcModel.parse(FILE.split(b"#2=")[0] + b"ENDSEC;\nEND-ISO-10303-21;\n")
        xsd = person.write_ifcxml(xsd_profile="IFC4")
        self.assertIn(b"IFC4/ADD2_TC1/XML", xsd)
        self.assertEqual(
            IfcModel.parse_ifcxml(xsd, xsd_profile="IFC4").attributes(1), person.attributes(1)
        )
        self.assertCode("unsupported-profile", model.write_ifcxml, xsd_profile="IFC2X3")
        self.assertCode("parse", IfcModel.parse_ifcxml, b"<a><b></a>")

    def test_unreachable_products_carry_a_stable_reason(self):
        uncontained = FILE.split(b"DATA;")[0] + (
            b"DATA;\n"
            b"#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$);\n"
            b"#11=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',());\n"
            b"#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#11));\n"
            b"#13=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,#12,$,.STANDARD.);\n"
            b"ENDSEC;\nEND-ISO-10303-21;\n"
        )
        products = IfcModel.parse(uncontained).unreachable_products()
        self.assertEqual(
            products,
            [
                UnreachableProduct(
                    13,
                    "not-contained-in-spatial-structure",
                    (),
                    products[0].message,
                )
            ],
        )
        self.assertTrue(products[0].message)


class DocumentedCapabilities(unittest.TestCase):
    """The example published on the docs site's Python page."""

    def test_lenient_read_header_validation_and_ifcxml(self):
        data = FILE
        # docs:snippet py-beyond-records
        import dataclasses

        from openbim_ifc import IfcModel, ParseOptions

        # A damaged export: skip what cannot be read, and say what was skipped.
        model = IfcModel.parse(data, options=ParseOptions.lenient())
        skipped = model.diagnostics()  # one message per recovery

        header = model.header  # Header(name=..., author=(...), schema=(...), ...)
        model.set_header(dataclasses.replace(header, author=("Reviewer",)))

        report = model.validate()  # ValidationReport(conformant=..., findings=(...))
        errors = [f for f in report.findings if f.severity == "error"]

        xml = model.write_ifcxml()  # lossless ifcXML; or xsd_profile="IFC4"
        from_xml = IfcModel.parse_ifcxml(xml)
        # docs:end
        self.assertEqual(len(skipped), 2)
        self.assertEqual(from_xml.header.author, ("Reviewer",))
        self.assertEqual(len(errors), report.errors)
        self.assertEqual(from_xml.ids(), model.ids())


if __name__ == "__main__":
    unittest.main()
