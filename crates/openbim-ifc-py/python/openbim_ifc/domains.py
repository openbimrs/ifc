"""Domain records (#123): property sets, the spatial tree, classification,
materials, systems, cost and georeferencing. All frozen dataclasses.

Each mirrors one record of the shared binding core, field for field, so
the JavaScript and C bindings carry the same data under their own names.
Ids are ``int``; an absent field is ``None``; an IFC value is one of the
:mod:`openbim_ifc.values` classes, in the lossless tagged encoding; a list
is a tuple.

The records are snapshots: a domain view borrows the model and cannot
leave the native layer, so each call reads the model as it is then.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Mapping, Optional, Sequence, Tuple

from .values import Value, from_wire as _value_from_wire, to_wire as _value_to_wire


@dataclass(frozen=True)
class Property:
    """One property, quantity or predefined-set attribute.

    ``kind`` is ``value``, ``enumerated``, ``list``, ``bounded``, ``table``,
    ``reference`` or ``complex``. ``value`` keeps the declared type:
    ``Typed("IFCLENGTHMEASURE", Real(0.2))``. ``unit`` is the unit the
    property states; :meth:`IfcModel.resolve_unit` resolves it, or the
    project default when it is ``None``.
    """

    id: int
    name: str
    type_name: str
    kind: str
    value_type: Optional[str]
    unit: Optional[int]
    value: Value
    enumeration: Optional["PropertyEnumeration"]
    bounds: Optional["PropertyBounds"]
    table: Optional["PropertyTable"]
    usage: Optional[str]
    discrimination: Optional[str]
    quality: Optional[str]
    members: Tuple["Property", ...]


@dataclass(frozen=True)
class PropertySet:
    """A property set, quantity set or predefined property set.

    ``source`` is ``occurrence`` (stated on the object) or ``type`` (held by
    the type object ``source_id``). An inherited property the occurrence
    overrides is left out.
    """

    id: int
    global_id: Optional[str]
    name: str
    type_name: str
    source: str
    source_id: Optional[int]
    properties: Tuple[Property, ...]


@dataclass(frozen=True)
class PropertyEnumeration:
    """An ``IfcPropertyEnumeration``: the values an enumerated property permits."""

    id: int
    name: str
    values: Tuple[Value, ...]


@dataclass(frozen=True)
class PropertyBounds:
    """An ``IfcPropertyBoundedValue``'s values; ``Null()`` when unstated."""

    lower: Value
    upper: Value
    set_point: Value


@dataclass(frozen=True)
class PropertyTableRow:
    """One row of a :class:`PropertyTable`."""

    defining: Value
    defined: Value


@dataclass(frozen=True)
class PropertyTable:
    """An ``IfcPropertyTableValue``."""

    rows: Tuple[PropertyTableRow, ...]
    expression: Optional[str]
    defining_unit: Optional[int]
    defined_unit: Optional[int]
    interpolation: Optional[str]


@dataclass(frozen=True)
class ResolvedUnit:
    """A measure's effective unit: ``si = value * scale + offset``.

    ``dimensions`` are the SI exponents ``(L, M, T, I, Θ, N, J)``.
    """

    unit: Optional[int]
    from_project: bool
    dimensions: Tuple[int, ...]
    scale: float
    offset: float


@dataclass(frozen=True)
class SpatialNode:
    """A spatial container. ``kind`` is ``project``, ``site``, ``building``,
    ``storey``, ``space`` or ``other``; ``elements`` are contained,
    ``referenced`` only referenced."""

    id: int
    global_id: Optional[str]
    name: Optional[str]
    type_name: str
    kind: str
    parent: Optional[int]
    children: Tuple[int, ...]
    elements: Tuple[int, ...]
    referenced: Tuple[int, ...]


@dataclass(frozen=True)
class SpatialDanglingReference:
    """A relationship naming an entity the file lacks."""

    relation: int
    target: int


@dataclass(frozen=True)
class SpatialAnomaly:
    """A second parent or a non-container structure the tree rejected."""

    kind: str
    relation: int
    subject: int
    kept: Optional[int]


@dataclass(frozen=True)
class SpatialTree:
    """The containment tree. ``release`` is the release containers were
    classified against (``IFC4_ADD2_TC1``), or ``None`` for one the tree is
    not verified for."""

    release: Optional[str]
    roots: Tuple[int, ...]
    nodes: Tuple[SpatialNode, ...]
    orphans: Tuple[int, ...]
    dangling: Tuple[SpatialDanglingReference, ...]
    anomalies: Tuple[SpatialAnomaly, ...]


