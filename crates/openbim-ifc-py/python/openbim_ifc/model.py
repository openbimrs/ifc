"""The public model class."""

from __future__ import annotations

import os
import warnings
from typing import Iterable, List, Mapping, Optional, Sequence, Tuple, Union

from ._native import NativeModel
from . import domains, geometry
from .domains import (
    AttributeInfo,
    InverseAttributeInfo,
    InverseReference,
    AuthorOp,
    AuthoringResult,
    RemovalReport,
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
    TableImport,
)
from .entity import (
    Assignable,
    Entity,
    ModelAccess,
    SelectedValue,
    _plain_wire,
    _selected,
)
from .geometry import ProductCurves, ProductGeometry, ProductMesh, ProductPlacement
from .records import (
    DuplicateGlobalId,
    Header,
    ParseOptions,
    UnreachableProduct,
    ValidationReport,
)
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

        An ifcZIP archive (``.ifczip``) is recognized by its ZIP signature
        and its one model entry read, STEP or ifcXML in either layout,
        within a decompression budget: an archive with no model or several
        raises ``parse``, an encrypted entry or a compression other than
        Stored and Deflate ``unsupported``, and one that inflates past the
        budget ``budget-exceeded``.

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
        """Read a STEP (``.ifc``) file, or an ifcZIP archive as :meth:`parse`
        reads one, from disk.

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

    def write_ifczip(self, *, inner: str = "step", xsd_profile: Optional[str] = None) -> bytes:
        """Serialize as ifcZIP bytes: one Deflate entry holding STEP
        (``model.ifc``), or, with ``inner="ifcxml"``, ifcXML
        (``model.ifcXML``) in the layout ``xsd_profile`` names, as for
        :meth:`write_ifcxml`. :meth:`parse` and :meth:`open` read it back.

        Raises ``invalid-value`` for another ``inner``, or an
        ``xsd_profile`` with STEP inside.
        """
        return self._native.write_ifczip(inner, xsd_profile)

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

    def product_geometry(
        self, ids: Optional[Iterable[int]] = None, encoding: str = "json"
    ) -> List[ProductGeometry]:
        """Each product's Body as Axiolid's neutral geometry graph, exact,
        for ``ids`` or every product with a shape, in Axiolid's versioned
        wire format: ``{"format": "axiolid-geometry-graph", "version":
        "1.0", "graph": {"nodes": [...], "roots": [...]}}`` (the lowest
        version the content needs, ``"1.0"`` to ``"1.4"``), in world
        coordinates, metres. ``encoding`` ``"json"`` gives the payload as a
        ``str``, ``"cbor"`` as ``bytes``; any other raises ``ValueError``. A
        product that cannot be lowered carries a typed ``refusal``; the call
        raises only ``unsupported-schema`` or ``feature-disabled``."""
        wire = self._native.product_geometry(None if ids is None else list(ids), encoding)
        return [geometry._graph(row) for row in wire]

    def product_meshes(
        self, ids: Optional[Iterable[int]] = None, representation: str = "Body"
    ) -> List[ProductMesh]:
        """Each product's Body as triangles from the reference backend, for
        ``ids`` or every product with a shape; a product that cannot be
        meshed carries a typed ``refusal``. ``representation`` meshes
        another one instead (#458): ``"Surface"``, or any name
        :meth:`product_curves` takes; its curves are left to that call, and
        ``closure`` says ``"solid"`` or ``"surface"``. Only a Body mesh
        carries a colour. Needs a wheel built with the ``mesh`` feature; the
        published wheel raises ``feature-disabled``."""
        wire = self._native.product_meshes(None if ids is None else list(ids), representation)
        return [geometry._mesh(row) for row in wire]

    def product_curves(
        self, ids: Optional[Iterable[int]] = None, representation: str = "Plan"
    ) -> List[ProductCurves]:
        """Each product's curves as polylines from the reference backend
        (#458), for ``ids`` or every product with a shape. ``representation``
        is ``"Plan"`` (the best drawing curves: ``Plan``, ``Annotation``,
        ``FootPrint``, then ``Axis``, preferring a ``PLAN_VIEW`` context),
        or ``"Axis"``, ``"FootPrint"``, ``"Annotation"``, ``"Body"`` or
        ``"Surface"``; another name raises ``invalid-value``. A product
        whose curves cannot be compiled carries a typed ``refusal``: a
        curve the reference compiler has no certified polyline for (an
        implicit curve, an offset of an offset) is ``unsupported``;
        trimmed, composite, offset and intrinsic curves compile (#506).
        Text literals and annotation fill areas beside the curves (a
        dimension's value) are listed in ``skipped``, not refused (#512).
        Needs a wheel
        built with the ``mesh`` feature."""
        wire = self._native.product_curves(None if ids is None else list(ids), representation)
        return [geometry._curves(row) for row in wire]

    def systems(self) -> Systems:
        """Every system with its members and served structures, and the
        memberships the reader could not honour."""
        return domains._from_wire(self._native.systems())

    def cost(self) -> Cost:
        """Every cost schedule and cost item; values as authored, typed."""
        return domains._from_wire(self._native.cost())

    def georeferencing(self) -> List[MapConversion]:
        """Every coordinate operation resolved with the project length
        unit; empty when the model has none. IFC2X3 is read from its
        ``ePSet_MapConversion`` and ``ePSet_ProjectedCRS`` property sets."""
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

    def select_ids(
        self, query: str, base: Optional[Iterable[Union[int, Entity]]] = None
    ) -> List[int]:
        """The ids selector ``query`` selects, ascending (#464).

        ``"IfcWall, Pset_WallCommon.IsExternal=TRUE + IfcSlab"``: class
        facets (subtypes included) add entities, ``!`` removes them,
        attribute, property and ``type=``, ``material=``,
        ``classification=``, ``location=`` and ``group=`` facets filter;
        ``,`` chains facets and ``+`` unites lists. With ``base``, facets
        select from those entities alone. The grammar and its differences
        from IfcOpenShell's selector are documented in the guide.

        Raises ``invalid-selector`` (with ``span``, the byte range in the
        UTF-8 query) for a query that does not parse or names a class or
        attribute the file's release does not declare.
        """
        if base is None:
            return self._native.select(query)
        ids = [item.id if isinstance(item, Entity) else item for item in base]
        return self._native.select_from(query, ids)

    def select(
        self, query: str, base: Optional[Iterable[Union[int, Entity]]] = None
    ) -> List[Entity]:
        """:meth:`select_ids` as :class:`Entity` views."""
        return [Entity(self, id) for id in self.select_ids(query, base)]

    def get_value(self, entity: Union[int, Entity], path: str) -> SelectedValue:
        """The value ``path`` reads from ``entity`` (#464): ``"Name"``,
        ``"type.Name"``, ``"Pset_WallCommon.FireRating"``,
        ``"materials.Name"``, ``"storey.Name"``, ``"occurrences.count"``.

        Plain Python values as :class:`Entity` attributes read them: a
        reference is an :class:`Entity`, ``$`` is ``None``, several values
        a tuple, and a property set a read-only mapping of its properties.
        """
        id = entity.id if isinstance(entity, Entity) else entity
        return _selected(self._native.get_value(id, path), self)

    def set_value(self, entity: Union[int, Entity], path: str, value: Assignable) -> None:
        """Write ``value`` at ``path`` of ``entity`` (#464), as one checked
        transaction: an attribute (``"Name"``, ``"type.Name"``) coerced as
        :meth:`set_attribute_by_name_plain` coerces it, or a property
        (``"Pset_WallCommon.FireRating"``) from a typed value, or a plain one
        when the property already states its type. A refused write changes
        nothing; a path naming no single value raises ``invalid-value``."""
        id = entity.id if isinstance(entity, Entity) else entity
        self._native.set_value(id, path, _plain_wire(value, self))

    def export_table(
        self,
        query: str,
        columns: Sequence[str],
        *,
        format: str = "csv",
        key: Optional[str] = None,
        null: str = "",
    ) -> bytes:
        """The entities selector ``query`` selects as a table (#474):
        ``format`` ``"csv"``, ``"tsv"`` or ``"xlsx"``, the key column first
        (``"GlobalId"``, or ``"id"`` with ``key="id"``), then one column per
        value path of ``columns`` (``"Name"``, ``"type.Name"``,
        ``"Pset_WallCommon.FireRating"``), each cell the value the path
        reads as text and ``NULL`` as ``null``.

        Raises ``invalid-selector`` for a query or path that does not parse,
        ``invalid-value`` for an unknown format or key, and
        ``wrong-entity-type`` for an entity without a ``GlobalId`` in a
        table keyed by it.
        """
        return self._native.export_table(query, list(columns), format, key, null)

    def import_table(
        self,
        data: Union[bytes, str],
        *,
        format: str = "csv",
        key: Optional[str] = None,
        null: str = "",
        dry_run: bool = False,
    ) -> TableImport:
        """Apply a table (``bytes``, or ``str`` for CSV and TSV) back to the
        model by its key column (#474), as one checked transaction: every
        other cell is written by its header's value path, typed by the
        attribute's declared type or the property's stated one, and a cell
        that reads what the model holds changes nothing, so an exported
        table imported unchanged is a no-op.

        The answer lists every change and every refusal by row and column;
        any refusal means nothing was written, as does ``dry_run``.
        Raises only for a call that cannot start: an unknown format or key,
        bytes that are no table of the format.
        """
        if isinstance(data, str):
            data = data.encode("utf-8")
        return domains._from_wire(
            self._native.import_table(data, format, key, null, dry_run)
        )

    def id_by_guid(self, global_id: str) -> Optional[int]:
        """The id of the entity whose ``GlobalId`` is ``global_id``, or
        ``None``. The first lookup indexes every ``GlobalId`` once; later
        ones reuse it until the model changes. Raises ``invalid-value`` for
        text that is not a ``GlobalId`` and ``invalid-model`` when several
        entities carry it. :meth:`by_guid` returns the :class:`Entity`."""
        return self._native.by_guid(global_id)

    def duplicate_global_ids(self) -> List[DuplicateGlobalId]:
        """Every ``GlobalId`` several entities carry, with its holders;
        empty for a conformant file."""
        return [
            DuplicateGlobalId(global_id, tuple(ids))
            for global_id, ids in self._native.duplicate_global_ids()
        ]

    def type_object(self, id: int) -> Optional[int]:
        """The type object of occurrence ``id`` (``IfcRelDefinesByType``),
        or ``None``. Raises ``wrong-entity-type`` for a non-``IfcObject``
        and ``invalid-model`` when the file states two types."""
        return self._native.type_object(id)

    def occurrences_of(self, id: int) -> List[int]:
        """Every object type object ``id`` types, in file order."""
        return self._native.occurrences_of(id)

    def openings_of(self, id: int) -> List[int]:
        """Every opening voiding element ``id`` (``IfcRelVoidsElement``)."""
        return self._native.openings_of(id)

    def fillings_of(self, id: int) -> List[int]:
        """Every element filling opening ``id`` (``IfcRelFillsElement``)."""
        return self._native.fillings_of(id)

    def voided_element_of(self, id: int) -> Optional[int]:
        """The element opening ``id`` voids, or ``None``."""
        return self._native.voided_element_of(id)

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
        ``INVERSE`` attributes hold no slot and are listed by
        :meth:`inverse_attribute_names`."""
        return list(domains._from_wire(self._native.attribute_names(id)))

    def attribute_by_name(self, id: int, name: str) -> Value:
        """Attribute ``name`` of entity ``id``, matched case-insensitively
        (``"Name"``) and resolved against the declared release; ``Null``
        when the record stops before its slot.

        An ``INVERSE`` name (``"IsDefinedBy"``, #462) reads the entities
        referencing ``id`` through the release's ``FOR`` clause: a ``List``
        of ``Ref`` for a ``SET`` or ``BAG`` -- ``SET [0:1]`` included, so
        ``ContainedInStructure`` is a list of zero or one -- in ascending
        id order; a ``Ref`` or ``Null`` for a single-valued inverse
        (``VoidsElements``), which two referrers refuse with
        ``invalid-model``."""
        return from_wire(self._native.attribute_by_name(id, name))

    def inverse_attribute_names(self, id: int) -> List[InverseAttributeInfo]:
        """Every ``INVERSE`` attribute of entity ``id``, inherited ones
        first, with its ``FOR`` clause and bounds, as the declared release
        defines them."""
        return list(domains._from_wire(self._native.inverse_attribute_names(id)))

    def inverse_references(self, id: int) -> List[InverseReference]:
        """Every reference to entity ``id``: the referencing entity and the
        attribute index holding the reference, in ascending order, one per
        entity and attribute. IfcOpenShell's ``file.get_inverse(entity,
        allow_duplicate=True, with_attribute_indices=True)``."""
        return list(domains._from_wire(self._native.inverse_references(id)))

    def total_inverses(self, id: int) -> int:
        """How many distinct entities reference entity ``id``, through
        any attribute. IfcOpenShell's ``file.get_total_inverses``."""
        return self._native.total_inverses(id)

    def set_attribute_by_name(self, id: int, name: str, value: Value) -> Value:
        """Set attribute ``name`` of entity ``id``; returns the old value.
        A derived attribute raises ``derived-attribute``, an ``INVERSE``
        one ``inverse-attribute``, an unknown name ``unknown-attribute``,
        and a refused write changes nothing."""
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
        """Remove entity ``id``, leaving references to it dangling.

        .. deprecated:: 0.9
           A file with dangling references parses and is wrong. Use
           :meth:`remove_with_relationships` for one entity, or
           :meth:`remove_product` for a product with what it solely owns.
           It keeps working, with a :class:`DeprecationWarning`.
        """
        warnings.warn(
            "IfcModel.remove leaves references dangling; use "
            "remove_with_relationships or remove_product",
            DeprecationWarning,
            stacklevel=2,
        )
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

    def remove_product(
        self,
        id: int,
        *,
        rebase_placements: bool = False,
        untype_occurrences: bool = False,
        dry_run: bool = False,
    ) -> RemovalReport:
        """Remove product (or type object) ``id`` with everything it solely
        owns -- its placement, representation, own property and quantity
        sets, material usage, openings and ports, and the relationships
        left without an end -- as one checked transaction. Shared data
        stays: a property set another object uses, a type's representation
        maps, a style used elsewhere. The report says what went and what
        stayed, and why; with ``dry_run`` nothing changes.

        Raises ``still-referenced`` while other placements are relative to
        its placement (``rebase_placements`` re-bases them, composing the
        transforms so nothing moves) or while a type object types objects
        (``untype_occurrences`` unassigns the type), ``wrong-entity-type``
        for an entity that is neither an ``IfcProduct`` nor an
        ``IfcTypeObject``, and ``unsupported`` for a placement that cannot
        be re-based.
        """
        return domains._from_wire(
            self._native.remove_product(
                id,
                rebase_placements=rebase_placements,
                untype_occurrences=untype_occurrences,
                dry_run=dry_run,
            )
        )

    def purge(
        self,
        classes: Sequence[str] = (),
        *,
        include_roots: bool = False,
        include_owner_history: bool = False,
        dry_run: bool = False,
    ) -> RemovalReport:
        """Remove unreferenced entities as one checked transaction; with
        ``dry_run``, only report.

        Without ``classes``: every resource entity no ``IfcRoot`` reaches
        (nor annotates something reached, as a styled item does). With
        ``classes`` (subtypes included, any case): the entities of those
        classes nothing references; an ``IfcRoot`` among them only with
        ``include_roots``. The owner history with its actors stays unless
        ``include_owner_history``. Raises ``unsupported-schema`` for a
        class the release does not declare.
        """
        return domains._from_wire(
            self._native.purge(
                list(classes),
                include_roots=include_roots,
                include_owner_history=include_owner_history,
                dry_run=dry_run,
            )
        )

    def remove_representation(self, id: int, *, dry_run: bool = False) -> RemovalReport:
        """Remove shape representation ``id`` with what only it uses, as
        one checked transaction: it is taken off its product definition
        shape (which goes when left empty), representation map or shape
        aspects, then removed with its items down to what nothing else
        uses. Shared items, profiles, points and styles stay, and the report
        says why; with ``dry_run`` nothing changes.

        Raises ``still-referenced`` for a representation map an
        ``IfcMappedItem`` still maps, and ``wrong-entity-type`` for an entity
        that is not an ``IfcShapeModel``.
        """
        return domains._from_wire(self._native.remove_representation(id, dry_run=dry_run))

    def convert_unit_value(self, value: float, from_unit: int, to_unit: int) -> float:
        """Convert ``value`` from unit entity ``from_unit`` to unit entity
        ``to_unit`` of this model (an ``IfcSIUnit``,
        ``IfcConversionBasedUnit`` or ``IfcDerivedUnit``), exactly: prefixes
        convert by a power of ten, conversion-based units through their
        factor, derived units by their exponents.

        Raises ``type-mismatch`` for units of different dimensions,
        ``unsupported`` for a monetary or context-dependent unit or one with
        an offset (#110), and ``missing-entity`` for an id the model lacks.
        """
        return self._native.convert_unit_value(value, from_unit, to_unit)

    def convert_length_unit(
        self,
        target: str,
        *,
        area_and_volume: bool = True,
        remove_replaced_units: bool = True,
        dry_run: bool = False,
    ) -> "domains.LengthUnitReport":
        """Convert the model's length unit to ``target`` (an SI ``METRE``
        with any prefix, such as ``"MILLIMETRE"``, or ``"inch"``,
        ``"foot"``, ``"yard"``, ``"mile"``, ``"US survey foot"``), rescaling
        every length, area and volume value -- geometry, placements,
        quantities and property values -- as one checked transaction.

        Which values are lengths is read from the release's measure types;
        values with a unit of their own keep it, and curve parameters that
        are angles, knots or multiples stay. With ``area_and_volume`` the
        area and volume units become the square and cube of the new length
        unit; with ``dry_run`` nothing changes and the report says what
        would.

        Raises ``unsupported`` naming every value the conversion cannot
        carry (a p-curve off a plane, a polynomial curve, a parameter on an
        undefined parameter space), ``invalid-value`` for a target that is
        not a length unit, ``invalid-model`` when the project length unit
        cannot be resolved and ``unsupported-schema`` for a release other
        than IFC2X3, IFC4 or IFC4X3. A refusal leaves the model unchanged.
        """
        return domains._from_wire(
            self._native.convert_length_unit(
                target,
                area_and_volume=area_and_volume,
                remove_replaced_units=remove_replaced_units,
                dry_run=dry_run,
            )
        )

    def dangling_references(self) -> List[Tuple[int, int]]:
        """Every ``(from, to)`` pair where ``to`` does not exist."""
        return self._native.dangling_references()

    def __repr__(self) -> str:
        return f"<IfcModel schema={self.schema!r} entities={len(self)}>"
