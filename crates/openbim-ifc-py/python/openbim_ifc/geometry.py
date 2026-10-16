"""Geometry per product (#328, #367, ADR 0021). All frozen dataclasses.

:meth:`IfcModel.product_placements` gives each product's world placement
and the Body representation a viewer draws;
:meth:`IfcModel.product_geometry` its Body as Axiolid's exact geometry
graph, serialised in Axiolid's wire format 1.0 to 1.4 (JSON ``str`` or CBOR
``bytes``) for a host's own kernel; :meth:`IfcModel.product_meshes`
gives its triangles, from a wheel built with the ``mesh`` feature. A
product that cannot be placed or meshed carries a :class:`GeometryRefusal`
instead of failing the call.

A ``transform`` is a 4x4 column-major matrix in metres, as a tuple of 16
floats: three basis columns, then the origin. A mesh's ``positions`` are
relative to it, so the world position of a vertex is ``transform`` applied
to it; ``numpy.frombuffer(mesh.positions, numpy.float32)`` reads them
without a copy. Each mesh carries its :class:`SurfaceColour` (#449): its
representation item's ``IfcStyledItem``, else its styled presentation
layer, else its material's ``IfcMaterialDefinitionRepresentation``; and its
``closure`` (#458): ``"solid"``, ``"surface"`` or ``"unknown"``.

:meth:`IfcModel.product_curves` gives the curves of a curve representation
(``Axis``, ``FootPrint``, an annotation's) as :class:`ProductCurves` (#458):
one ``array('f')`` of positions relative to ``transform``, every polyline's
vertices in turn, and a :class:`ProductPolyline` per curve naming its slice
and its certified chord bound.
"""

from __future__ import annotations

from array import array
from dataclasses import dataclass
from typing import Any, Dict, Optional, Tuple, Union

Matrix4 = Tuple[float, ...]


