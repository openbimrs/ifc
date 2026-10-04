"""Pythonic access (#332): entities as objects, plain values, property sets
as mappings.

A pure-Python layer over :class:`openbim_ifc.IfcModel`'s calls; it adds no
IFC semantics (ADR 0013). An :class:`Entity` is a light view -- the model
and an id -- that reads and writes through ``attribute_by_name`` and
``set_attribute_by_name``, so names resolve against the release the header
declares and match ASCII case-insensitively, exactly as those calls do.

Reads return *plain* Python values (:data:`PlainValue`) for the common
cases. The conversion is lossy where the tagged values are not:

==============================  ===========================================
Tagged value                    Plain value
==============================  ===========================================
``Null()`` (``$``)              ``None``
``Derived()`` (``*``)           ``None`` -- lossy: indistinguishable from ``$``
``Unknown()`` (``.U.``)         ``None`` -- lossy: indistinguishable from ``$``
``Bool(b)``                     ``b``
``Integer(i)`` / ``Real(r)``    ``int`` / ``float`` (the Python type keeps
                                the distinction)
``Text(s)``                     ``s``
``Enum(s)``                     ``s`` -- lossy: indistinguishable from text
``Binary(h)``                   unchanged, a ``Binary``: no plain form
                                without decoding
``Ref(id)``                     an :class:`Entity` of the same model
``List(items)``                 a ``tuple`` of plain values
``Typed(t, v)``                 the plain value of ``v`` -- lossy: the type
                                name ``t`` (``IFCLABEL``) is dropped
==============================  ===========================================

:meth:`Entity.raw` and :meth:`Entity.property_sets` return the exact tagged
values, which is what a write takes.
"""

from __future__ import annotations

from types import MappingProxyType
from typing import (
    TYPE_CHECKING,
    Any,
    Dict,
    Iterator,
    Mapping,
    Optional,
    Sequence,
    Tuple,
    Union,
    cast,
)

from ._native import IfcError
from .domains import AttributeInfo, Property, PropertySet
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

if TYPE_CHECKING:
    import pandas

    from .model import IfcModel

PlainValue = Union[None, bool, int, float, str, Binary, "Entity", Tuple["PlainValue", ...]]
"""What a read returns: a plain Python value, an :class:`Entity` for a
reference, a tuple for an aggregate; see the module docs for what is lost."""

PropertyValue = Union[PlainValue, Property, Mapping[str, "PropertyValue"]]
"""A value in :attr:`Entity.psets`: the plain value of a single, enumerated,
list or reference property; a read-only mapping of a complex property's
members; the exact :class:`Property` record of a bounded or table value,
which has no single plain form."""

Assignable = Union[Value, "Entity", None, Tuple["Assignable", ...], list["Assignable"]]
"""What an attribute write takes: a tagged value, an :class:`Entity` (a
reference), ``None`` (``$``), or a tuple or list of these (an aggregate).
Bare ``str``, ``int``, ``float`` and ``bool`` are refused: ``"x"`` could be
``Text`` or ``Enum``, ``1`` ``Integer`` or ``Real``, and the binding does not
guess."""

_QUANTITY_SET = "IFCELEMENTQUANTITY"


def plain(value: Value, model: "IfcModel") -> PlainValue:
    """The plain Python form of a tagged ``value`` read from ``model``.

    References become :class:`Entity` views of ``model``; see the module
    docs for the conversions that lose information.
    """
    if isinstance(value, (Null, Derived, Unknown)):
        return None
    if isinstance(value, (Bool, Integer, Real, Text, Enum)):
        return value.value
    if isinstance(value, Binary):
        return value
    if isinstance(value, Ref):
        return Entity(model, value.id)
    if isinstance(value, List):
        return tuple(plain(item, model) for item in value.items)
    if isinstance(value, Typed):
        return plain(value.value, model)
    raise TypeError(f"expected an openbim_ifc value, got {type(value).__name__}")


def _assignable(value: Assignable, model: "IfcModel") -> Value:
    """The tagged value an attribute write stores; refuses anything that
    would need a guess."""
    if value is None:
        return Null()
    if isinstance(value, Entity):
        if value.model is not model:
            raise ValueError(f"{value!r} belongs to another model")
        return Ref(value.id)
    if isinstance(value, (tuple, list)):
        return List(tuple(_assignable(item, model) for item in value))
    if isinstance(value, (Null, Derived, Bool, Unknown, Integer, Real, Text, Binary, Enum, Ref, List, Typed)):
        return value
    raise TypeError(
        f"cannot write a bare {type(value).__name__}: wrap it in the value it is, "
        "e.g. Text('x'), Enum('ELEMENT'), Real(1.0), Integer(1), Bool(True) or "
        "Typed('IFCLABEL', Text('x'))"
    )


def _property_value(prop: Property, model: "IfcModel") -> PropertyValue:
    if prop.kind in ("value", "enumerated", "list", "reference"):
        return plain(prop.value, model)
    if prop.kind == "complex":
        return MappingProxyType({m.name: _property_value(m, model) for m in prop.members})
    # bounded, table, and any form a later release adds: the exact record.
    return prop


