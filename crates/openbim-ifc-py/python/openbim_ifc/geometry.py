"""Geometry per product (#328, ADR 0021). All frozen dataclasses.

:meth:`IfcModel.product_placements` gives each product's world placement
and the Body representation a viewer draws; :meth:`IfcModel.product_meshes`
gives its triangles, from a wheel built with the ``mesh`` feature. A
product that cannot be placed or meshed carries a :class:`GeometryRefusal`
instead of failing the call.

A ``transform`` is a 4x4 column-major matrix in metres, as a tuple of 16
floats: three basis columns, then the origin. A mesh's ``positions`` are
relative to it, so the world position of a vertex is ``transform`` applied
to it; ``numpy.frombuffer(mesh.positions, numpy.float32)`` reads them
without a copy.
"""

from __future__ import annotations

from array import array
from dataclasses import dataclass
from typing import Any, Dict, Optional, Tuple

Matrix4 = Tuple[float, ...]


@dataclass(frozen=True)
class GeometryRefusal:
    """Why one product has no placement, representation or mesh.

    ``code`` is ``unsupported``, ``invalid-model``, ``missing-reference``
    or ``budget-exceeded``; ``entity`` the entity at fault, if named.
    """

    code: str
    entity: Optional[int]
    message: str


@dataclass(frozen=True)
class SelectedRepresentation:
    """The ``IfcShapeRepresentation`` selected as a product's Body, and its
    context (``ContextOfItems``)."""

    id: int
    identifier: Optional[str]
    representation_type: Optional[str]
    context: Optional[int]
    context_type: Optional[str]
    context_identifier: Optional[str]
    target_view: Optional[str]


@dataclass(frozen=True)
class ProductPlacement:
    """One product's world placement and selected Body.

    ``representation`` is ``None`` for a product with no solid
    representation (an axis only), which is not a refusal.
    """

    id: int
    global_id: Optional[str]
    type_name: str
    transform: Optional[Matrix4]
    representation: Optional[SelectedRepresentation]
    refusal: Optional[GeometryRefusal]


@dataclass(frozen=True)
class ProductMesh:
    """One product's Body as triangles.

    ``positions`` holds ``x, y, z`` per vertex (``array('f')``, metres,
    relative to ``transform``), ``indices`` three vertex indices per
    triangle (``array('I')``). Empty arrays and no refusal: a product with
    no Body representation.
    """

    id: int
    global_id: Optional[str]
    type_name: str
    transform: Optional[Matrix4]
    vertex_count: int
    triangle_count: int
    refusal: Optional[GeometryRefusal]
    positions: "array[float]"
    indices: "array[int]"


def _refusal(data: Optional[Dict[str, Any]]) -> Optional[GeometryRefusal]:
    if data is None:
        return None
    return GeometryRefusal(data["code"], data["entity"], data["message"])


def _matrix(data: Optional[Any]) -> Optional[Matrix4]:
    return None if data is None else tuple(data)


def _placement(data: Dict[str, Any]) -> ProductPlacement:
    body = data["representation"]
    return ProductPlacement(
        id=data["id"],
        global_id=data["global_id"],
        type_name=data["type_name"],
        transform=_matrix(data["transform"]),
        representation=None
        if body is None
        else SelectedRepresentation(**{k: v for k, v in body.items() if k != "_record"}),
        refusal=_refusal(data["refusal"]),
    )


def _mesh(data: Dict[str, Any]) -> ProductMesh:
    positions = array("f")
    positions.frombytes(data["positions"])
    indices = array("I")
    indices.frombytes(data["indices"])
    return ProductMesh(
        id=data["id"],
        global_id=data["global_id"],
        type_name=data["type_name"],
        transform=_matrix(data["transform"]),
        vertex_count=data["vertex_count"],
        triangle_count=data["triangle_count"],
        refusal=_refusal(data["refusal"]),
        positions=positions,
        indices=indices,
    )


__all__ = ["GeometryRefusal", "ProductMesh", "ProductPlacement", "SelectedRepresentation"]
