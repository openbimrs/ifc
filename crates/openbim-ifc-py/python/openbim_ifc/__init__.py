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

from ._native import IfcError, name_based_guid as _name_based_guid, new_guid as _new_guid
from .geometry import (
    GEOMETRY_FORMAT,
    GEOMETRY_FORMAT_VERSION,
    GeometryRefusal,
    ProductGeometry,
    ProductMesh,
    ProductPlacement,
    SelectedRepresentation,
    SurfaceColour,
)
from .domains import (
    HANDLE_BASE,
    AttributeInfo,
    AuthorOp,
    AuthoringResult,
    KeptEntity,
    RemovalReport,
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
from .entity import Assignable, Entity, PlainValue, PropertyValue
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


__all__ = [
    "open",
    "new_guid",
    "name_based_guid",
    "DuplicateGlobalId",
    "IfcError",
    "IfcModel",
    "Entity",
    "PlainValue",
    "PropertyValue",
    "Assignable",
    "AttributeInfo",
    "AuthorOp",
    "AuthoringResult",
    "RemovalReport",
    "RemovedEntity",
    "KeptEntity",
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
