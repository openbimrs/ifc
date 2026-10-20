"""Unit conversion and convert_length_unit (#469). The conversion itself is
pinned in openbim-ifc and openbim-ifc-binding-core; this checks how the
functions, the report and the refusals cross into Python."""

import unittest

import openbim_ifc
from openbim_ifc import ConvertedValues, IfcError, IfcModel, LengthUnitReport, Real

MODEL = b"""ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCMONETARYUNIT('EUR');
#4=IFCUNITASSIGNMENT((#1,#2,#3));
#5=IFCPROJECT('0000000000000000000005',$,'P',$,$,$,$,(#6),#4);
#6=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,0.01,#7,$);
#7=IFCAXIS2PLACEMENT3D(#8,$,$);
#8=IFCCARTESIANPOINT((1000.,2500.,0.));
#9=IFCQUANTITYAREA('Area',$,$,7.5,$);
ENDSEC;
END-ISO-10303-21;
"""


def model() -> IfcModel:
    return IfcModel.parse(MODEL)


class ConvertUnit(unittest.TestCase):
    def test_named_units_convert_exactly(self) -> None:
        self.assertEqual(openbim_ifc.convert_unit(1.0, "foot", "MILLIMETRE"), 304.8)
        self.assertEqual(openbim_ifc.convert_unit(1500.0, "MILLI METRE", "METRE"), 1.5)

    def test_refusals_carry_codes(self) -> None:
        for args, code in (
            ((1.0, "METRE", "SQUARE_METRE"), "type-mismatch"),
            ((1.0, "furlong", "METRE"), "invalid-value"),
            ((1.0, "degree Fahrenheit", "KELVIN"), "unsupported"),
        ):
            with self.assertRaises(IfcError) as raised:
                openbim_ifc.convert_unit(*args)
            self.assertEqual(raised.exception.code, code)

    def test_format_length(self) -> None:
        self.assertEqual(
            openbim_ifc.format_length(444.5, "MILLIMETRE", style="feet-inches"),
            "1' - 5 1/2\"",
        )
        self.assertEqual(
            openbim_ifc.format_length(1250.0, "MILLIMETRE", display_unit="METRE"), "1.25"
        )
        self.assertEqual(openbim_ifc.format_length(66.5, "inch", style="inches", places=2), '66 1/2"')

    def test_model_units(self) -> None:
        m = model()
        self.assertEqual(m.convert_unit_value(2.0, 1, 1), 2.0)
        for args, code in (((1.0, 3, 1), "unsupported"), ((1.0, 99, 1), "missing-entity")):
            with self.assertRaises(IfcError) as raised:
                m.convert_unit_value(*args)
            self.assertEqual(raised.exception.code, code)


class ConvertLengthUnit(unittest.TestCase):
    def test_dry_run_then_apply(self) -> None:
        m = model()
        dry = m.convert_length_unit("METRE", dry_run=True)
        self.assertIsInstance(dry, LengthUnitReport)
        self.assertFalse(dry.applied)
        self.assertEqual(dry.removed, (1,))
        self.assertEqual(m.attributes(8)[0].items[0], Real(1000.0))

        report = m.convert_length_unit("METRE")
        self.assertTrue(report.applied)
        self.assertEqual(report.from_unit, "MILLI METRE")
        self.assertEqual(report.to, "METRE")
        self.assertEqual(report.length_factor, 0.001)
        self.assertEqual(report.area_factor, 1.0)
        self.assertIn(ConvertedValues("IFCCARTESIANPOINT", "Coordinates", "length", 1, 3), report.converted)
        self.assertEqual(
            [value.value for value in m.attributes(8)[0].items], [1.0, 2.5, 0.0]
        )

    def test_a_target_that_is_not_a_length_is_refused(self) -> None:
        m = model()
        with self.assertRaises(IfcError) as raised:
            m.convert_length_unit("SQUARE_METRE")
        self.assertEqual(raised.exception.code, "invalid-value")


if __name__ == "__main__":
    unittest.main()
