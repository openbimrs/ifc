"""Entities by GlobalId, new GlobalIds, and an object's type, a type's
occurrences, openings and fillings (#463), over the IFC2X3, IFC4 and IFC4X3
relation fixtures. The answers themselves are pinned in
openbim-ifc-binding-core; this checks how they cross into Python."""

import os
import unittest

import openbim_ifc
from openbim_ifc import (
    DuplicateGlobalId,
    Entity,
    IfcError,
    IfcModel,
    InverseAttributeInfo,
    InverseReference,
    List,
    Ref,
)

FIXTURES = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures", "synthetic-relations"
)
RELEASES = ("relations_ifc2x3.ifc", "relations_ifc4.ifc", "relations_ifc4x3.ifc")


def fixture(name: str) -> IfcModel:
    return openbim_ifc.open(os.path.join(FIXTURES, name))


class GlobalIds(unittest.TestCase):
    def test_every_release_finds_an_entity_by_its_global_id(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                wall = model[17]
                found = model.by_guid(wall.GlobalId)
                self.assertIsInstance(found, Entity)
                self.assertEqual(found, wall)
                self.assertEqual(model.id_by_guid(wall.GlobalId), 17)
                self.assertIsNone(model.by_guid("3MtNd0Uaz8Ix2OPtRq_3Kk"))
                self.assertEqual(model.duplicate_global_ids(), [])

    def test_text_that_is_not_a_global_id_is_refused(self) -> None:
        model = fixture("relations_ifc4.ifc")
        for text in ("", "short", "4000000000000000000000"):
            with self.assertRaises(IfcError) as raised:
                model.by_guid(text)
            self.assertEqual(raised.exception.code, "invalid-value")

    def test_a_copied_global_id_is_refused_and_listed(self) -> None:
        model = fixture("relations_ifc4.ifc")
        guid = model[17].GlobalId
        self.assertEqual(model.id_by_guid(guid), 17)
        copy = model.add(model.type_of(17), model.attributes(17))
        with self.assertRaises(IfcError) as raised:
            model.by_guid(guid)
        self.assertEqual(raised.exception.code, "invalid-model")
        self.assertIn(f"#17, #{copy}", str(raised.exception))
        self.assertEqual(model.duplicate_global_ids(), [DuplicateGlobalId(guid, (17, copy))])

    def test_new_global_ids_are_valid_unique_and_parse_back(self) -> None:
        model = fixture("relations_ifc4.ifc")
        drawn = {openbim_ifc.new_guid() for _ in range(1000)}
        self.assertEqual(len(drawn), 1000)
        for guid in list(drawn)[:20]:
            self.assertEqual(len(guid), 22)
            self.assertIsNone(model.by_guid(guid))  # parses: refused otherwise

    def test_name_based_global_ids_are_reproducible(self) -> None:
        one = openbim_ifc.name_based_guid("my-exporter", "wall", "42")
        self.assertEqual(one, openbim_ifc.name_based_guid("my-exporter", "wall", "42"))
        self.assertNotEqual(one, openbim_ifc.name_based_guid("my-exporter", "wall", "43"))
        # Pinned in the facade: every binding derives the same GlobalId.
        self.assertEqual(
            openbim_ifc.name_based_guid("openbim-ifc", "example"), "347eFU5Y69bPY9V9tumQuV"
        )


class Relations(unittest.TestCase):
    def test_types_openings_and_fillings_in_every_release(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                self.assertEqual(model.type_object(17), 16)
                self.assertIsNone(model.type_object(19))
                self.assertEqual(model.occurrences_of(16), [17, 18])
                self.assertEqual(model.type_object(28), 27)
                self.assertEqual(model.openings_of(17), [21, 23])
                self.assertEqual(model.fillings_of(21), [28])
                self.assertEqual(model.fillings_of(23), [])
                self.assertEqual(model.voided_element_of(22), 18)

    def test_refusals_carry_the_shared_codes(self) -> None:
        model = fixture("relations_ifc2x3.ifc")
        for call, code in (
            (lambda: model.type_object(999), "missing-entity"),
            (lambda: model.type_object(8), "wrong-entity-type"),
            (lambda: model.occurrences_of(17), "wrong-entity-type"),
            (lambda: model.voided_element_of(17), "wrong-entity-type"),
        ):
            with self.assertRaises(IfcError) as raised:
                call()
            self.assertEqual(raised.exception.code, code)


class Inverses(unittest.TestCase):
    """INVERSE attributes by name (#462): how the answers cross into Python."""

    def test_an_inverse_name_reads_entities_through_the_declared_release(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                wall = model[17]
                if name == "relations_ifc2x3.ifc":
                    self.assertEqual(wall.IsDefinedBy, (model[20], model[36]))
                    self.assertFalse(hasattr(wall, "IsTypedBy"))
                else:
                    self.assertEqual(wall.IsDefinedBy, (model[36],))
                    self.assertEqual(wall.istypedby, (model[20],))
                self.assertEqual(wall.ContainedInStructure, (model[33],))
                self.assertEqual(model[11].Decomposes, ())
                self.assertEqual(model[21].VoidsElements, model[24])
                self.assertEqual(model.attribute_by_name(17, "HasOpenings"), List([Ref(24), Ref(25)]))
                with self.assertRaises(IfcError) as refused:
                    wall.HasOpenings = ()
                self.assertEqual(refused.exception.code, "inverse-attribute")

    def test_references_totals_and_declarations(self) -> None:
        for name in RELEASES:
            with self.subTest(name):
                model = fixture(name)
                self.assertEqual(
                    model.inverse_references(21),
                    [InverseReference(24, 5), InverseReference(30, 4)],
                )
                self.assertEqual(model[21].inverse_references(), [(model[24], 5), (model[30], 4)])
                self.assertEqual(model.total_inverses(17), 5)
                self.assertEqual(model[36].total_inverses(), 0)
                voids = next(i for i in model.inverse_attribute_names(21) if i.name == "VoidsElements")
                self.assertIsInstance(voids, InverseAttributeInfo)
                self.assertEqual(
                    voids,
                    InverseAttributeInfo(
                        "VoidsElements",
                        "IfcRelVoidsElement",
                        "RelatedOpeningElement",
                        False,
                        1,
                        1,
                        "IfcFeatureElementSubtraction",
                    ),
                )
                defined = next(i for i in model[17].inverse_attribute_names() if i.name == "IsDefinedBy")
                self.assertIsNone(defined.upper)
                with self.assertRaises(IfcError) as missing:
                    model.total_inverses(999)
                self.assertEqual(missing.exception.code, "missing-entity")


if __name__ == "__main__":
    unittest.main()
