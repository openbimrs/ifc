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
Convention-only base curve and surface records prove lowering rejects absent
explicit knots instead of synthesizing them.

Origin: generated in this repository. License: MIT.
