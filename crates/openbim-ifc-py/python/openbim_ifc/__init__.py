"""Read, edit and write IFC STEP files.

Python bindings for the ``openbim-ifc`` Rust crates (ADR 0013). An
:class:`IfcModel` holds entities, each a type name plus positional
attributes. Attribute values are the frozen dataclasses in
:mod:`openbim_ifc.values`, which keep every distinction IFC makes -- ``$``
from ``*``, ``.U.`` from ``.F.``, integer from real, and typed wrappers such
as ``IFCLENGTHMEASURE(2.5)`` from their payload -- so a file read and
written back through Python is unchanged.

An :class:`Entity` view reads and writes attributes by name with plain
Python values (``wall.Name``), and ``wall.psets`` maps property sets
(:mod:`openbim_ifc.entity`).

>>> import openbim_ifc
>>> model = openbim_ifc.open("model.ifc")  # doctest: +SKIP
>>> walls = model.by_type("IfcWall")  # doctest: +SKIP
"""

import os as _os
from typing import Optional as _Optional, Union as _Union

from ._native import (
    SELECTOR_GRAMMAR_VERSION,
    IfcError,
    canonical_selector as _canonical_selector,
    convert_unit as _convert_unit,
    format_length as _format_length,
    name_based_guid as _name_based_guid,
    new_guid as _new_guid,
)
from .geometry import (
    GEOMETRY_FORMAT,
    GEOMETRY_FORMAT_VERSION,
    GeometryRefusal,
    ProductCurves,
    ProductGeometry,
    ProductMesh,
    ProductPlacement,
    ProductPolyline,
    SelectedRepresentation,
    SurfaceColour,
)
from .domains import (
    HANDLE_BASE,
    AttributeInfo,
    InverseAttributeInfo,
    InverseReference,
    AuthorOp,
    AuthoringResult,
    KeptEntity,
    RemovalReport,
    LengthUnitReport,
    ConvertedValues,
    KeptValues,
    RemovedEntity,
    Classification,
    ClassificationSystem,
    Cost,
    CostAnomaly,
    CostItem,
    CostSchedule,
    CostValue,
    LengthUnit,
    MapConversion,
    MaterialAssignment,
    MaterialConstituent,
    MaterialLayer,
    MaterialProfile,
    MaterialRef,
    MaterialUsage,
    ObjectPropertySets,
    ProjectedCrs,
    Property,
    PropertyBounds,
    PropertyEdit,
    PropertyEditResult,
    PropertyEnumeration,
    PropertyRefusal,
    PropertySet,
    PropertyTable,
    PropertyTableRow,
    ResolvedUnit,
    SpatialAnomaly,
    SpatialDanglingReference,
    SpatialNode,
    SpatialTree,
    System,
    SystemAnomaly,
    Systems,
    UnitBasis,
    handle,
)
from .entity import Assignable, Entity, PlainValue, PropertyValue, SelectedValue
from .model import IfcModel
from .records import (
    DuplicateGlobalId,
    Header,
    ParseOptions,
    UnreachableProduct,
    ValidationFinding,
    ValidationReport,
)
from .values import (
    Binary,
    Bool,
    Derived,
    Enum,
    Integer,
    List,
    Null,
    Real,
    Ref,
    Text,
    Typed,
    Unknown,
    Value,
)



def open(
    path: _Union[str, "_os.PathLike[str]"],
    *,
    mapped: bool = False,
    options: _Optional[ParseOptions] = None,
) -> IfcModel:
    """Read a STEP (``.ifc``) file from disk: :meth:`IfcModel.open`."""
    return IfcModel.open(path, mapped=mapped, options=options)


def new_guid() -> str:
    """A new random ``GlobalId``: an RFC 9562 version-4 UUID from the
    operating system's generator, in IFC's 22-character form."""
    return _new_guid()


def name_based_guid(*parts: str) -> str:
    """The ``GlobalId`` named by ``parts`` (RFC 9562 version 8): the same
    parts give the same ``GlobalId`` in every binding and release, for a
    reproducible export. Choose parts unique together, such as a tool name
    and a stable key."""
    return _name_based_guid(list(parts))


