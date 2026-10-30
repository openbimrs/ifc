# Changelog -- ifc-style

All notable changes to the `ifc-style` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added (#597)

- `StyleIndex::material_fill(material, units)`: the cut fill a material's `IfcMaterialDefinitionRepresentation` states, as a `MaterialFill` (fill area style, styled item, name, `ModelOrDraughting`, background colour) with one `HatchLine` per `IfcFillAreaStyleHatching`: angle in radians; spacing, along-line shift, origin, pattern start, dashes and width in metres; line colour. An `IfcVector` (IFC2X3: `IfcOneDirectionRepeatFactor`) distance splits into the perpendicular spacing and the shift. Fills over several contexts must draw the same, else `StyleError::AmbiguousFill`. Tiles, externally defined hatch styles, predefined curve fonts other than 'continuous' and 'by layer' deferrals are `StyleError::UnsupportedFill`.
- `StyleUnits`: the project's metres per length unit and radians per plane angle unit, resolved by the caller; a measure read without its factor is `StyleError::UnitUndeclared`, a bad factor `StyleError::InvalidUnits`.

### Fixed (#597)

- `create_fill_area_style_hatching` writes a length distance as `IFCPOSITIVELENGTHMEASURE(..)`, the select member, not a bare real; in IFC2X3 `HatchLineDistance::Vector` takes an `IfcOneDirectionRepeatFactor` and refuses an `IfcVector` and an `IfcTwoDirectionRepeatFactor` (WR21). `HatchLineAngle` is documented in the project's plane angle unit.

Semver: additive (`StyleError` is `#[non_exhaustive]`), a minor release while 0.x.

## [0.6.3] - 2026-10-29

### Changed (#608)

- `StyleIndex` finds an item's styled items and layers, and a material's
  representations, through referrers: the model's reverse index when built,
  else one pass over the style records only. A malformed assignment refuses
  only the lookups that reach it, and `StyleIndex::new` no longer fails.
  Several records on one item or material are read in ascending id.

## [0.6.2] - 2026-10-28

### Changed

- Copyright holder in NOTICE and licence texts is rheonda.

## [0.6.1] - 2026-10-28

### Changed

- Licence: `AGPL-3.0-or-later OR LicenseRef-Commercial` (was `AGPL-3.0-or-later`); a commercial licence is available from the copyright holder. See LICENSING.md.

Semver: patch.

## [0.6.0] - 2026-10-27

## [0.5.1] - 2026-10-20

### Fixed (#514)

Semver: patch. No public item changes.

- `FillAreaStyle::fill_styles` accepts a solid colour fill. It listed the
  `IfcColour` select itself as a member of `IfcFillStyleSelect`, and a
  select name never matches an instance's entity type, so every
  `IfcColourRgb` (and `IfcDraughtingPreDefinedColour`) fill was refused
  with `StyleError::ReferenceType`. It now lists `IfcColour`'s own members,
  `IfcColourSpecification` and `IfcPreDefinedColour`, as every bundled
  release declares them; the colour then reads through
  `StyleView::colour_rgb`. A non-colour member is still refused.

## [0.5.0] - 2026-10-13

### Added (#449)

- `StyleIndex`: every `IfcStyledItem`, `IfcPresentationLayerAssignment`
  and `IfcMaterialDefinitionRepresentation` of a model indexed once, with
  `item_colour(item)` (the item's own `IfcStyledItem`, then its styled
  presentation layers) and `material_colour(material)` (the item-less
  `IfcStyledItem`s of the material's `IfcStyledRepresentation`s). Both
  return a `SurfaceColour`: RGB, transparency (`None` when unstated),
  the `IfcSurfaceStyle` and a `ColourSource` (`StyledItem`,
  `PresentationLayer`, `Material`). The precedence, item over layer
  over material, is documented and cited (IFC4 ADD2 TC1
  `IfcPresentationLayerWithStyle`, `IfcStyledItem`,
  `IfcMaterialDefinitionRepresentation`). It is applied per colour: a
  level that states no surface colour does not hide the next. IFC2X3
  reads its `IfcPresentationStyleAssignment` wrappers, and its
  transparency from `IfcSurfaceStyleRendering`.
- `StyleError::AmbiguousSurfaceColour`: one level stating two different
  surface colours (two surface styles in one styled item, two styled
  layers, a material whose styled representations disagree).

`resolve_item_style` is unchanged.

Semver: additive (`StyleError` is `#[non_exhaustive]`), a minor release
while 0.x.

## [0.4.0] - 2026-09-29

### Changed (breaking)

- `AppearanceSupport` is `#[non_exhaustive]`: a match needs a wildcard arm;
  `AppearanceDeclaration` is `#[non_exhaustive]`.
- Every authoring draft is `#[non_exhaustive]`, so a struct literal no
  longer compiles outside the crate. Each gains `new(required…)` and one
  builder setter per other field, named after the field and taking the
  unwrapped value: `BlobTextureDraft::new(raster_format, raster_code)`,
  `TextModelDraft::new()`, `LightSourceDraft::new(light_colour)`,
  `AnnotationDraft::new(global_id)`,
  `TextLiteralDraft::new(literal, placement, path)`,
  `TextLiteralWithExtentDraft::new(literal, placement, path, extent,
  box_alignment)`, `AnnotationFillAreaDraft::new(outer_boundary)`,
  `CurveStyleDraft::new()`,
  `PixelTextureDraft::new(width, height, colour_components, pixel)`,
  `ColourRgbDraft::new(red, green, blue)`,
  `SurfaceStyleShadingDraft::new(surface_colour)`,
  `SurfaceStyleDraft::new(side, elements)`, `StyledItemDraft::new(styles)`,
  `PresentationLayerDraft::new(name, assigned_items)`,
  `SurfaceStyleRenderingDraft::new(surface_colour, reflectance_method)` and
  `ImageTextureDraft::new(url_reference)`. Fields stay public.

### Added

- `StyleError::UnsupportedSchema`: `StyledItem::styles` refuses a
  recognised release it is not verified for instead of applying the
  IFC4X3 rule to it.

### Changed

- Links no bundled schema table itself: every entry point takes the
  `Schema` from the caller. A consumer that used a table through this
  crate's dependency (`ifc_schema::ifc4()`) enables it on its own
  `ifc-schema` dependency (default features bundle every release).
- `StyledItem::styles` reads IFC4X1 and IFC4X2 like IFC4: both declare
  `Styles` over `IfcStyleAssignmentSelect`, which still admits
  `IfcPresentationStyleAssignment`; only IFC4X3 narrowed it. Pinned
  against both bundled tables.

## [0.3.0] - 2026-09-23

### Added

- `IndexedTextureMap::triangle_coordinates(triangle_count)` resolves an
  `IfcIndexedTriangleTextureMap` to `(s, t)` coordinates for each triangle
  corner, in `CoordIndex` order. Corners, not vertices: one position may
  carry a different coordinate in each triangle that uses it. A shorter
  `TexCoordIndex` covers only the leading triangles, a longer one is an
  error, and an omitted one returns `None` because the schema does not
  define it (#30).

### Fixed

- **Breaking:** `IndexedTextureMap::maps` now returns `Vec<EntityId>`.
  `Maps` is `LIST [1:?] OF IfcSurfaceTexture`, but it was read as a single
  reference, so it returned an error on every conforming file, including
  those this crate writes itself.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-style-v0.6.3...HEAD
[0.6.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.6.3
[0.6.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.6.2
[0.6.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.6.1
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.6.0
[0.5.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.5.1
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-style-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
