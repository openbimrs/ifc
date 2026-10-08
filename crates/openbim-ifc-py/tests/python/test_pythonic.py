"""The Pythonic layer (#332): entities as objects, plain values, iteration,
property sets as mappings and the pandas export. Typed: test_typing.py
checks this file, the docs snippets in it included, under ``mypy --strict``.
Run by scripts/check-python.sh."""

import contextlib
import io
import os
import sys
import unittest
from types import MappingProxyType
from typing import Any, Iterator, Optional

import openbim_ifc
from openbim_ifc import (
    Assignable,
    Binary,
    Bool,
    Derived,
    Entity,
    Enum,
    IfcError,
    IfcModel,
    Integer,
    List,
    Null,
    Property,
    Real,
    Ref,
    Text,
    Typed,
    Unknown,
)

FIXTURES = os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "test", "fixtures")
PROPERTIES = os.path.join(FIXTURES, "synthetic-properties", "synthetic_properties.ifc")

# One entity per value form the plain conversion handles.
FORMS = b"""ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#2=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#3=IFCPROPERTYSINGLEVALUE('P',$,IFCLOGICAL(.U.),$);
#4=IFCCARTESIANPOINT((1.5,-2.));
#5=IFCPIXELTEXTURE($,$,$,$,$,1,1,1,("0A1"));
#6=IFCPROPERTYSINGLEVALUE('Q',$,IFCINTEGER(9007199254740993),$);
#7=IFCRELDEFINESBYPROPERTIES('1YvctVUKr0kugbFTf53O9L',$,$,$,(#1),#8);
#8=IFCPROPERTYSET('2YvctVUKr0kugbFTf53O9L',$,'Custom',$,(#3,#6));
ENDSEC;
END-ISO-10303-21;
"""

REQUIRED = bool(os.environ.get("OPENBIM_IFC_REQUIRE_EXTRAS"))


def fixture() -> IfcModel:
    return openbim_ifc.open(PROPERTIES)


def code(error: BaseException) -> Optional[str]:
    return getattr(error, "code", None)


class Entities(unittest.TestCase):
    def test_documented_read(self) -> None:
        path = PROPERTIES
        printed = io.StringIO()
        with contextlib.redirect_stdout(printed):
            # docs:snippet py-entities
            import openbim_ifc

            model = openbim_ifc.open(path)  # IfcModel.open(path)

            for wall in model.by_type("IfcWall"):  # subtypes included
                print(wall.id, wall.type, wall.Name, wall.GlobalId)

            wall = model[31]  # or model.by_id(31); 31 in model is True
            history = wall.OwnerHistory  # a reference: Entity #5
            placement = wall.ObjectPlacement  # $: None
            exact = wall.raw("Name")  # Text('Wall B'), the lossless value
            assert wall.is_a("IfcBuildingElement") and not wall.is_a("IfcSlab")
            # docs:end
        self.assertEqual(
            printed.getvalue(),
            "30 IFCWALL Wall A 31t64Uzj98DhdKplMxBpF5\n31 IFCWALL Wall B 2CazjTQP11iu6o3c47EEd8\n",
        )
        self.assertEqual(history, model.by_id(5))
        self.assertIsNone(placement)
        self.assertEqual(exact, Text("Wall B"))

    def test_views_compare_by_model_and_id(self) -> None:
        model = fixture()
        wall = model.by_id(31)
        self.assertEqual(wall, model[31])
        self.assertEqual(hash(wall), hash(model[31]))
        self.assertEqual(len({wall, model[31], model[30]}), 2)
        self.assertNotEqual(wall, fixture()[31], "another model's #31")
        self.assertNotEqual(wall, 31)
        self.assertEqual((wall.id, wall.type, wall.model), (31, "IFCWALL", model))
        self.assertEqual(repr(wall), "<Entity #31=IFCWALL>")

    def test_names_match_case_insensitively_and_unknown_names_raise_attribute_error(self) -> None:
        wall = fixture()[31]
        self.assertEqual(wall.name, "Wall B")
        self.assertEqual(wall.get("GLOBALID"), "2CazjTQP11iu6o3c47EEd8")
        self.assertIn("ObjectPlacement", dir(wall))
        self.assertFalse(hasattr(wall, "Nmae"))
        with self.assertRaises(AttributeError) as caught:
            wall.IsDefinedBy  # INVERSE: no slot
        self.assertEqual(code(caught.exception.__cause__ or caught.exception), "unknown-attribute")
        with self.assertRaises(IfcError) as refused:
            wall.get("Nmae")
        self.assertEqual(refused.exception.code, "unknown-attribute")

    def test_missing_entities(self) -> None:
        model = fixture()
        with self.assertRaises(KeyError):
            model[999]
        with self.assertRaises(IfcError) as caught:
            model.by_id(999)
        self.assertEqual(caught.exception.code, "missing-entity")
        ghost = Entity(model, 999)
        self.assertEqual(repr(ghost), "<Entity #999 (missing)>")
        with self.assertRaises(IfcError):
            ghost.Name

    def test_subtypes(self) -> None:
        model = fixture()
        wall = model[31]
        self.assertTrue(wall.is_a("IfcWall"))
        self.assertTrue(wall.is_a("ifcproduct"))
        self.assertTrue(wall.is_a("IfcRoot"))
        self.assertTrue(model[30].is_a("IfcProduct"), "answered from the cache")
        self.assertFalse(wall.is_a("IfcSlab"))
        self.assertFalse(wall.is_a("IfcWallType"))
        self.assertFalse(wall.is_a("IfcNoSuchType"))
        self.assertFalse(model[29].is_a("IfcProduct"))


