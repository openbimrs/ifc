#!/usr/bin/env python3
"""Emit IfcOpenShell reference geometry for the fixture corpus."""
import json, sys, time, pathlib
import numpy as np
import ifcopenshell
import ifcopenshell.geom as geom
import ifcopenshell.util.shape as ushape


def volume(V, F):
    """Signed volume by the divergence theorem, centred on the centroid.

    Centring is required, not cosmetic: the terms scale with the cube of the
    distance to the origin while the true volume does not, so on survey
    coordinates (~1.5e6) a cubic-metre answer is reconstructed from
    differences sixteen digits down and comes out several percent low.
    """
    centre = V.mean(axis=0)
    tri = V[F] - centre
    cross = np.cross(tri[:, 1], tri[:, 2])
    return float((tri[:, 0] * cross).sum(axis=1).sum() / 6.0)


def edge_manifold(F):
    """Every directed edge exactly once, and paired with its opposite."""
    seen = {}
    for a, b, c in F:
        for e in ((a, b), (b, c), (c, a)):
            if e in seen:
                return False
            seen[e] = True
    return all((b, a) in seen for (a, b) in seen)


def main(paths):
    settings = geom.settings()
    for path in paths:
        name = pathlib.Path(path).name
        try:
            model = ifcopenshell.open(path)
        except Exception as exc:
            print(json.dumps({"file": name, "error": f"open: {type(exc).__name__}"}))
            continue
        for product in model.by_type("IfcProduct"):
            if not getattr(product, "Representation", None):
                continue
            record = {"file": name, "id": product.id(), "type": product.is_a()}
            start = time.perf_counter()
            try:
                shape = geom.create_shape(settings, product)
            except Exception as exc:
                record["error"] = f"{type(exc).__name__}: {str(exc)[:120]}"
                print(json.dumps(record))
                continue
            record["ms"] = (time.perf_counter() - start) * 1000.0
            g = shape.geometry
            # These numpy views alias C++-owned memory freed when `shape`
            # drops. Copy before the borrow can dangle: without this the
            # index buffer reads freed memory and yields out-of-range
            # triangles, silently corrupting every derived number.
            V = np.array(ushape.get_vertices(g), dtype=float, copy=True)
            F = np.array(ushape.get_faces(g), dtype=np.int64, copy=True)
            if len(F) and F.max() >= len(V):
                record["error"] = "corrupt index buffer"
                print(json.dumps(record))
                continue
            record["triangles"] = int(F.shape[0])
            record["volume"] = volume(V, F)
            record["manifold"] = edge_manifold(F)
            print(json.dumps(record))


if __name__ == "__main__":
    main(sys.argv[1:])
