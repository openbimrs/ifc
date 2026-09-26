"""The public model class."""

from __future__ import annotations

import os
from typing import Iterable, List, Optional, Tuple, Union

from ._native import NativeModel
from .values import Value, from_wire, to_wire


class IfcModel:
    """An IFC model: entities keyed by their ``#id``, in file order.

    Failures raise :class:`openbim_ifc.IfcError`, whose ``code`` is one of
    ``parse``, ``write``, ``missing-entity``, ``invalid-value``,
    ``out-of-range``, ``unsupported-schema`` or ``io``.

    A parsed model decodes each entity the first time it is read: parsing
    checks every record but builds nothing, so opening a large file is fast
    and memory holds the file plus what has been touched.
    """

    __slots__ = ("_native",)

    def __init__(self) -> None:
        """An empty model."""
        self._native = NativeModel()

    @classmethod
    def parse(cls, data: bytes) -> "IfcModel":
        """Parse a STEP (``.ifc``) file from its bytes."""
        model = cls.__new__(cls)
        model._native = NativeModel.parse(bytes(data))
        return model

    @classmethod
    def open(cls, path: Union[str, "os.PathLike[str]"], *, mapped: bool = False) -> "IfcModel":
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

        Raises :class:`openbim_ifc.IfcError` with ``code == "io"`` when the
        file cannot be read.
        """
        model = cls.__new__(cls)
        model._native = NativeModel.open(path, mapped)
        return model

    def write(self) -> bytes:
        """Serialize as STEP bytes."""
        return self._native.write()

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
