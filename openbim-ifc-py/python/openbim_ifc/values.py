"""IFC attribute values: one frozen dataclass per kind.

Each class maps to exactly one STEP form:

=========  ==========================  ====================
Class      Fields                      STEP
=========  ==========================  ====================
Null       --                          ``$``
Derived    --                          ``*``
Bool       ``value: bool``             ``.T.`` / ``.F.``
Unknown    --                          ``.U.``
Integer    ``value: int``              ``42``
Real       ``value: float``            ``2.5``
Text       ``value: str``              ``'Wall'``
Binary     ``value: str`` (hex)        ``"0123ABC"``
Enum       ``value: str``              ``.ELEMENT.``
Ref        ``id: int``                 ``#42``
List       ``items: tuple[Value, ...]``  ``(1,2)``
Typed      ``type: str, value: Value`` ``IFCLABEL('x')``
=========  ==========================  ====================

``Unknown`` is its own class, not ``Bool`` with ``None``, so no truthiness
check can mistake it for ``.F.``. Python ints are unbounded, so 64-bit IFC
integers need no special type; values outside 64 bits are refused on write.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Tuple, Union


@dataclass(frozen=True)
class Null:
    """``$`` -- the attribute is not set."""


@dataclass(frozen=True)
class Derived:
    """``*`` -- derived in a supertype; distinct from ``$``."""


@dataclass(frozen=True)
class Bool:
    """``.T.`` or ``.F.``."""

    value: bool


@dataclass(frozen=True)
class Unknown:
    """``.U.`` -- the third logical state."""


@dataclass(frozen=True)
class Integer:
    """An integer literal (64-bit)."""

    value: int


@dataclass(frozen=True)
class Real:
    """A real literal; must be finite."""

    value: float


@dataclass(frozen=True)
class Text:
    """A string."""

    value: str


@dataclass(frozen=True)
class Binary:
    """A binary literal, as its hex digits."""

    value: str


@dataclass(frozen=True)
class Enum:
    """An enumeration constant, without dots: ``Enum("ELEMENT")``."""

    value: str


@dataclass(frozen=True)
class Ref:
    """A reference to entity ``#id``."""

    id: int


@dataclass(frozen=True)
class List:
    """An aggregate (list, set, array or bag)."""

    items: Tuple["Value", ...]

    def __post_init__(self) -> None:
        # Accept any iterable but store a tuple, so the value stays hashable
        # and immutable like every other kind.
        object.__setattr__(self, "items", tuple(self.items))


@dataclass(frozen=True)
class Typed:
    """A typed wrapper such as ``IFCLENGTHMEASURE(2.5)``."""

    type: str
    value: "Value"


Value = Union[Null, Derived, Bool, Unknown, Integer, Real, Text, Binary, Enum, Ref, List, Typed]


_SINGLETONS = {"null": Null(), "derived": Derived(), "unknown": Unknown()}
_SCALARS = {"bool": Bool, "integer": Integer, "real": Real, "text": Text, "binary": Binary, "enum": Enum}


def from_wire(data: Dict[str, Any]) -> Value:
    """Build a value from the native layer's tagged dict."""
    kind = data["kind"]
    if kind in _SINGLETONS:
        return _SINGLETONS[kind]
    if kind in _SCALARS:
        return _SCALARS[kind](data["value"])
    if kind == "ref":
        return Ref(data["id"])
    if kind == "list":
        return List(tuple(from_wire(item) for item in data["items"]))
    if kind == "typed":
        return Typed(data["type"], from_wire(data["value"]))
    raise ValueError(f"unknown kind {kind!r}")


def to_wire(value: Value) -> Dict[str, Any]:
    """The tagged dict the native layer validates and stores.

    Anything that is not one of the value classes is refused here with a
    ``TypeError``, before reaching the model: passing a bare ``3`` or ``"x"``
    would otherwise force a guess between Integer/Real or Text/Enum.
    """
    for kind, instance in _SINGLETONS.items():
        if type(value) is type(instance):
            return {"kind": kind}
    for kind, cls in _SCALARS.items():
        if type(value) is cls:
            return {"kind": kind, "value": value.value}
    if type(value) is Ref:
        return {"kind": "ref", "id": value.id}
    if type(value) is List:
        return {"kind": "list", "items": [to_wire(item) for item in value.items]}
    if type(value) is Typed:
        return {"kind": "typed", "type": value.type, "value": to_wire(value.value)}
    raise TypeError(
        f"expected an openbim_ifc value (Null, Text, Ref, ...), got {type(value).__name__}"
    )