class PlainValues(unittest.TestCase):
    def test_every_form_converts_and_raw_keeps_it_exact(self) -> None:
        model = IfcModel.parse(FORMS)
        wall, unit, prop, point, texture, big = (model[i] for i in (1, 2, 3, 4, 5, 6))
        self.assertEqual(wall.PredefinedType, "STANDARD")
        self.assertEqual(wall.raw("PredefinedType"), Enum("STANDARD"))
        self.assertIsNone(wall.Description)
        self.assertEqual(wall.raw("Description"), Null())
        self.assertIsNone(unit.Dimensions)
        self.assertEqual(unit.raw("Dimensions"), Derived())
        self.assertIsNone(prop.NominalValue)
        self.assertEqual(prop.raw("NominalValue"), Typed("IFCLOGICAL", Unknown()))
        self.assertEqual(point.Coordinates, (1.5, -2.0))
        self.assertEqual(point.raw("Coordinates"), List((Real(1.5), Real(-2.0))))
        self.assertEqual(texture.Pixel, (Binary("0A1"),))
        self.assertEqual(texture.Width, 1)
        self.assertIsInstance(texture.Width, int)
        self.assertEqual(big.NominalValue, 9007199254740993)
        self.assertEqual(model[7].RelatedObjects, (wall,))
        self.assertEqual(model[7].RelatingPropertyDefinition, model[8])


