//! The surface colour of a representation item or a material (#449);
//! see [`StyleIndex`] for the precedence and its citations.

use std::collections::BTreeMap;

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};
use crate::StyleView;

/// The colour a surface is painted with, and where it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SurfaceColour {
    /// `SurfaceColour`: red, green and blue, each in `[0, 1]`.
    pub rgb: [f64; 3],
    /// `Transparency` in `[0, 1]`, 0 opaque. `None` when the file states
    /// none, which the standard reads as opaque; IFC2X3 states it on
    /// `IfcSurfaceStyleRendering` only.
    pub transparency: Option<f64>,
    /// The `IfcSurfaceStyle` the colour is read from.
    pub surface_style: EntityId,
    /// Where the style was assigned.
    pub source: ColourSource,
}

/// How a surface style reached the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColourSource {
    /// An `IfcStyledItem` on the representation item; carries its id.
    StyledItem(EntityId),
    /// An `IfcPresentationLayerWithStyle` the item is on; carries its id.
    PresentationLayer(EntityId),
    /// The material's `IfcMaterialDefinitionRepresentation`.
    Material {
        /// The `IfcMaterial`.
        material: EntityId,
        /// The item-less `IfcStyledItem` in its `IfcStyledRepresentation`.
        styled_item: EntityId,
    },
}

/// Every style assignment of a model, indexed once, for colour lookups by
/// item and by material.
///
/// [`StyleView::resolve_item_style`](crate::StyleView::resolve_item_style)
/// scans the model per item, which is fine for one item and quadratic for
/// every item of every product; the index reads each `IfcStyledItem`,
/// `IfcPresentationLayerAssignment` and `IfcMaterialDefinitionRepresentation`
/// once.
///
/// # Precedence
///
/// A viewer paints a surface with the colour of an `IfcSurfaceStyle`'s
/// `IfcSurfaceStyleShading` (or its `IfcSurfaceStyleRendering` subtype).
/// The style reaches a representation item in one of three ways, and
/// [`StyleIndex`] answers each of them separately:
///
/// 1. **The item.** An `IfcStyledItem` whose `Item` is the representation
///    item ([`StyleIndex::item_colour`], [`ColourSource::StyledItem`]).
/// 2. **Its presentation layer.** An `IfcPresentationLayerWithStyle` whose
///    `AssignedItems` include it ([`StyleIndex::item_colour`],
///    [`ColourSource::PresentationLayer`]).
/// 3. **Its material.** The `IfcMaterialDefinitionRepresentation` of the
///    `IfcMaterial` the element is made of: its `IfcStyledRepresentation`s
///    hold `IfcStyledItem`s with no `Item` whose styles present the
///    material ([`StyleIndex::material_colour`], [`ColourSource::Material`]).
///
/// The precedence is that order. The item over the layer is the
/// standard's: "If the styled item has style information assigned [...]
/// then it overrides the style provided by the
/// IfcPresentationLayerAssignmentWithStyle" (IFC4 ADD2 TC1,
/// `IfcPresentationLayerWithStyle`, second note), and it is the cascade
/// [`StyleView::resolve_item_style`](crate::StyleView::resolve_item_style)
/// already applies. The material comes last because it is the general
/// case: `IfcStyledItem` "holds presentation style information for
/// products, either explicitly for an IfcGeometricRepresentationItem being
/// part of an IfcShapeRepresentation assigned to a product, or by assigning
/// presentation information to IfcMaterial being assigned as other
/// representation for a product" (IFC4 ADD2 TC1, `IfcStyledItem`), and
/// `IfcMaterialDefinitionRepresentation` "defines presentation information
/// relating to IfcMaterial" used at element occurrences and types (IFC4
/// ADD2 TC1, `IfcMaterialDefinitionRepresentation`; the entity is new in
/// IFC2x3 and unchanged in IFC4X3 ADD2). The standard states no order
/// between a layer style and a material style; this one is IfcOpenShell's
/// (`mapping.cpp`, `find_style` before the material fallback) and the
/// one #449 fixes.
///
/// The precedence is per colour, not per style: a level that states no
/// surface colour (a styled item with only a curve style, say) does not
/// hide the next one. The layer note says a styled item on a layer
/// "inherits the style information from the layer", omitting "its own
/// style information" for the styles it does not state.
///
/// Which `IfcMaterial` an element is made of (a layer of a set, a
/// constituent) is the material domain's question, not this crate's: the
/// caller resolves it and passes the material here. The `openbim-ifc`
/// facade joins the two (`product_colour`).
#[derive(Debug, Clone)]
pub struct StyleIndex<'m, 's> {
    view: StyleView<'m, 's>,
    /// Representation item -> the `IfcStyledItem`s naming it.
    direct: BTreeMap<EntityId, Vec<EntityId>>,
    /// Representation item -> the layer assignments listing it.
    layers: BTreeMap<EntityId, Vec<EntityId>>,
    /// `IfcMaterial` -> its `IfcMaterialDefinitionRepresentation`s.
    materials: BTreeMap<EntityId, Vec<EntityId>>,
}

