# ifc-alignment

IFC4X3 linear positioning: `IfcAlignment` with its horizontal, vertical and
cant layouts, referents and stationing, and linear placement. Alignment
curves are lowered exactly into the format-neutral Axiolid geometry model;
nothing is tessellated or numerically integrated. A segment form that cannot
be lowered exactly yet (the cubic spiral, circular and clothoid vertical
curves) is a typed refusal, and the open gaps are listed on the
[capabilities page](https://openbimrs.github.io/ifc/capabilities).

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
  the section frame). The cant-carrying centreline
  (`IfcSegmentedReferenceCurve`) is a typed refusal: the neutral curve
  vocabulary has no roll law yet (#93).
- A multi-segment horizontal layout elevates as one exact plan curve: the
  first segment's start frame plus one curvature piece per segment. Seams
  are checked in closed form: a heading kink, or a position gap after a
  line or arc, is refused; the position after a transition spiral, a
  Fresnel-type integral, is reported as authored rather than verified.

- This crate is the geometric bridge for alignments, not a road or rail
  application. Product workflows (corridors, cross-sections, track
  design) and rendering policy stay out of it and belong to the
  application that composes it.