@dataclass(frozen=True)
class ClassificationSystem:
    """An ``IfcClassification``."""

    id: int
    name: str
    source: Optional[str]
    edition: Optional[str]


@dataclass(frozen=True)
class Classification:
    """A classification that applies to an object.

    ``kind`` is ``reference``, ``system`` or ``notation`` (IFC2X3);
    ``source`` is ``occurrence`` or ``type`` (via ``type_object``).
    ``parents`` are the references above a reference, nearest first.
    """

    relationship: int
    global_id: Optional[str]
    source: str
    type_object: Optional[int]
    target: int
    kind: str
    identification: Optional[str]
    name: Optional[str]
    location: Optional[str]
    notation: Tuple[str, ...]
    parents: Tuple[int, ...]
    system: Optional[ClassificationSystem]


@dataclass(frozen=True)
class MaterialRef:
    """An ``IfcMaterial``."""

    id: int
    name: str
    category: Optional[str]


@dataclass(frozen=True)
class MaterialLayer:
    """An ``IfcMaterialLayer``; ``thickness`` in the file's length unit,
    ``is_ventilated`` a ``Bool``, ``Unknown`` or ``Null`` value."""

    id: int
    material: Optional[MaterialRef]
    thickness: float
    is_ventilated: Value
    name: Optional[str]
    category: Optional[str]
    priority: Optional[int]


@dataclass(frozen=True)
class MaterialProfile:
    """An ``IfcMaterialProfile``; ``profile`` is its ``IfcProfileDef``."""

    id: int
    material: Optional[MaterialRef]
    profile: int
    name: Optional[str]
    category: Optional[str]
    priority: Optional[int]


@dataclass(frozen=True)
class MaterialConstituent:
    """An ``IfcMaterialConstituent``."""

    id: int
    material: MaterialRef
    name: Optional[str]
    category: Optional[str]
    fraction: Optional[float]


@dataclass(frozen=True)
class MaterialUsage:
    """How a layer or profile set is placed."""

    layer_set_direction: Optional[str]
    direction_sense: Optional[str]
    offset_from_reference_line: Optional[float]
    reference_extent: Optional[float]
    cardinal_point: Optional[int]
    end_set: Optional[int]
    cardinal_end_point: Optional[int]


@dataclass(frozen=True)
class MaterialAssignment:
    """The material association of an object, its own or its type's.

    ``kind`` is ``material``, ``list``, ``layer``, ``layer-set``,
    ``layer-set-usage``, ``profile``, ``profile-set``,
    ``profile-set-usage``, ``constituent`` or ``constituent-set``.
    """

    relationship: int
    global_id: Optional[str]
    source: str
    type_object: Optional[int]
    target: int
    type_name: str
    kind: str
    set: Optional[int]
    name: Optional[str]
    materials: Tuple[MaterialRef, ...]
    layers: Tuple[MaterialLayer, ...]
    profiles: Tuple[MaterialProfile, ...]
    constituents: Tuple[MaterialConstituent, ...]
    usage: Optional[MaterialUsage]


@dataclass(frozen=True)
class System:
    """An ``IfcSystem`` with its members and the structures it serves."""

    id: int
    global_id: Optional[str]
    type_name: str
    name: Optional[str]
    long_name: Optional[str]
    predefined_type: Optional[str]
    members: Tuple[int, ...]
    serviced_buildings: Tuple[int, ...]
    serviced_facilities: Tuple[int, ...]


@dataclass(frozen=True)
class SystemAnomaly:
    """A membership the systems reader could not honour."""

    kind: str
    subject: int
    other: Optional[int]
    message: str


@dataclass(frozen=True)
class Systems:
    """Every system, and what the reader could not place."""

    systems: Tuple[System, ...]
    anomalies: Tuple[SystemAnomaly, ...]


@dataclass(frozen=True)
class UnitBasis:
    """The ``IfcMeasureWithUnit`` a rate is stated per."""

    id: int
    value: Value
    unit: Optional[int]


@dataclass(frozen=True)
class CostValue:
    """An ``IfcCostValue``; ``applied_value`` as authored, typed."""

    id: int
    name: Optional[str]
    description: Optional[str]
    category: Optional[str]
    condition: Optional[str]
    applied_value: Value
    operator: Optional[str]
    unit_basis: Optional[UnitBasis]
    components: Tuple["CostValue", ...]