impl<'m, 's> StyleIndex<'m, 's> {
    /// Index every style assignment in `view`'s model.
    ///
    /// # Errors
    ///
    /// The projection errors of a malformed `IfcStyledItem.Item`,
    /// `IfcPresentationLayerAssignment.AssignedItems` or
    /// `IfcMaterialDefinitionRepresentation.RepresentedMaterial`, as
    /// [`StyleView::resolve_item_style`](crate::StyleView::resolve_item_style)
    /// raises them.
    pub fn new(view: StyleView<'m, 's>) -> StyleResult<Self> {
        let mut index = Self {
            view,
            direct: BTreeMap::new(),
            layers: BTreeMap::new(),
            materials: BTreeMap::new(),
        };
        let schema = view.schema;
        let has_definition = schema
            .entity("IfcMaterialDefinitionRepresentation")
            .is_some();
        for (id, entity) in view.model.iter() {
            let type_name = &entity.type_name;
            if schema.is_a(type_name, "IfcStyledItem") {
                if let Some(item) = view.styled_item(id)?.item()? {
                    index.direct.entry(item).or_default().push(id);
                }
            } else if schema.is_a(type_name, "IfcPresentationLayerAssignment") {
                for item in view.presentation_layer(id)?.assigned_items()? {
                    index.layers.entry(item).or_default().push(id);
                }
            } else if has_definition
                && schema.is_a(type_name, "IfcMaterialDefinitionRepresentation")
            {
                let material = view
                    .record(id, "IfcMaterialDefinitionRepresentation")?
                    .required_ref("RepresentedMaterial", "IfcMaterial")?;
                index.materials.entry(material).or_default().push(id);
            }
        }
        // `Model::iter` is in id order already; the lists are sorted.
        Ok(index)
    }

    /// The view the index reads through.
    #[must_use]
    pub fn view(&self) -> StyleView<'m, 's> {
        self.view
    }

    /// The surface colour `item` states itself: its own `IfcStyledItem`
    /// first, then its presentation layers (see [the precedence](StyleIndex#precedence)).
    /// `None` when neither states a surface colour.
    ///
    /// # Errors
    ///
    /// [`StyleError::AmbiguousStyleAssignment`] for two `IfcStyledItem`s on
    /// `item`, as `resolve_item_style` refuses it;
    /// [`StyleError::AmbiguousSurfaceColour`] when one styled item, or the
    /// layers together, state two different surface colours; and the
    /// projection errors of a malformed style.
    pub fn item_colour(&self, item: EntityId) -> StyleResult<Option<SurfaceColour>> {
        if let Some(direct) = self.direct.get(&item) {
            if direct.len() > 1 {
                return Err(StyleError::AmbiguousStyleAssignment {
                    item,
                    count: direct.len(),
                });
            }
            let styled = direct[0];
            let styles = self
                .view
                .flatten_assignments(self.view.styled_item(styled)?.styles()?)?;
            if let Some(colour) =
                self.unique_colour(styled, &styles, ColourSource::StyledItem(styled))?
            {
                return Ok(Some(colour));
            }
        }
        let mut found = Vec::new();
        for &layer in self.layers.get(&item).map_or(&[][..], Vec::as_slice) {
            let styles = self
                .view
                .flatten_assignments(self.view.presentation_layer(layer)?.layer_styles()?)?;
            if let Some(colour) =
                self.unique_colour(layer, &styles, ColourSource::PresentationLayer(layer))?
            {
                found.push(colour);
            }
        }
        agree(item, found)
    }