class Writes(unittest.TestCase):
    def test_documented_write(self) -> None:
        model = fixture()
        # docs:snippet py-entity-write
        from openbim_ifc import Text

        wall = model[31]
        wall.Name = "Wall B (checked)"  # IfcLabel: written 'Wall B (checked)'
        wall.PredefinedType = "standard"  # IfcWallTypeEnum: written .STANDARD.
        wall.Description = None  # $
        wall.OwnerHistory = model[5]  # a reference, checked to be an IfcOwnerHistory
        old = wall.set("Name", Text("Wall B"))  # exact; returns the old tagged value
        # docs:end
        self.assertEqual(old, Text("Wall B (checked)"))
        self.assertEqual(wall.raw("PredefinedType"), Enum("STANDARD"))
        self.assertEqual(wall.raw("Description"), Null())
        self.assertEqual(wall.raw("OwnerHistory"), Ref(5))

    def test_plain_values_follow_the_declared_type(self) -> None:
        model = IfcModel.parse(FORMS)
        wall, prop, point, texture = model[1], model[3], model[4], model[5]
        wall.Name = "W"
        self.assertEqual(wall.raw("Name"), Text("W"))
        self.assertIn(b"IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',", model.write())
        wall.PredefinedType = "Partitioning"
        self.assertEqual(wall.raw("PredefinedType"), Enum("PARTITIONING"))
        point.Coordinates = [1, 2.5]  # LIST OF IfcLengthMeasure: reals
        self.assertEqual(point.raw("Coordinates"), List((Real(1.0), Real(2.5))))
        texture.Width = 4  # IfcInteger
        self.assertEqual(texture.raw("Width"), Integer(4))
        texture.RepeatS = True  # IfcBoolean
        self.assertEqual(texture.raw("RepeatS"), Bool(True))
        # A tagged value is written exactly as given, a SELECT included.
        prop.NominalValue = Typed("IFCLABEL", Text("x"))
        self.assertEqual(prop.raw("NominalValue"), Typed("IFCLABEL", Text("x")))
        model[7].RelatingPropertyDefinition = model[8]  # an entity in a SELECT
        self.assertEqual(model[7].raw("RelatingPropertyDefinition"), Ref(8))

    def test_refusals_change_nothing(self) -> None:
        model = fixture()
        wall = model[31]
        before = model.write()
        cases: list[tuple[Assignable, str]] = [
            (1, "type-mismatch"),  # a number is no label
            (True, "type-mismatch"),
            (["x"], "type-mismatch"),  # nor a list
        ]
        for bare, expected in cases:
            with self.assertRaises(IfcError, msg=repr(bare)) as caught:
                wall.Name = bare
            self.assertEqual(caught.exception.code, expected)
        with self.assertRaises(IfcError) as caught:
            wall.PredefinedType = "CURVED"
        self.assertEqual(caught.exception.code, "type-mismatch")
        self.assertIn("IfcWallTypeEnum", str(caught.exception))
        with self.assertRaises(IfcError) as caught:
            wall.OwnerHistory = model[30]  # a wall is no IfcOwnerHistory
        self.assertEqual(caught.exception.code, "type-mismatch")
        with self.assertRaises(TypeError):
            wall.Name = b"x"  # type: ignore[assignment]
        with self.assertRaises(ValueError):
            wall.OwnerHistory = fixture()[5]
        with self.assertRaises(AttributeError) as unknown:
            wall.Nmae = Text("x")
        self.assertEqual(code(unknown.exception.__cause__ or unknown.exception), "unknown-attribute")
        with self.assertRaises(AttributeError):
            wall.id = 3  # type: ignore[misc]
        with self.assertRaises(AttributeError):
            del wall.Name
        derived = IfcModel.parse(FORMS)[2]
        with self.assertRaises(IfcError) as caught:
            derived.Dimensions = None
        self.assertEqual(caught.exception.code, "derived-attribute")
        self.assertEqual(model.write(), before)

    def test_an_ambiguous_select_asks_for_a_wrapper(self) -> None:
        model = IfcModel.parse(FORMS)
        prop = model[3]  # NominalValue: IfcValue
        with self.assertRaises(IfcError) as caught:
            prop.NominalValue = "x"
        self.assertEqual(caught.exception.code, "ambiguous-value")
        for member in ("IfcLabel", "IfcText", "IfcIdentifier"):
            self.assertIn(member, str(caught.exception))
        self.assertEqual(prop.raw("NominalValue"), Typed("IFCLOGICAL", Unknown()), "unchanged")

    def test_aggregates_write_as_lists(self) -> None:
        model = IfcModel.parse(FORMS)
        point = model[4]
        point.Coordinates = (Real(0.0), Real(1.0), Real(2.0))
        self.assertEqual(point.raw("Coordinates"), List((Real(0.0), Real(1.0), Real(2.0))))
        relation = model[7]
        # An Entity is checked against SET OF IfcObjectDefinition; an
        # exact Ref is written as given.
        relation.RelatedObjects = [model[1], Ref(4)]
        self.assertEqual(relation.raw("RelatedObjects"), List((Ref(1), Ref(4))))
        with self.assertRaises(IfcError) as caught:
            relation.RelatedObjects = [model[1], model[4]]
        self.assertEqual(caught.exception.code, "type-mismatch")