@dataclass(frozen=True)
class CostItem:
    """An ``IfcCostItem``; ``objects`` are the objects it prices."""

    id: int
    global_id: Optional[str]
    name: Optional[str]
    identification: Optional[str]
    description: Optional[str]
    predefined_type: Optional[str]
    parent: Optional[int]
    children: Tuple[int, ...]
    values: Tuple[CostValue, ...]
    quantities: Tuple[int, ...]
    objects: Tuple[int, ...]


@dataclass(frozen=True)
class CostSchedule:
    """An ``IfcCostSchedule``; ``items`` are the objects assigned to it."""

    id: int
    global_id: Optional[str]
    name: Optional[str]
    identification: Optional[str]
    status: Optional[str]
    predefined_type: Optional[str]
    items: Tuple[int, ...]


@dataclass(frozen=True)
class CostAnomaly:
    """A cost item nested under two parents."""

    item: int
    kept: int
    rejected: int
    relation: int


@dataclass(frozen=True)
class Cost:
    """Every cost schedule and cost item."""

    schedules: Tuple[CostSchedule, ...]
    items: Tuple[CostItem, ...]
    anomalies: Tuple[CostAnomaly, ...]


@dataclass(frozen=True)
class ProjectedCrs:
    """An ``IfcProjectedCRS``."""

    id: int
    name: Optional[str]
    description: Optional[str]
    geodetic_datum: Optional[str]
    vertical_datum: Optional[str]
    map_projection: Optional[str]
    map_zone: Optional[str]
    well_known_text: Optional[str]


@dataclass(frozen=True)
class LengthUnit:
    """A length unit reduced to metres."""

    name: str
    metres_per_unit: float


@dataclass(frozen=True)
class MapConversion:
    """A coordinate operation resolved with the project length unit.

    ``linear`` maps project metres to map metres as three columns;
    ``translation`` is where the project origin lands, in map metres.
    """

    operation: int
    kind: str
    source: int
    source_kind: str
    target_crs: ProjectedCrs
    eastings: float
    northings: float
    orthogonal_height: float
    x_axis: Tuple[float, ...]
    scale: float
    factors: Optional[Tuple[float, ...]]
    project_unit: LengthUnit
    map_unit: LengthUnit
    map_unit_declared: bool
    linear: Tuple[Tuple[float, ...], ...]
    translation: Tuple[float, ...]



@dataclass(frozen=True)
class PropertyEdit:
    """One edit for :meth:`IfcModel.set_properties`.

    Writes ``value`` to property ``name`` of set ``set`` on ``object``, or,
    built with :meth:`removal`, removes it. ``value`` is the read side's
    :attr:`Property.value`: a ``Typed`` ``IfcValue`` (or ``Null()``), a
    ``List`` of them for an enumerated or list value, a typed measure for a
    quantity. ``set_type`` (``"IfcPropertySet"`` or ``"IfcElementQuantity"``)
    names the entity of a set the edit creates that neither the type object
    nor the catalog describes.
    """

    object: int
    set: str
    name: str
    value: Optional[Value] = None
    set_type: Optional[str] = None
    remove: bool = False

    @classmethod
    def removal(cls, object: int, set: str, name: str) -> "PropertyEdit":
        """An edit removing property ``name`` from ``object``'s own set."""
        return cls(object, set, name, remove=True)

    def _to_wire(self) -> Dict[str, Any]:
        return {
            "object": self.object,
            "set": self.set,
            "name": self.name,
            "value": None if self.value is None else _value_to_wire(self.value),
            "set_type": self.set_type,
            "remove": self.remove,
        }


@dataclass(frozen=True)
class PropertyEditResult:
    """What a committed :meth:`IfcModel.set_properties` batch did.

    ``properties`` holds, per edit, the entity holding the property
    afterwards, or ``None`` when the batch leaves none.
    """

    properties: Tuple[Optional[int], ...]
    created: Tuple[int, ...]
    removed: Tuple[int, ...]