Sets = Mapping[str, Mapping[str, PropertyValue]]


def _sets(model: "IfcModel", id: int) -> Tuple[Sets, Sets]:
    """``(psets, qtos)`` of object ``id``, from one ``property_sets`` call.

    The native call lists the object's own sets first and leaves out an
    inherited property the occurrence overrides; sets of one name merge,
    the occurrence's properties first.
    """
    psets: Dict[str, Dict[str, PropertyValue]] = {}
    qtos: Dict[str, Dict[str, PropertyValue]] = {}
    for pset in model.property_sets(id):
        target = qtos if pset.type_name == _QUANTITY_SET else psets
        values = target.setdefault(pset.name, {})
        for prop in pset.properties:
            values.setdefault(prop.name, _property_value(prop, model))
    return _frozen(psets), _frozen(qtos)


def _frozen(sets: Dict[str, Dict[str, PropertyValue]]) -> Sets:
    return MappingProxyType({name: MappingProxyType(values) for name, values in sets.items()})


class Entity:
    """One entity of a model, read and written by attribute name.

    ``wall.Name`` reads attribute ``Name`` as a plain value (see
    :data:`PlainValue`) and ``wall.Name = Text("W1")`` writes it, both
    through the model's ``attribute_by_name``/``set_attribute_by_name``, so
    names match ASCII case-insensitively against the declared release. An
    unknown name raises ``AttributeError``, chained from the
    ``unknown-attribute`` :class:`openbim_ifc.IfcError`. The view holds no
    data: every read sees the model as it is then. Two views are equal when
    they name the same id of the same model.
    """

    __slots__ = ("_model", "_id")

    if TYPE_CHECKING:
        _model: IfcModel
        _id: int

    def __init__(self, model: "IfcModel", id: int) -> None:
        """A view of entity ``id``; :meth:`IfcModel.by_id` checks it exists."""
        object.__setattr__(self, "_model", model)
        object.__setattr__(self, "_id", id)

    @property
    def id(self) -> int:
        """The entity's ``#id``."""
        return self._id

    @property
    def model(self) -> "IfcModel":
        """The model the entity belongs to."""
        return self._model

    @property
    def type(self) -> str:
        """The upper-case type name, e.g. ``"IFCWALL"``."""
        return self._model.type_of(self._id)

    def is_a(self, type_name: str) -> bool:
        """Whether the entity is a ``type_name`` or a subtype of it, per
        the file's schema; case-insensitive. A name the schema lacks is
        ``False``."""
        return self._model._is_a(self._id, self.type, type_name)

    def get(self, name: str) -> PlainValue:
        """Attribute ``name`` as a plain value; ``entity.Name`` with the
        precise type. Raises :class:`openbim_ifc.IfcError` as
        :meth:`IfcModel.attribute_by_name` does."""
        return plain(self._model.attribute_by_name(self._id, name), self._model)

    def raw(self, name: str) -> Value:
        """Attribute ``name`` as the exact tagged value, e.g.
        ``Typed("IFCLABEL", Text("W1"))`` or ``Derived()``."""
        return self._model.attribute_by_name(self._id, name)

    def set(self, name: str, value: Assignable) -> Value:
        """Write attribute ``name`` (``entity.Name = value``); returns the
        old tagged value. A refused write changes nothing."""
        return self._model.set_attribute_by_name(self._id, name, _assignable(value, self._model))

    def attribute_names(self) -> list[AttributeInfo]:
        """Every explicit attribute, in slot order, as
        :meth:`IfcModel.attribute_names` lists them."""
        return self._model.attribute_names(self._id)

    def property_sets(self) -> list[PropertySet]:
        """The exact records :meth:`IfcModel.property_sets` returns."""
        return self._model.property_sets(self._id)

    @property
    def psets(self) -> Mapping[str, Mapping[str, PropertyValue]]:
        """Property sets and predefined property sets by name, each a
        read-only mapping of property name to :data:`PropertyValue`.

        The object's own values and those its type object holds, an
        occurrence value overriding an inherited one, as
        :meth:`IfcModel.property_sets` resolves them; quantity sets are in
        :attr:`qtos`. A snapshot: read it again after a write. Write with
        :meth:`set_property`.
        """
        return _sets(self._model, self._id)[0]

    @property
    def qtos(self) -> Mapping[str, Mapping[str, PropertyValue]]:
        """Quantity sets (``IfcElementQuantity``) by name, as :attr:`psets`."""
        return _sets(self._model, self._id)[1]

    def set_property(
        self, set: str, name: str, value: Value, *, set_type: Optional[str] = None
    ) -> int:
        """Write one property or quantity value: :meth:`IfcModel.set_property`
        on this entity. ``value`` is exact, e.g. ``Typed("IFCLABEL",
        Text("F60"))``. Returns the id of the entity holding it."""
        return self._model.set_property(self._id, set, name, value, set_type=set_type)

    def remove_property(self, set: str, name: str) -> None:
        """Remove one property from the entity's own set:
        :meth:`IfcModel.remove_property`."""
        self._model.remove_property(self._id, set, name)

    def __getattr__(self, name: str) -> Any:
        # Only called for names the class lacks: IFC attributes.
        if name.startswith("_"):
            raise AttributeError(name)
        try:
            return self.get(name)
        except IfcError as error:
            if error.code == "unknown-attribute":
                raise AttributeError(f"{self!r} has no attribute {name!r}") from error
            raise

    def __setattr__(self, name: str, value: Assignable) -> None:
        if name.startswith("_") or hasattr(type(self), name):
            raise AttributeError(f"{name!r} is not an IFC attribute and cannot be set")
        try:
            self.set(name, value)
        except IfcError as error:
            if error.code == "unknown-attribute":
                raise AttributeError(f"{self!r} has no attribute {name!r}") from error
            raise

    def __delattr__(self, name: str) -> None:
        raise AttributeError("an attribute cannot be deleted; assign None to unset it")

    def __dir__(self) -> list[str]:
        names = list(super().__dir__())
        try:
            names.extend(info.name for info in self.attribute_names())
        except IfcError:
            pass
        return sorted(set(names))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Entity):
            return NotImplemented
        return self._model is other._model and self._id == other._id

    def __hash__(self) -> int:
        return hash(self._id)

    def __repr__(self) -> str:
        try:
            return f"<Entity #{self._id}={self.type}>"
        except IfcError:
            return f"<Entity #{self._id} (missing)>"