class Iteration(unittest.TestCase):
    def test_documented_iteration(self) -> None:
        model = fixture()
        # docs:snippet py-iteration
        count = len(model)  # entities
        present = 31 in model  # True; an Entity of this model works too
        every = list(model)  # Entity views, in file order
        walls = model.by_type("IfcBuildingElement")  # IfcWall #30, #31
        exact = model.by_type("IfcBuildingElement", include_subtypes=False)  # []
        # docs:end
        self.assertEqual(count, 68)
        self.assertTrue(present)
        self.assertEqual([e.id for e in every], model.ids())
        self.assertEqual([w.id for w in walls], [30, 31])
        self.assertEqual(exact, [])

    def test_membership(self) -> None:
        model = fixture()
        self.assertIn(model[31], model)
        self.assertNotIn(fixture()[31], model)
        for absent in (999, -1, 2**70, "31", True, 31.0, None):
            self.assertNotIn(absent, model)
        self.assertEqual(model.by_type("IfcNoSuchType"), [])
        iterator: Iterator[Entity] = iter(model)
        self.assertEqual(next(iterator), model[1])


class PropertySets(unittest.TestCase):
    def test_documented_psets(self) -> None:
        model = fixture()
        # docs:snippet py-psets
        from openbim_ifc import Text, Typed

        wall = model[30]
        common = wall.psets["Pset_WallCommon"]
        external = common["IsExternal"]  # False: the wall's own value
        rating = common["FireRating"]  # 'F30', inherited from its type
        width = wall.qtos["Qto_WallBaseQuantities"]["Width"]  # 200.0, in its stated unit

        wall.set_property("Pset_WallCommon", "FireRating", Typed("IFCLABEL", Text("F60")))
        assert wall.psets["Pset_WallCommon"]["FireRating"] == "F60"  # a fresh snapshot
        # docs:end
        self.assertIs(external, False)
        self.assertEqual(rating, "F30")
        self.assertEqual(width, 200.0)
        self.assertEqual(model[31].psets["Pset_WallCommon"]["FireRating"], "F30", "type unchanged")

    def test_every_property_form(self) -> None:
        model = fixture()
        families = model[31].psets["Pset_Families"]
        self.assertEqual(families["Thickness"], 0.2)
        self.assertEqual(families["Colour"], ("red",))
        self.assertEqual(families["Layers"], (0.012, 0.15, 0.012))
        self.assertEqual(families["Material"], model[46])
        self.assertEqual(families["Assembly"], {"Core": 0.15, "Finish": "paint"})
        bounded = families["Range"]
        assert isinstance(bounded, Property)
        self.assertEqual(bounded.bounds.upper if bounded.bounds else None, Typed("IFCLENGTHMEASURE", Real(10.0)))
        self.assertIsInstance(families["Curve"], Property)
        self.assertNotIn("Pset_Families", model[31].qtos)
        self.assertEqual(model[30].qtos["Qto_WallBaseQuantities"]["Layers"], {"Inner": 12.0, "Outer": 12.0})
        self.assertEqual(dict(model[29].psets["Pset_WallCommon"]), {"IsExternal": True, "FireRating": "F30"})
        self.assertEqual(IfcModel.parse(FORMS)[1].psets["Custom"], {"P": None, "Q": 9007199254740993})

    def test_mappings_are_read_only_and_writes_go_through_set_property(self) -> None:
        model = fixture()
        wall = model[31]
        psets: Any = wall.psets
        self.assertIsInstance(psets, MappingProxyType)
        with self.assertRaises(TypeError):
            psets["Pset_WallCommon"] = {}
        with self.assertRaises(TypeError):
            psets["Pset_WallCommon"]["IsExternal"] = False
        holder = wall.set_property("Custom", "Note", Typed("IFCLABEL", Text("x")))
        self.assertEqual(model.type_of(holder), "IFCPROPERTYSINGLEVALUE")
        self.assertEqual(wall.psets["Custom"]["Note"], "x")
        wall.remove_property("Custom", "Note")
        self.assertNotIn("Custom", wall.psets)
        with self.assertRaises(TypeError):
            wall.set_property("Custom", "Note", "x")  # type: ignore[arg-type]
        self.assertEqual(wall.property_sets(), model.property_sets(31))
        with self.assertRaises(IfcError) as caught:
            model[5].psets
        self.assertEqual(caught.exception.code, "wrong-entity-type")