@dataclass(frozen=True)
class AttributeInfo:
    """One explicit attribute of an entity, from
    :meth:`IfcModel.attribute_names`, as the release the header declares
    defines it.

    ``name`` is the schema's spelling (``GlobalId``); ``index`` its slot,
    the ``index`` of :meth:`IfcModel.attribute`. ``type_name`` is the
    declared type, or an aggregate's element type. A ``derived`` attribute
    is written ``*`` and :meth:`IfcModel.set_attribute_by_name` refuses it.
    ``declared_by`` names the entity that introduces it (``IfcRoot``).
    """

    name: str
    index: int
    type_name: str
    optional: bool
    aggregate: bool
    derived: bool
    declared_by: str


#: The first id of the handle range (2**62): ``HANDLE_BASE + i`` names the
#: entity operation ``i`` of an :meth:`IfcModel.author` batch produced.
HANDLE_BASE = 1 << 62


def handle(index: int) -> int:
    """The handle of the entity operation ``index`` of an
    :meth:`IfcModel.author` batch produces, usable wherever a later
    operation of the batch takes an id, attribute values included."""
    if not isinstance(index, int) or isinstance(index, bool) or not 0 <= index < HANDLE_BASE:
        raise ValueError(f"a handle index is a non-negative int, not {index!r}")
    return HANDLE_BASE + index


@dataclass(frozen=True)
class AuthorOp:
    """One operation of :meth:`IfcModel.author`, built with the class
    methods below.

    Ids are entity ids or :func:`handle` values. ``attributes`` map
    attribute names (any case) to :mod:`openbim_ifc.values` values. A
    builder writes the ``owner_history`` it is given on every record it
    creates; none is invented, and IFC2X3, which requires one, refuses a
    record without.
    """

    op: str
    fields: Tuple[Tuple[str, Any], ...] = ()

    @classmethod
    def _of(cls, op: str, **fields: Any) -> "AuthorOp":
        return cls(op, tuple((key, value) for key, value in fields.items() if value is not None))

    @classmethod
    def create(cls, type_name: str, attributes: Optional[Mapping[str, Value]] = None) -> "AuthorOp":
        """One entity of ``type_name`` from named attributes; an ``IfcRoot``
        without a ``GlobalId`` gets one."""
        return cls._of("create", type=type_name, attributes=attributes)

    @classmethod
    def edit(cls, entity: int, attributes: Mapping[str, Value]) -> "AuthorOp":
        """Replace named attributes; the whole entity is checked again."""
        return cls._of("edit", entity=entity, attributes=attributes)

    @classmethod
    def remove(cls, entity: int) -> "AuthorOp":
        """Remove an entity with the relationships that reference it."""
        return cls._of("remove", entity=entity)

    @classmethod
    def project(
        cls,
        attributes: Optional[Mapping[str, Value]] = None,
        *,
        owner_history: Optional[int] = None,
    ) -> "AuthorOp":
        """The model's one ``IfcProject``."""
        return cls._of("project", attributes=attributes, owner_history=owner_history)

    @classmethod
    def spatial(
        cls,
        type_name: str,
        parent: int,
        attributes: Optional[Mapping[str, Value]] = None,
        *,
        placement: Optional[int] = None,
        owner_history: Optional[int] = None,
    ) -> "AuthorOp":
        """A spatial element aggregated under ``parent``
        (``IfcRelAggregates``)."""
        return cls._of(
            "spatial",
            type=type_name,
            parent=parent,
            attributes=attributes,
            placement=placement,
            owner_history=owner_history,
        )

    @classmethod
    def product(
        cls,
        type_name: str,
        attributes: Optional[Mapping[str, Value]] = None,
        *,
        container: Optional[int] = None,
        placement: Optional[int] = None,
        type_object: Optional[int] = None,
        owner_history: Optional[int] = None,
    ) -> "AuthorOp":
        """A product, contained in ``container``
        (``IfcRelContainedInSpatialStructure``) and typed by
        ``type_object`` (``IfcRelDefinesByType``)."""
        return cls._of(
            "product",
            type=type_name,
            attributes=attributes,
            container=container,
            placement=placement,
            type_object=type_object,
            owner_history=owner_history,
        )

    @classmethod
    def type_object(
        cls,
        type_name: str,
        attributes: Optional[Mapping[str, Value]] = None,
        *,
        owner_history: Optional[int] = None,
    ) -> "AuthorOp":
        """A type object (``IfcWallType``, ...)."""
        return cls._of("type_object", type=type_name, attributes=attributes, owner_history=owner_history)

    @classmethod
    def assign_type(
        cls, type_object: int, objects: Sequence[int], *, owner_history: Optional[int] = None
    ) -> "AuthorOp":
        """``IfcRelDefinesByType``; an object already typed is refused."""
        return cls._of("assign_type", type_object=type_object, objects=list(objects), owner_history=owner_history)

    @classmethod
    def contain(
        cls, structure: int, elements: Sequence[int], *, owner_history: Optional[int] = None
    ) -> "AuthorOp":
        """``IfcRelContainedInSpatialStructure``; an element already
        contained is refused."""
        return cls._of("contain", structure=structure, elements=list(elements), owner_history=owner_history)

    @classmethod
    def aggregate(
        cls, parent: int, parts: Sequence[int], *, owner_history: Optional[int] = None
    ) -> "AuthorOp":
        """``IfcRelAggregates``; a part already aggregated is refused."""
        return cls._of("aggregate", parent=parent, parts=list(parts), owner_history=owner_history)

    @classmethod
    def placement(
        cls,
        *,
        relative_to: Optional[int] = None,
        location: Optional[Sequence[float]] = None,
        axis: Optional[Sequence[float]] = None,
        ref_direction: Optional[Sequence[float]] = None,
    ) -> "AuthorOp":
        """An ``IfcLocalPlacement``; ``axis`` and ``ref_direction`` both or
        neither."""
        return cls._of(
            "placement",
            relative_to=relative_to,
            location=None if location is None else list(location),
            axis=None if axis is None else list(axis),
            ref_direction=None if ref_direction is None else list(ref_direction),
        )

    @classmethod
    def owner_history(
        cls,
        *,
        organization: str,
        application_name: str,
        application_version: str,
        application_identifier: str,
        creation_date: int,
        person_identification: Optional[str] = None,
        family_name: Optional[str] = None,
        given_name: Optional[str] = None,
        change_action: Optional[str] = None,
        last_modified_date: Optional[int] = None,
    ) -> "AuthorOp":
        """An ``IfcOwnerHistory`` with its person, organization and
        application, through ``ifc-author``."""
        return cls._of(
            "owner_history",
            organization=organization,
            application_name=application_name,
            application_version=application_version,
            application_identifier=application_identifier,
            creation_date=creation_date,
            person_identification=person_identification,
            family_name=family_name,
            given_name=given_name,
            change_action=change_action,
            last_modified_date=last_modified_date,
        )

    def _to_wire(self) -> Dict[str, Any]:
        wire: Dict[str, Any] = {"op": self.op}
        for key, value in self.fields:
            if key == "attributes":
                value = {name: _value_to_wire(item) for name, item in value.items()}
            wire[key] = value
        return wire