@dataclass(frozen=True)
class GeometryRefusal:
    """Why one product has no placement, representation, graph or mesh.

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


#: The envelope's ``format`` entry.
GEOMETRY_FORMAT = "axiolid-geometry-graph"
#: The newest wire format version this build writes. An envelope's
#: ``version`` is the lowest its content needs: ``"1.0"``; ``"1.1"`` when
#: a station carries a seam-snapping window (#423), ``"1.2"`` when a run's
#: sections do (#429), ``"1.3"`` when an offset run continues to a basis
#: end the kernel measures (#433), ``"1.4"`` when a polygonal bounded
#: half-space clips (#515).
GEOMETRY_FORMAT_VERSION = "1.4"


@dataclass(frozen=True)
class ProductGeometry:
    """One product's Body as Axiolid's neutral geometry graph.

    ``payload`` is the wire envelope, a ``str`` for ``encoding`` ``"json"``
    (``json.loads`` reads it) and ``bytes`` for ``"cbor"``. The graph is in
    world coordinates, metres; ``transform`` is the product's placement,
    already applied, never to be applied again. No payload and no refusal:
    a product with no Body representation.
    """

    id: int
    global_id: Optional[str]
    type_name: str
    transform: Optional[Matrix4]
    encoding: str
    payload_size: int
    refusal: Optional[GeometryRefusal]
    payload: Optional[Union[str, bytes]]


@dataclass(frozen=True)
class SurfaceColour:
    """The surface colour a mesh is painted with (#449).

    ``red``, ``green`` and ``blue`` are in ``[0, 1]``; ``transparency`` too,
    0 opaque, and ``None`` when the file states none (which the standard
    reads as opaque). ``source`` is ``item``, ``layer`` or ``material``, in
    that precedence; ``source_entity`` the ``IfcStyledItem``, the layer
    assignment or the ``IfcMaterial``; ``surface_style`` the
    ``IfcSurfaceStyle``.
    """

    red: float
    green: float
    blue: float
    transparency: Optional[float]
    source: str
    source_entity: int
    surface_style: int


@dataclass(frozen=True)
class ProductMesh:
    """One product's Body as triangles.

    ``positions`` holds ``x, y, z`` per vertex (``array('f')``, metres,
    relative to ``transform``), ``indices`` three vertex indices per
    triangle (``array('I')``). Empty arrays and no refusal: a product with
    no Body representation.

    ``colour`` is the colour the Body's items agree on (#449), ``None``
    when nothing colours the product or ``colour_refusal`` says why it is
    not resolved: code ``ambiguous`` (two styled items or two colours at
    one level, a layer or profile set of several materials, a constituent
    set no shape aspect resolves, or Body items of different colours) or a
    shared geometry code. A colour never refuses the mesh.
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
    colour: Optional[SurfaceColour] = None
    colour_refusal: Optional[GeometryRefusal] = None
    closure: str = "unknown"


@dataclass(frozen=True)
class ProductPolyline:
    """One curve of a :class:`ProductCurves`, as a slice of its positions:
    vertices ``first_vertex`` to ``first_vertex + vertex_count``, in the
    curve's direction. ``closed``: the last vertex repeats the first.
    ``chord_bound`` is the certified bound, in metres, on how far the exact
    curve lies from the polyline (``0.0`` for a polyline the file authored);
    ``chord_requested`` the budget, one millimetre. ``curve`` is the IFC
    curve, when the lowering names one."""

    curve: Optional[int]
    first_vertex: int
    vertex_count: int
    closed: bool
    chord_bound: float
    chord_requested: float


@dataclass(frozen=True)
class ProductCurves:
    """One product's curves as polylines (#458).

    ``positions`` holds ``x, y, z`` per vertex (``array('f')``, metres,
    relative to ``transform``), every polyline's vertices in turn; each of
    ``polylines`` names its slice. No polylines and no refusal: a product
    without the representation, or one with no curves.
    """

    id: int
    global_id: Optional[str]
    type_name: str
    transform: Optional[Matrix4]
    vertex_count: int
    polylines: Tuple[ProductPolyline, ...]
    refusal: Optional[GeometryRefusal]
    positions: "array[float]"


def _refusal(data: Optional[Dict[str, Any]]) -> Optional[GeometryRefusal]:
    if data is None:
        return None
    return GeometryRefusal(data["code"], data["entity"], data["message"])


def _colour(data: Optional[Dict[str, Any]]) -> Optional[SurfaceColour]:
    if data is None:
        return None
    return SurfaceColour(**{k: v for k, v in data.items() if k != "_record"})


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


def _graph(data: Dict[str, Any]) -> ProductGeometry:
    return ProductGeometry(
        id=data["id"],
        global_id=data["global_id"],
        type_name=data["type_name"],
        transform=_matrix(data["transform"]),
        encoding=data["encoding"],
        payload_size=data["payload_size"],
        refusal=_refusal(data["refusal"]),
        payload=data["payload"],
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
        colour=_colour(data["colour"]),
        colour_refusal=_refusal(data["colour_refusal"]),
        closure=data["closure"],
    )


def _curves(data: Dict[str, Any]) -> ProductCurves:
    positions = array("f")
    positions.frombytes(data["positions"])
    return ProductCurves(
        id=data["id"],
        global_id=data["global_id"],
        type_name=data["type_name"],
        transform=_matrix(data["transform"]),
        vertex_count=data["vertex_count"],
        polylines=tuple(
            ProductPolyline(**{k: v for k, v in row.items() if k != "_record"})
            for row in data["polylines"]
        ),
        refusal=_refusal(data["refusal"]),
        positions=positions,
    )


__all__ = [
    "GEOMETRY_FORMAT",
    "GEOMETRY_FORMAT_VERSION",
    "GeometryRefusal",
    "ProductCurves",
    "ProductGeometry",
    "ProductMesh",
    "ProductPlacement",
    "ProductPolyline",
    "SelectedRepresentation",
    "SurfaceColour",
]
