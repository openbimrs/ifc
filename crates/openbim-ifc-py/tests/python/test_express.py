"""Validation with caller-supplied EXPRESS rules (#538). Run by
scripts/check-python.sh.

The refusals need no schema text. The happy path reads the IFC4 ``.exp`` from
``references/ifc-spec/`` (CC BY-ND 4.0, never committed): it is skipped
without it, and fails when ``IFC_SPEC_REQUIRED`` is set, as the gate does.
"""

import os
import unittest
from pathlib import Path

from openbim_ifc import IfcError, IfcModel

FILE = b"""ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('py.ifc','2026-10-03T00:00:00',('Ann'),('Org'),'pre','sys','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCCARTESIANPOINT((1.,0.));
#3=IFCCARTESIANPOINT((1.,1.,0.));
#4=IFCPOLYLOOP((#1,#2,#3));
ENDSEC;
END-ISO-10303-21;
"""

SPEC = Path(__file__).resolve().parents[4] / "references" / "ifc-spec" / "ifc4-add2-tc1" / "IFC4.exp"
RULE = "IfcPolyLoop.AllPointsSameDim"


def severity(report, rule):
    found = [f.severity for f in report.findings if f.rule == rule]
    return found[0] if found else None


class ExpressValidation(unittest.TestCase):
    def assertCode(self, code, call, *args, **kwargs):
        with self.assertRaises(IfcError) as caught:
            call(*args, **kwargs)
        self.assertEqual(caught.exception.code, code, str(caught.exception))

    def test_text_for_another_release_is_an_express_mismatch(self):
        model = IfcModel.parse(FILE)
        self.assertCode("express-mismatch", model.validate_express, "SCHEMA IFC2X3; END_SCHEMA;")
        self.assertCode("express-mismatch", model.validate_express, "SCHEMA IFC4; END_SCHEMA;")

    def test_text_that_does_not_parse_is_invalid_express(self):
        model = IfcModel.parse(FILE)
        self.assertCode("invalid-express", model.validate_express, "SCHEMA IFC4; ENTITY ; END_SCHEMA;")

    def test_a_plain_run_reports_no_indeterminate_rules(self):
        report = IfcModel.parse(FILE).validate()
        self.assertEqual(report.indeterminate, 0)
        self.assertEqual(severity(report, RULE), "unsupported")

    def test_the_supplied_rules_decide_the_polyloop_rule(self):
        if not SPEC.is_file():
            if os.environ.get("IFC_SPEC_REQUIRED"):
                self.fail(f"IFC_SPEC_REQUIRED is set but {SPEC} was not found")
            self.skipTest("references/ifc-spec not present")
        report = IfcModel.parse(FILE).validate_express(SPEC.read_text(encoding="utf-8"))
        self.assertEqual(severity(report, RULE), "error")
        self.assertFalse(report.conformant)
        self.assertGreaterEqual(report.errors, 1)


if __name__ == "__main__":
    unittest.main()
