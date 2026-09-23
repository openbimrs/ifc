"""Read, edit and write IFC STEP files.

Python bindings for the ``openbim-ifc`` Rust crates (ADR 0013). An
:class:`IfcModel` holds entities, each a type name plus positional
attributes. Attribute values are the frozen dataclasses in
:mod:`openbim_ifc.values`, which keep every distinction IFC makes -- ``$``
from ``*``, ``.U.`` from ``.F.``, integer from real, and typed wrappers such
as ``IFCLENGTHMEASURE(2.5)`` from their payload -- so a file read and
written back through Python is unchanged.

>>> from openbim_ifc import IfcModel
>>> model = IfcModel.parse(open("model.ifc", "rb").read())  # doctest: +SKIP
"""

from ._native import IfcError
from .model import IfcModel
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

__all__ = [
    "IfcError",
    "IfcModel",
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
