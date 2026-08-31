# NURBS regression fixtures

`ifc4_rational_bspline_curve_surface.ifc` is a minimal synthetic OpenBIM.rs
fixture. Its schema basis is the IFC4 ADD2 TC1 declarations for both polynomial
and rational B-spline curve/surface-with-knots entities, consulted read-only and
mapped against this repository's typed curve/surface slot layouts. The
normative schema is not redistributed.

The curve is a rational quadratic quarter circle; its midpoint is
`(sqrt(1/2), sqrt(1/2), 0)`.
The surface is a weighted degree-(2,1) tensor patch; its midpoint is
`(0.8, 1.2, 0.4)`. Its repeated first/last U rows make the authored U-closure
flag geometrically consistent, while V remains open. The polynomial siblings
exercise an actually closed curve and a V-closed/U-open surface.
The rational records carry explicit compact knots, multiplicities, control points, and weights.
Polynomial sibling records prove the same dispatch does not invent weights.
`invalid_abstract_base_splines.ifc` is a separate, deliberately invalid file.
`IfcBSplineCurve` and `IfcBSplineSurface` are ABSTRACT SUPERTYPE in IFC4, so a
conforming file never instantiates them; `ifcopenshell.validate` reports both
as abstract. They are kept because real exporters emit them and lowering must
reject them with a typed report instead of synthesizing the knots the concrete
`*WithKnots` subtypes carry. They live apart so the valid fixture above stays
schema-clean and usable as validation ground truth.

Origin: generated in this repository. License: AGPL-3.0-or-later.
