"""The domain views from Python (#123): property sets and units, the spatial
tree, classification, materials, systems, cost and georeferencing, as frozen
dataclasses. Run by scripts/check-python.sh."""

import contextlib
import dataclasses
import io
import os
import unittest

from openbim_ifc import (
    Bool,
    IfcError,
    IfcModel,
    MaterialAssignment,
    PropertySet,
    Real,
    Typed,
    Unknown,
)

FIXTURES = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures")

# A wall classified directly, with a type that holds a property set and a
# layer set: no shipped fixture states classification or layered materials.
CLASSIFIED = b"""ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'WT',$,$,(#30),$,$,$,.SOLIDWALL.);
#3=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#4=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#3),#2);
#10=IFCCLASSIFICATION('CSI',$,$,'Uniclass 2015',$,$,$);
#12=IFCCLASSIFICATIONREFERENCE($,'Ss_25_10','Wall systems',#10,$,$);
#13=IFCRELASSOCIATESCLASSIFICATION('0ZvctVUKr0kugbFTf53O9L',$,$,$,(#3),#12);
#20=IFCMATERIAL('Concrete',$,$);
#22=IFCMATERIALLAYER(#20,0.2,.U.,'Core',$,$,$);
#24=IFCMATERIALLAYERSET((#22),'WT-200',$);
#26=IFCRELASSOCIATESMATERIAL('2ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#24);
#30=IFCPROPERTYSET('3ZvctVUKr0kugbFTf53O9L',$,'Pset_WallCommon',$,(#31));
#31=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);
ENDSEC;
END-ISO-10303-21;
"""


def fixture(name):
    return IfcModel.open(os.path.join(FIXTURES, name))


class Domains(unittest.TestCase):
    def assertCode(self, code, call, *args):
        with self.assertRaises(IfcError) as caught:
            call(*args)
        self.assertEqual(caught.exception.code, code, str(caught.exception))

    def test_property_sets_resolve_type_inheritance_with_typed_values(self):
        model = fixture("synthetic-properties/synthetic_properties.ifc")
        sets = model.property_sets(30)
        self.assertTrue(all(isinstance(s, PropertySet) for s in sets))
        self.assertEqual(
            [(s.name, s.source, s.source_id) for s in sets],
            [
                ("Pset_WallCommon", "occurrence", None),
                ("Qto_WallBaseQuantities", "occurrence", None),
                ("Pset_WallCommon", "type", 29),
            ],
        )
        self.assertEqual(sets[0].properties[0].value, Typed("IFCBOOLEAN", Bool(False)))
        width = sets[1].properties[0]
        self.assertEqual((width.type_name, width.unit), ("IFCQUANTITYLENGTH", 7))
        self.assertEqual(width.value, Typed("IFCLENGTHMEASURE", Real(200.0)))
        unit = model.resolve_unit("IFCLENGTHMEASURE", width.unit)
        self.assertFalse(unit.from_project)
        self.assertAlmostEqual(unit.scale, 0.001)
        with self.assertRaises(dataclasses.FrozenInstanceError):
            sets[0].name = "changed"

    def test_spatial_tree_systems_cost_and_georeferencing(self):
        tree = fixture("synthetic-properties/synthetic_properties.ifc").spatial_tree()
        self.assertEqual(
            [(n.id, n.kind) for n in tree.nodes],
            [(22, "project"), (23, "site"), (24, "building"), (25, "storey")],
        )
        self.assertEqual(tree.nodes[3].elements, (30, 31))

        systems = fixture("synthetic-systems/synthetic_systems.ifc").systems()
        heating = next(s for s in systems.systems if s.id == 14)
        self.assertEqual(heating.predefined_type, "HEATING")
        self.assertEqual(systems.anomalies[0].kind, "not-a-system")

        cost = fixture("synthetic-cost-schedule/synthetic_cost_schedule.ifc").cost()
        self.assertEqual(cost.schedules[0].items, (38, 42, 44))
        setup = next(i for i in cost.items if i.id == 44).values[0]
        self.assertEqual(setup.operator, "ADD")
        self.assertEqual(
            setup.components[0].applied_value, Typed("IFCMONETARYMEASURE", Real(320.0))
        )

        (conversion,) = fixture(
            "synthetic-surfaces/synthetic_conic_offset_bounded.ifc"
        ).georeferencing()
        self.assertEqual(conversion.target_crs.name, "EPSG:25832")
        self.assertEqual(conversion.translation, (1.0, 2.0, 0.01))

    def test_documented_example(self):
        data = CLASSIFIED
        printed = io.StringIO()
        with contextlib.redirect_stdout(printed):
            self._documented(data)
        self.assertEqual(
            printed.getvalue(),
            "Pset_WallCommon IsExternal Typed(type='IFCBOOLEAN', value=Bool(value=True))\n",
        )

    def _documented(self, data):
        # docs:snippet py-domain-views
        from openbim_ifc import IfcModel

        model = IfcModel.parse(data)
        (wall,) = model.ids_of_type("IfcWall")

        # Property sets: the wall's own first, then its type's; values typed.
        for pset in model.property_sets(wall):
            for prop in pset.properties:
                print(pset.name, prop.name, prop.value)

        classes = model.classifications(wall)  # (Classification(identification=...), ...)
        material = model.material(wall)  # MaterialAssignment(kind="layer-set", layers=(...))
        tree = model.spatial_tree()  # SpatialTree(nodes=(SpatialNode(kind=...), ...))
        # docs:end
        self.assertEqual(classes[0].identification, "Ss_25_10")
        self.assertEqual(classes[0].system.name, "Uniclass 2015")
        self.assertIsInstance(material, MaterialAssignment)
        self.assertEqual((material.kind, material.source), ("layer-set", "type"))
        self.assertEqual(material.layers[0].is_ventilated, Unknown())
        self.assertEqual(tree.nodes, ())
        inherited = model.property_sets(wall)[0]
        self.assertEqual((inherited.source, inherited.source_id), ("type", 2))

    def test_domain_refusals_carry_the_shared_codes(self):
        model = IfcModel.parse(CLASSIFIED)
        self.assertCode("missing-entity", model.property_sets, 999)
        ifc2x3 = IfcModel.parse(CLASSIFIED.replace(b"'IFC4'", b"'IFC2X3'"))
        self.assertCode("unsupported-schema", ifc2x3.georeferencing)
        ifc4x1 = IfcModel.parse(CLASSIFIED.replace(b"'IFC4'", b"'IFC4X1'"))
        self.assertCode("unsupported-schema", ifc4x1.property_sets, 3)


if __name__ == "__main__":
    unittest.main()
