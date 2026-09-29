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

- This crate is the geometric bridge for alignments, not a road or rail
  application. Product workflows (corridors, cross-sections, track
  design) and rendering policy stay out of it and belong to the
  application that composes it.