    /// The surface colour `material`'s `IfcMaterialDefinitionRepresentation`
    /// states, over every one of its `IfcStyledRepresentation`s (one per
    /// representation context). `None` when the material has no
    /// representation or none states a surface colour.
    ///
    /// # Errors
    ///
    /// [`StyleError::AmbiguousSurfaceColour`] when its styled items state
    /// two different surface colours (in two contexts, say);
    /// [`StyleError::ReferenceType`] for a representation that is no
    /// `IfcStyledRepresentation` or an item that is no `IfcStyledItem`
    /// (both rules of the standard); and the projection errors of a
    /// malformed style.
    pub fn material_colour(&self, material: EntityId) -> StyleResult<Option<SurfaceColour>> {
        let mut found = Vec::new();
        for &definition in self.materials.get(&material).map_or(&[][..], Vec::as_slice) {
            let representations = self
                .view
                .record(definition, "IfcMaterialDefinitionRepresentation")?
                .required_refs("Representations", "IfcStyledRepresentation", 1, None)?;
            for representation in representations {
                let items = self
                    .view
                    .record(representation, "IfcStyledRepresentation")?
                    .required_refs("Items", "IfcStyledItem", 1, None)?;
                for styled in items {
                    let styles = self
                        .view
                        .flatten_assignments(self.view.styled_item(styled)?.styles()?)?;
                    let source = ColourSource::Material {
                        material,
                        styled_item: styled,
                    };
                    if let Some(colour) = self.unique_colour(styled, &styles, source)? {
                        found.push(colour);
                    }
                }
            }
        }
        agree(material, found)
    }

    /// The one surface colour among `styles`, assigned by `owner`.
    fn unique_colour(
        &self,
        owner: EntityId,
        styles: &[EntityId],
        source: ColourSource,
    ) -> StyleResult<Option<SurfaceColour>> {
        let mut found = Vec::new();
        for &style in styles {
            if let Some((rgb, transparency)) = self.surface_colour(style)? {
                found.push(SurfaceColour {
                    rgb,
                    transparency,
                    surface_style: style,
                    source,
                });
            }
        }
        agree(owner, found)
    }

    /// `style`'s shading colour and transparency, when it is an
    /// `IfcSurfaceStyle` with an `IfcSurfaceStyleShading` element.
    fn surface_colour(&self, style: EntityId) -> StyleResult<Option<([f64; 3], Option<f64>)>> {
        let view = self.view;
        let entity = view
            .model
            .get(style)
            .ok_or(StyleError::UnknownEntity { id: style })?;
        if !view.schema.is_a(&entity.type_name, "IfcSurfaceStyle") {
            return Ok(None);
        }
        for element in view.surface_style(style)?.elements()? {
            let Some(element_entity) = view.model.get(element) else {
                continue;
            };
            if !view
                .schema
                .is_a(&element_entity.type_name, "IfcSurfaceStyleShading")
            {
                continue;
            }
            let shading = view.surface_style_shading(element)?;
            let rgb = view.colour_rgb(shading.surface_colour()?)?.channels()?;
            // IFC2X3 states `Transparency` on the rendering subtype only;
            // IFC4 moved it up to the shading. Read it where the release
            // declares it.
            let transparency = if view
                .schema
                .is_a(&element_entity.type_name, "IfcSurfaceStyleRendering")
            {
                view.surface_style_rendering(element)?.transparency()?
            } else {
                shading.transparency()?
            };
            return Ok(Some((rgb, transparency)));
        }
        Ok(None)
    }
}

/// The first of `found` when every colour agrees (same RGB and
/// transparency), `None` when empty, else ambiguous.
fn agree(owner: EntityId, found: Vec<SurfaceColour>) -> StyleResult<Option<SurfaceColour>> {
    let Some(first) = found.first().copied() else {
        return Ok(None);
    };
    if found
        .iter()
        .all(|c| c.rgb == first.rgb && c.transparency == first.transparency)
    {
        return Ok(Some(first));
    }
    let mut styles: Vec<EntityId> = found.iter().map(|c| c.surface_style).collect();
    styles.sort_unstable();
    styles.dedup();
    Err(StyleError::AmbiguousSurfaceColour { owner, styles })
}