def pandas_or_skip(case: unittest.TestCase) -> None:
    try:
        import pandas  # noqa: F401
    except ImportError:
        if REQUIRED:
            raise
        case.skipTest("pandas is not installed (the openbim-ifc[pandas] extra)")


class DataFrames(unittest.TestCase):
    def test_documented_dataframe(self) -> None:
        pandas_or_skip(self)
        model = fixture()
        # docs:snippet py-dataframe
        frame = model.to_dataframe("IfcWall")  # needs openbim-ifc[pandas]
        # index: id; columns: type, GlobalId, Name, then "Set.Property"
        external = frame["Pset_WallCommon.IsExternal"]  # #30 False, #31 True
        # docs:end
        self.assertEqual(list(frame.index), [30, 31])
        self.assertEqual(frame.index.name, "id")
        self.assertEqual(list(external), [False, True])
        self.assertEqual(
            list(frame.columns[:5]),
            ["type", "GlobalId", "Name", "Pset_WallCommon.IsExternal", "Pset_WallCommon.FireRating"],
        )
        self.assertEqual(frame.loc[30, "Qto_WallBaseQuantities.Width"], 200.0)
        self.assertTrue(frame.isna().loc[31, "Qto_WallBaseQuantities.Width"])
        self.assertEqual(frame.loc[31, "Pset_WallCommon.FireRating"], "F30")
        self.assertEqual(frame.loc[31, "Pset_Families.Material"], model[46])
        self.assertEqual(frame.loc[30, "Name"], "Wall A")

    def test_options(self) -> None:
        pandas_or_skip(self)
        model = fixture()
        plain = model.to_dataframe("IfcProduct", psets=False, qtos=False, attributes=("Name",))
        self.assertEqual(list(plain.columns), ["type", "Name"])
        self.assertEqual(list(plain["type"]), ["IFCSITE", "IFCBUILDING", "IFCBUILDINGSTOREY", "IFCWALL", "IFCWALL"])
        quantities = model.to_dataframe("IfcWall", psets=False)
        self.assertNotIn("Pset_WallCommon.IsExternal", quantities.columns)
        self.assertIn("Qto_WallBaseQuantities.Width", quantities.columns)
        empty = model.to_dataframe("IfcSlab")
        self.assertEqual((len(empty), list(empty.columns)), (0, ["type", "GlobalId", "Name"]))
        self.assertEqual(len(model.to_dataframe("IfcBuildingElement", include_subtypes=False)), 0)
        with self.assertRaises(ValueError):
            model.to_dataframe(attributes=("Name", "Name"))

    def test_a_missing_pandas_is_a_clear_error(self) -> None:
        saved = sys.modules.get("pandas")
        sys.modules["pandas"] = None  # type: ignore[assignment]
        try:
            with self.assertRaises(ImportError) as caught:
                fixture().to_dataframe()
        finally:
            if saved is None:
                del sys.modules["pandas"]
            else:
                sys.modules["pandas"] = saved
        self.assertIn("openbim-ifc[pandas]", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
