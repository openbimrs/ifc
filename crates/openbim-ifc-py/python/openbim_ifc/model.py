"""The public model class."""

from __future__ import annotations

import os
from typing import Iterable, List, Optional, Tuple, Union

from ._native import NativeModel
from . import domains
from .domains import (
    Classification,
    Cost,
    MapConversion,
    MaterialAssignment,
    PropertySet,
    ResolvedUnit,
    SpatialTree,
    Systems,
)
from .records import Header, ParseOptions, UnreachableProduct, ValidationReport
from .values import Value, from_wire, to_wire


class IfcModel:
    """An IFC model: entities keyed by their ``#id``, in file order.

    Failures raise :class:`openbim_ifc.IfcError`, whose ``code`` is one of
    ``parse``, ``write``, ``missing-entity``, ``invalid-value``,
    ``out-of-range``, ``unsupported-schema``, ``io``,
    ``unsupported-profile``, ``feature-disabled``, or, from a domain view,
    ``invalid-model``, ``missing-reference``, ``budget-exceeded``,
    ``unsupported`` or ``wrong-entity-type``.

    A parsed model decodes each entity the first time it is read: parsing
    checks every record but builds nothing, so opening a large file is fast
    and memory holds the file plus what has been touched.
    """

    __slots__ = ("_native",)

    def __init__(self) -> None:
        """An empty model."""
        self._native = NativeModel()

    @classmethod
    def parse(cls, data: bytes, *, options: Optional[ParseOptions] = None) -> "IfcModel":
        """Parse a STEP (``.ifc``) file from its bytes.

        ``options`` relaxes the strict read, e.g. ``ParseOptions.lenient()``
        skips damaged records; each one skipped is listed by
        :meth:`diagnostics`.
        """
        model = cls.__new__(cls)
        model._native = NativeModel.parse(bytes(data), **(options or ParseOptions())._keywords())
        return model

    @classmethod
    def parse_ifcxml(cls, data: bytes, *, xsd_profile: Optional[str] = None) -> "IfcModel":
        """Parse an ifcXML document.

        Without ``xsd_profile`` it reads this library's lossless layout; with
        ``"IFC4"`` or ``"IFC4X3_ADD2"`` it reads the buildingSMART XSD layout
        of that release.
        """
        model = cls.__new__(cls)
        model._native = NativeModel.parse_ifcxml(bytes(data), xsd_profile)
        return model

    @classmethod
    def open(
        cls,
        path: Union[str, "os.PathLike[str]"],
        *,
        mapped: bool = False,
        options: Optional[ParseOptions] = None,
    ) -> "IfcModel":
        """Read a STEP (``.ifc``) file from disk.

        Cheaper than ``IfcModel.parse(Path(path).read_bytes())``: the file is
        read once, straight into the model, with the GIL released.

        ``mapped=True`` memory-maps the file instead of reading it: nothing
        is copied, and the pages belong to the OS page cache. The model keeps
        decoding from the file while it is alive, so **the file must not be
        modified or truncated until the model is gone** -- a truncated file
        can terminate the interpreter (``SIGBUS``), and a rewritten one makes
        reads fail or return other content. Use it only for files that stay
        put, such as a read-only export.

        ``options`` is as for :meth:`parse`.

        Raises :class:`openbim_ifc.IfcError` with ``code == "io"`` when the
        file cannot be read.
        """
        model = cls.__new__(cls)
        model._native = NativeModel.open(path, mapped, **(options or ParseOptions())._keywords())
        return model

    def write(self) -> bytes:
        """Serialize as STEP bytes."""
        return self._native.write()

    def write_ifcxml(self, *, xsd_profile: Optional[str] = None) -> bytes:
        """Serialize as ifcXML bytes, in the layout ``parse_ifcxml`` reads.

        An XSD-layout write needs the header to declare the profile's schema
        and refuses with ``write`` what the layout cannot carry.
        """
        return self._native.write_ifcxml(xsd_profile)

    @property
    def header(self) -> Header:
        """The STEP file header: description, name, time stamp, author, ..."""
        return Header._from_wire(self._native.header())

    def set_header(self, header: Header) -> None:
        """Replace the STEP file header, e.g. with ``dataclasses.replace``."""
        if not isinstance(header, Header):
            raise TypeError(f"expected an openbim_ifc.Header, got {type(header).__name__}")
        self._native.set_header(header._to_wire())

    def validate(self, max_findings: Optional[int] = None) -> ValidationReport:
        """Validate against the schema the header declares.

        Findings are sorted by severity, rule, entity and slot.
        ``max_findings`` caps the report (default 10,000) and sets
        ``truncated`` when reached.
        """
        return ValidationReport._from_wire(self._native.validate(max_findings))

    def unreachable_products(self) -> List[UnreachableProduct]:
        """Products no viewer will draw, with a stable ``reason``, in id order."""
        return [UnreachableProduct(**row) for row in self._native.unreachable_products()]

    def property_sets(self, id: int) -> List[PropertySet]:
        """The property sets, quantity sets and predefined property sets of
        object ``id``: its own first, then those its type object holds, an
        occurrence property overriding an inherited one of the same name.

        Values keep their declared IFC type, e.g.
        ``Typed("IFCLENGTHMEASURE", Real(0.2))``. Resolved against the
        release the header declares (IFC2X3, IFC4 or IFC4X3).
        """
        return list(domains._from_wire(self._native.property_sets(id)))

    def resolve_unit(self, measure_type: str, unit: Optional[int] = None) -> ResolvedUnit:
        """The effective unit of a ``measure_type`` value
        (``"IFCAREAMEASURE"``): ``unit`` when given (a property's stated
        unit), otherwise the project default, resolved exactly to SI."""
        return domains._from_wire(self._native.resolve_unit(measure_type, unit))

    def spatial_tree(self) -> SpatialTree:
        """The spatial containment tree: every container with its parent,
        sub-containers and contained elements."""
        return domains._from_wire(self._native.spatial_tree())

    def classifications(self, id: int) -> List[Classification]:
        """The classifications of object ``id``: its own, then its type's."""
        return list(domains._from_wire(self._native.classifications(id)))

    def material(self, id: int) -> Optional[MaterialAssignment]:
        """The material association of object ``id``, its own or its
        type's, or ``None``."""
        wire = self._native.material(id)
        return None if wire is None else domains._from_wire(wire)

    def systems(self) -> Systems:
        """Every system with its members and served structures, and the
        memberships the reader could not honour."""
        return domains._from_wire(self._native.systems())

    def cost(self) -> Cost:
        """Every cost schedule and cost item; values as authored, typed."""
        return domains._from_wire(self._native.cost())

    def georeferencing(self) -> List[MapConversion]:
        """Every coordinate operation resolved with the project length
        unit; empty when the model has none."""
        return list(domains._from_wire(self._native.georeferencing()))

    def __len__(self) -> int:
        return len(self._native)

    @property
    def schema(self) -> Optional[str]:
        """The first ``FILE_SCHEMA`` token, e.g. ``"IFC4"``, or ``None``."""
        return self._native.schema

    def diagnostics(self) -> List[str]:
        """Non-fatal problems found while reading."""
        return self._native.diagnostics()

    def ids(self) -> List[int]:
        """Every entity id, in file order."""
        return self._native.ids()

    def ids_of_type(self, type_name: str) -> List[int]:
        """Ids of every entity of exactly ``type_name`` (case-insensitive)."""
        return self._native.ids_of_type(type_name)

    def ids_of_type_including_subtypes(self, type_name: str) -> List[int]:
        """Ids of ``type_name`` or any subtype, per the file's schema."""
        return self._native.ids_of_type_including_subtypes(type_name)

    def type_of(self, id: int) -> str:
        """The upper-case type name of entity ``id``."""
        return self._native.type_of(id)

    def attributes(self, id: int) -> List[Value]:
        """Every attribute of entity ``id``, in declaration order."""
        return [from_wire(value) for value in self._native.attributes(id)]

    def attribute(self, id: int, index: int) -> Value:
        """Attribute ``index`` of entity ``id``; ``Null`` past the end."""
        return from_wire(self._native.attribute(id, index))

    def set_attribute(self, id: int, index: int, value: Value) -> Value:
        """Set attribute ``index`` of entity ``id``; returns the old value."""
        return from_wire(self._native.set_attribute(id, index, to_wire(value)))

    def add(self, type_name: str, attributes: Iterable[Value]) -> int:
        """Append an entity; returns its new id."""
        return self._native.add(type_name, [to_wire(value) for value in attributes])

    def remove(self, id: int) -> None:
        """Remove entity ``id``, leaving references to it dangling."""
        self._native.remove(id)

    def dangling_references(self) -> List[Tuple[int, int]]:
        """Every ``(from, to)`` pair where ``to`` does not exist."""
        return self._native.dangling_references()

    def __repr__(self) -> str:
        return f"<IfcModel schema={self.schema!r} entities={len(self)}>"
