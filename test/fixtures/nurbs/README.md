# NURBS regression fixtures

`ifc4_rational_bspline_curve_surface.ifc` is a minimal synthetic OpenBIM.rs fixture.
Schema basis: IFC4 ADD2 TC1 declarations for the rational B-spline curve/surface-with-knots entities.
The normative schema was consulted read-only and is not redistributed.

The curve is a rational quadratic quarter circle; its midpoint is `(sqrt(1/2), sqrt(1/2), 0)`.
The surface is a weighted bilinear patch; its midpoint is `(0.8, 1.2, 0.4)`.
The rational records carry explicit compact knots, multiplicities, control points, and weights.
Polynomial sibling records prove the same dispatch does not invent weights.

Origin: generated in this repository. License: MIT.