@dataclass(frozen=True)
class AuthoringResult:
    """What a committed :meth:`IfcModel.author` batch did.

    ``ids`` holds, per operation, the id of the entity it produced, or
    ``None`` for a removal.
    """

    ids: Tuple[Optional[int], ...]
    created: Tuple[int, ...]
    removed: Tuple[int, ...]


_RECORDS = {
    cls.__name__: cls
    for cls in (
        Property,
        PropertySet,
        PropertyEnumeration,
        PropertyBounds,
        PropertyTableRow,
        PropertyTable,
        ResolvedUnit,
        SpatialNode,
        SpatialDanglingReference,
        SpatialAnomaly,
        SpatialTree,
        ClassificationSystem,
        Classification,
        MaterialRef,
        MaterialLayer,
        MaterialProfile,
        MaterialConstituent,
        MaterialUsage,
        MaterialAssignment,
        System,
        SystemAnomaly,
        Systems,
        UnitBasis,
        CostValue,
        CostItem,
        CostSchedule,
        CostAnomaly,
        Cost,
        ProjectedCrs,
        LengthUnit,
        MapConversion,
        PropertyEditResult,
        AttributeInfo,
        AuthoringResult,
    )
}


def _from_wire(data: Any) -> Any:
    """Build a record, a value or a tuple from the native layer's wire form."""
    if isinstance(data, dict):
        if "_record" in data:
            name = data["_record"]
            fields: Dict[str, Any] = {
                key: _from_wire(value) for key, value in data.items() if key != "_record"
            }
            return _RECORDS[name](**fields)
        return _value_from_wire(data)
    if isinstance(data, list):
        return tuple(_from_wire(item) for item in data)
    return data


__all__ = sorted([*_RECORDS, "PropertyEdit", "AuthorOp", "HANDLE_BASE", "handle"])