class ModelAccess:
    """Entity access for :class:`openbim_ifc.IfcModel`; documented there."""

    __slots__ = ("_subtypes",)

    if TYPE_CHECKING:
        _subtypes: Dict[Tuple[Optional[str], str, str], bool]

    def _as_model(self) -> "IfcModel":
        return cast("IfcModel", self)

    def by_id(self, id: int) -> Entity:
        """Entity ``#id`` as an :class:`Entity`; raises
        :class:`openbim_ifc.IfcError` (``missing-entity``) when absent."""
        model = self._as_model()
        model.type_of(id)
        return Entity(model, id)

    def __getitem__(self, id: int) -> Entity:
        """``model[id]``: :meth:`by_id`, raising ``KeyError`` when absent."""
        if not self.__contains__(id):
            raise KeyError(id)
        return Entity(self._as_model(), id)

    def __contains__(self, item: object) -> bool:
        """``id in model`` (an ``int``) or ``entity in model``."""
        model = self._as_model()
        if isinstance(item, Entity):
            if item.model is not model:
                return False
            item = item.id
        if not isinstance(item, int) or isinstance(item, bool) or item < 0:
            return False
        try:
            model.type_of(item)
        except IfcError as error:
            if error.code == "missing-entity":
                return False
            raise
        except OverflowError:
            return False
        return True

    def __iter__(self) -> Iterator[Entity]:
        """``iter(model)``: every entity as an :class:`Entity`, in file order."""
        model = self._as_model()
        return (Entity(model, id) for id in model.ids())

    def by_type(self, type_name: str, *, include_subtypes: bool = True) -> list[Entity]:
        """Every entity of ``type_name`` (case-insensitive) as an
        :class:`Entity`, in file order; with ``include_subtypes=False`` only
        those of exactly that type. A name the schema lacks gives ``[]``."""
        model = self._as_model()
        ids = (
            model.ids_of_type_including_subtypes(type_name)
            if include_subtypes
            else model.ids_of_type(type_name)
        )
        return [Entity(model, id) for id in ids]

    def to_dataframe(
        self,
        type: str = "IfcProduct",
        *,
        psets: bool = True,
        qtos: bool = True,
        attributes: Sequence[str] = ("GlobalId", "Name"),
        include_subtypes: bool = True,
    ) -> "pandas.DataFrame":
        """A pandas ``DataFrame`` of the entities :meth:`by_type` returns:
        one row per entity, indexed by id, with a ``type`` column, one
        column per name in ``attributes``, and one ``"Set.Property"`` column
        per property (``psets``) and quantity (``qtos``) any row has.

        Cells hold the plain values of :attr:`Entity.psets`; a row without
        a column holds pandas' missing marker. Needs the ``pandas`` extra
        (``pip install 'openbim-ifc[pandas]'``), imported only here.
        """
        from .frames import to_dataframe

        return to_dataframe(
            self._as_model(), type, include_subtypes, tuple(attributes), psets, qtos
        )

    def _is_a(self, id: int, entity_type: str, type_name: str) -> bool:
        query = type_name.upper()
        if entity_type == query:
            return True
        model = self._as_model()
        key = (model.schema, entity_type, query)
        try:
            cache = self._subtypes
        except AttributeError:
            cache = {}
            object.__setattr__(self, "_subtypes", cache)
        if key not in cache:
            # The subtype relation is the schema's; any entity of this type
            # answers it for every other.
            cache[key] = id in model.ids_of_type_including_subtypes(type_name)
        return cache[key]
