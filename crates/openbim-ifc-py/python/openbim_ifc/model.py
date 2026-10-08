"""The public model class."""

from __future__ import annotations

import os
from typing import Iterable, List, Mapping, Optional, Tuple, Union

from ._native import NativeModel
from . import domains, geometry
from .domains import (
    AttributeInfo,
    AuthorOp,
    AuthoringResult,
    Classification,
    Cost,
    MapConversion,
    MaterialAssignment,
    ObjectPropertySets,
    PropertyEdit,
    PropertyEditResult,
    PropertySet,
    ResolvedUnit,
    SpatialTree,
    Systems,
)
from .entity import Assignable, ModelAccess, _plain_wire
from .geometry import ProductMesh, ProductPlacement
from .records import Header, ParseOptions, UnreachableProduct, ValidationReport
from .values import Value, from_wire, to_wire


class IfcModel(ModelAccess):
    """An IFC model: entities keyed by their ``#id``, in file order.

    Failures raise :class:`openbim_ifc.IfcError`, whose ``code`` is one of
    ``parse``, ``write``, ``missing-entity``, ``invalid-value``,
    ``out-of-range``, ``unsupported-schema``, ``io``,
    ``unsupported-profile``, ``feature-disabled``, or, from a domain view,
    ``invalid-model``, ``missing-reference``, ``budget-exceeded``,
    ``unsupported`` or ``wrong-entity-type``, or, from a property edit,
    ``template-violation`` or ``missing-property``, or, from an attribute
    named rather than numbered, ``unknown-attribute`` or
    ``derived-attribute``, or, from a plain value written by name,
    ``type-mismatch`` or ``ambiguous-value``, or, from authoring,
    ``missing-attribute`` or ``still-referenced``.

    A parsed model decodes each entity the first time it is read: parsing
    checks every record but builds nothing, so opening a large file is fast
    and memory holds the file plus what has been touched.

    ``model[id]``, ``by_id``, ``by_type``, ``iter(model)`` and ``id in model``
    give :class:`openbim_ifc.Entity` views (:mod:`openbim_ifc.entity`).
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

    def property_sets_many(
        self, ids: Optional[Iterable[int]] = None
    ) -> List[ObjectPropertySets]:
        """:meth:`property_sets` of each of ``ids``, in that order, or, with
        ``None``, of every object definition (``IfcObjectDefinition`` and
        its subtypes) in file order, in one pass.

        The file's property relationships are validated once for the whole
        call rather than once per object, so resolving every object is
        linear in the model. Each :class:`ObjectPropertySets` holds exactly
        what :meth:`property_sets` returns for its object, or, in
        ``refusal``, the code and message it raises; only a refusal of the
        whole model (``unsupported-schema``) raises. No index outlives the
        call, so a call after an edit sees the edit.
        """
        selection = None if ids is None else list(ids)
        return list(domains._from_wire(self._native.property_sets_many(selection)))

    def resolve_unit(self, measure_type: str, unit: Optional[int] = None) -> ResolvedUnit:
        """The effective unit of a ``measure_type`` value
        (``"IFCAREAMEASURE"``): ``unit`` when given (a property's stated
        unit), otherwise the project default, resolved exactly to SI."""
        return domains._from_wire(self._native.resolve_unit(measure_type, unit))

    def set_properties(self, edits: Iterable[PropertyEdit]) -> PropertyEditResult:
        """Write and remove property and quantity values as one checked
        transaction: every edit, in order, or none.

        A refused batch raises :class:`openbim_ifc.IfcError` and leaves the
        model unchanged. A write's ``value`` is the read side's
        :attr:`Property.value`. A value an occurrence inherits from its type
        is overridden on the occurrence, never changed on the shared type
        set; pass the type object's id to change that. Values are checked
        against the declared release (IFC2X3, IFC4 or IFC4X3) and, for a
        ``Pset_``/``Qto_`` set, its PSD/QTO catalog template.
        """
        wire = []
        for edit in edits:
            if not isinstance(edit, PropertyEdit):
                raise TypeError(f"expected an openbim_ifc.PropertyEdit, got {type(edit).__name__}")
            wire.append(edit._to_wire())
        return domains._from_wire(self._native.set_properties(wire))

    def set_property(
        self,
        object: int,
        set: str,
        name: str,
        value: Value,
        *,
        set_type: Optional[str] = None,
    ) -> int:
        """Write one value (:meth:`set_properties` with one edit); returns
        the id of the entity holding it."""
        return self._native.set_property(object, set, name, to_wire(value), set_type)

    def remove_property(self, object: int, set: str, name: str) -> None:
        """Remove one property from ``object``'s own set
        (:meth:`set_properties` with one edit)."""
        self._native.remove_property(object, set, name)

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

    def product_placements(self, ids: Optional[Iterable[int]] = None) -> List[ProductPlacement]:
        """Each product's world placement (a column-major 4x4 in metres) and
        the Body representation a viewer draws, for ``ids`` or every
        product with a shape. A product that cannot be placed carries a
        typed ``refusal``; the call raises only ``unsupported-schema`` or
        ``feature-disabled``."""
        wire = self._native.product_placements(None if ids is None else list(ids))
        return [geometry._placement(row) for row in wire]

    def product_meshes(self, ids: Optional[Iterable[int]] = None) -> List[ProductMesh]:
        """Each product's Body as triangles from the reference backend, for
        ``ids`` or every product with a shape; a product that cannot be
        meshed carries a typed ``refusal``. Needs a wheel built with the
        ``mesh`` feature; the published wheel raises ``feature-disabled``."""
        wire = self._native.product_meshes(None if ids is None else list(ids))
        return [geometry._mesh(row) for row in wire]

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

    def attribute_names(self, id: int) -> List[AttributeInfo]:
        """Every explicit attribute of entity ``id`` in slot order,
        inherited first, as the release the header declares defines them.
        ``INVERSE`` attributes hold no slot and are not listed."""
        return list(domains._from_wire(self._native.attribute_names(id)))

    def attribute_by_name(self, id: int, name: str) -> Value:
        """Attribute ``name`` of entity ``id``, matched case-insensitively
        (``"Name"``) and resolved against the declared release; ``Null``
        when the record stops before its slot."""
        return from_wire(self._native.attribute_by_name(id, name))

    def set_attribute_by_name(self, id: int, name: str, value: Value) -> Value:
        """Set attribute ``name`` of entity ``id``; returns the old value.
        A derived attribute raises ``derived-attribute``, an unknown name
        ``unknown-attribute``, and a refused write changes nothing."""
        return from_wire(self._native.set_attribute_by_name(id, name, to_wire(value)))

    def set_attribute_by_name_plain(self, id: int, name: str, value: Assignable) -> Value:
        """Set attribute ``name`` of entity ``id`` from a plain Python value,
        coerced against the attribute's declared type in the declared
        release; returns the old tagged value. ``wall.Name = "x"`` calls
        this.

        A ``str`` becomes a label (written bare, ``'x'``) or the enumeration
        item it names in any case (``.STANDARD.``); an ``int`` an
        ``INTEGER`` or a ``REAL`` as declared; a ``float`` a ``REAL``; a
        ``bool`` a ``BOOLEAN`` or ``LOGICAL``; a list or tuple an aggregate,
        element by element; an :class:`openbim_ifc.Entity` a reference,
        checked to exist and to be of an accepted type; ``None`` ``$``. In a
        SELECT the one member that takes the value is written as its typed
        parameter (``IFCDESCRIPTIVEMEASURE('by layer')``). A tagged value
        (``Text``, ``Enum``, ``Typed``, ...) is written exactly as given.

        Raises ``type-mismatch`` for a value that does not fit,
        ``ambiguous-value`` when several SELECT members would take it (the
        message names them; pass a ``Typed`` value instead), and otherwise
        as :meth:`set_attribute_by_name`. A refused write changes nothing.
        """
        wire = _plain_wire(value, self)
        return from_wire(self._native.set_attribute_by_name_plain(id, name, wire))

    def add(self, type_name: str, attributes: Iterable[Value]) -> int:
        """Append an entity; returns its new id."""
        return self._native.add(type_name, [to_wire(value) for value in attributes])

    def remove(self, id: int) -> None:
        """Remove entity ``id``, leaving references to it dangling."""
        self._native.remove(id)

    def author(self, ops: Iterable[AuthorOp]) -> AuthoringResult:
        """Apply authoring operations as one checked transaction against
        the release the header declares: every operation, in order, or none.

        A refused batch raises :class:`openbim_ifc.IfcError` and leaves the
        model unchanged. An operation names the entity an earlier one
        produced by :func:`openbim_ifc.handle`; ``result.ids`` holds, per
        operation, the id the produced entity received.
        """
        wire = []
        for op in ops:
            if not isinstance(op, AuthorOp):
                raise TypeError(f"expected an openbim_ifc.AuthorOp, got {type(op).__name__}")
            wire.append(op._to_wire())
        return domains._from_wire(self._native.author(wire))

    def create_entity(self, type_name: str, attributes: Optional[Mapping[str, Value]] = None) -> int:
        """Create one entity of ``type_name`` from named attributes, checked
        against the declared release (:meth:`author` with one
        :meth:`AuthorOp.create`); returns its id."""
        wire = {name: to_wire(value) for name, value in (attributes or {}).items()}
        return self._native.create_entity(type_name, wire)

    def remove_with_relationships(self, id: int) -> None:
        """Remove entity ``id`` with the relationships that reference it,
        leaving nothing dangling (:meth:`author` with one
        :meth:`AuthorOp.remove`). Raises ``still-referenced`` while an
        entity other than a relationship needs it."""
        self._native.remove_with_relationships(id)

    def dangling_references(self) -> List[Tuple[int, int]]:
        """Every ``(from, to)`` pair where ``to`` does not exist."""
        return self._native.dangling_references()

    def __repr__(self) -> str:
        return f"<IfcModel schema={self.schema!r} entities={len(self)}>"
