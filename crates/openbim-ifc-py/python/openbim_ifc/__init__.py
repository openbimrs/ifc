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

from ._native import IfcError
from .geometry import (
    GEOMETRY_FORMAT,
    GEOMETRY_FORMAT_VERSION,
    GeometryRefusal,
    ProductGeometry,
    ProductMesh,
    ProductPlacement,
    SelectedRepresentation,
)
from .domains import (
    HANDLE_BASE,
    AttributeInfo,
    AuthorOp,
    AuthoringResult,
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
from .entity import Assignable, Entity, PlainValue, PropertyValue
from .model import IfcModel
from .records import (
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


__all__ = [
    "open",
    "IfcError",
    "IfcModel",
    "Entity",
    "PlainValue",
    "PropertyValue",
    "Assignable",
    "AttributeInfo",
    "AuthorOp",
    "AuthoringResult",
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
