# ifc-alignment

IFC4X3 linear positioning: `IfcAlignment` with its horizontal, vertical and
cant layouts, referents and stationing, and linear placement. Alignment
curves are lowered exactly into the format-neutral Axiolid geometry model;
nothing is tessellated or numerically integrated. Transition spirals are
curvature laws, a `CUBIC` is its cubic parabola trimmed by arc length, a
vertical arc is the circle itself, and cant makes the centreline a banked
curve. A form the file does not determine (a vertical `CLOTHOID`, whose
segment states no curvature) is a typed refusal, and the open gaps are
listed on the [capabilities page](https://openbimrs.github.io/ifc/capabilities).

```bash
cargo add ifc-alignment
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`alignment` feature.

- API documentation: [docs.rs/ifc-alignment](https://docs.rs/ifc-alignment)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-alignment)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- A vertical profile read from a file is checked at the seam tolerance the
  file declares: `IfcGeometricRepresentationContext.Precision` (3D, coarsest,
  in the project length unit, capped at 1 mm). Exporters that round stations
  and heights to their stated precision are accepted; a step beyond it is
  refused. Without a declared precision only floating-point rounding is
  tolerated (`SeamTolerance`, `vertical_profile_law`).
- A grade break at a height-continuous vertical seam is legal IFC4.3 and
  accepted; the piecewise elevation law carries it exactly, and
  `VerticalLayout::seams` reports it. `require_tangential` asks for a
  tangent profile explicitly.
- The zero-length segment IFC4.3 requires at the end of every layout adds
  no geometry; its start is checked against the layout's end like any
  seam. A zero-length segment anywhere else is refused.
- Cant is exact data per station (`CantLayout::frame_at_distance`: rail
  heights, cant, bank angle `arcsin(D / b)`, rotation-point elevation and
  the section frame), and the cant-carrying centreline
  (`IfcSegmentedReferenceCurve`) is a `Curve3::Banked`
  (`lower_segmented_reference_curve`): the gradient curve, the cant law
  `D = left - right` and the pivot `(left + right) / 2`, one piece per cant
  segment in the IFC4.3 base formula, rotating the section about the 3D
  tangent by `arcsin(D / b)` (`BankConvention::TangentRotation`, the angle
  reading IFC4.3 states). Both read a Viennese bend as the standard writes
  it, for the section's bank angle, with the rails `D / 2` either side of
  a rotation point that stays put; a pivot that moves through a Viennese
  bend is refused by both (#364): it would follow the bank angle, which a
  height-form pivot law cannot carry.
- A multi-segment horizontal layout elevates as one exact plan curve: the
  first segment's start frame plus one curvature piece per segment, or an
  arc-length chain when it holds a `CUBIC`. Seams are checked in closed
  form: a heading kink after a segment with a curvature law, or a position
  gap after a line or arc, is refused; the position after a transition
  spiral, and the position and heading after a `CUBIC`, are reported as
  authored rather than verified.
- A `CUBIC` is IFC4.3's `y = x^3 / (6 R L)` leaving a straight, with
  `SegmentLength` along the curve: an exact cubic Bezier trimmed at
  `TrimSelector::ArcLength`, the inverse left to the kernel. A `CUBIC` that
  starts curved has no formula in the standard and is refused.
- A vertical `CIRCULARARC` is `ElevationLaw::CircularArc` from
  `StartHeight`, `StartGradient` and the signed `RadiusOfCurvature`
  (positive is a sag), not the EN 13803 parabola that approximates it.

- This crate is the geometric bridge for alignments, not a road or rail
  application. Product workflows (corridors, cross-sections, track
  design) and rendering policy stay out of it and belong to the
  application that composes it.