def canonical_selector(query: str) -> str:
    """Selector ``query`` in its canonical form (#464): spacing, quoting
    and aliases normalised, so two spellings of one query compare equal.
    Raises ``invalid-selector`` (with ``span``) when it does not parse
    against grammar :data:`SELECTOR_GRAMMAR_VERSION`."""
    return _canonical_selector(query)


def convert_unit(value: float, from_unit: str, to_unit: str) -> float:
    """Convert ``value`` between two units named as text (#469): an SI unit
    with its prefix written before it (``"METRE"``, ``"MILLIMETRE"``,
    ``"MILLI METRE"``, ``"SQUARE_MILLIMETRE"``, ``"KILOGRAM"``) or a named
    unit of IFC4 Table 673 with exact factors (``"inch"``, ``"foot"``,
    ``"square foot"``, ``"gallon US"``, ``"pound"``, ``"degree"`` ...).

    Raises ``type-mismatch`` for units of different dimensions,
    ``invalid-value`` for text that names no unit, and ``unsupported`` for a
    unit with an offset (degree Fahrenheit, #110)."""
    return _convert_unit(value, from_unit, to_unit)


def format_length(
    value: float,
    unit: str,
    *,
    style: str = "decimal",
    places: _Optional[int] = None,
    display_unit: _Optional[str] = None,
    suppress_zero_inches: bool = False,
) -> str:
    """Write ``value``, a length in ``unit``, as text (#469).

    ``style`` is ``"decimal"`` (in ``display_unit``, default ``unit``, with
    ``places`` digits, default 2), ``"feet-inches"`` (``5' - 6 1/2"``,
    rounded to ``1/places`` of an inch, default 8; with
    ``suppress_zero_inches`` a whole foot is ``5'``) or ``"inches"``
    (``66 1/2"``)."""
    if places is None:
        places = 2 if style == "decimal" else 8
    return _format_length(value, unit, style, places, display_unit, suppress_zero_inches)


__all__ = [
    "open",
    "convert_unit",
    "format_length",
    "new_guid",
    "name_based_guid",
    "canonical_selector",
    "SELECTOR_GRAMMAR_VERSION",
    "SelectedValue",
    "DuplicateGlobalId",
    "IfcError",
    "IfcModel",
    "Entity",
    "PlainValue",
    "PropertyValue",
    "Assignable",
    "AttributeInfo",
    "InverseAttributeInfo",
    "InverseReference",
    "AuthorOp",
    "AuthoringResult",
    "RemovalReport",
    "RemovedEntity",
    "KeptEntity",
    "LengthUnitReport",
    "ConvertedValues",
    "KeptValues",
    "HANDLE_BASE",
    "handle",
    "ParseOptions",
    "Header",
    "ValidationReport",
    "ValidationFinding",
    "UnreachableProduct",
    "PropertySet",
    "Property",
    "PropertyEnumeration",
    "PropertyBounds",
    "PropertyTable",
    "PropertyTableRow",
    "ResolvedUnit",
    "PropertyEdit",
    "PropertyEditResult",
    "SpatialTree",
    "SpatialNode",
    "SpatialDanglingReference",
    "SpatialAnomaly",
    "Classification",
    "ClassificationSystem",
    "MaterialAssignment",
    "MaterialRef",
    "MaterialLayer",
    "MaterialProfile",
    "MaterialConstituent",
    "MaterialUsage",
    "Systems",
    "System",
    "SystemAnomaly",
    "Cost",
    "CostSchedule",
    "CostItem",
    "CostValue",
    "UnitBasis",
    "CostAnomaly",
    "MapConversion",
    "ProjectedCrs",
    "LengthUnit",
    "ProductPlacement",
    "SelectedRepresentation",
    "GeometryRefusal",
    "ProductGeometry",
    "GEOMETRY_FORMAT",
    "GEOMETRY_FORMAT_VERSION",
    "ProductMesh",
    "ProductCurves",
    "ProductPolyline",
    "SurfaceColour",
    "Value",
    "Null",
    "Derived",
    "Bool",
    "Unknown",
    "Integer",
    "Real",
    "Text",
    "Binary",
    "Enum",
    "Ref",
    "List",
    "Typed",
]
