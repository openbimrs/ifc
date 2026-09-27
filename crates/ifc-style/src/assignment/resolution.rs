//! Deterministic direct-item and presentation-layer style resolution.

use ifc_model::EntityId;

use crate::assignment::StyledItem;
use crate::error::{StyleError, StyleResult};
use crate::layer::PresentationLayer;
use crate::StyleView;

/// Which mechanism, if any, supplied a resolved item's effective styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSource {
    /// Neither a direct `IfcStyledItem` nor a presentation layer contributed
    /// any style to this item.
    None,
    /// The styles came from the unique direct `IfcStyledItem` bound to this
    /// item; carries that `IfcStyledItem`'s entity id.
    DirectStyledItem(EntityId),
    /// The styles came from an `IfcPresentationLayerAssignment` covering
    /// this item, in the absence of any direct assignment; carries that
    /// layer assignment's entity id.
    PresentationLayer(EntityId),
}

/// The outcome of resolving an item's effective presentation style through
/// the direct-assignment-over-layer-style cascade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStyle {
    source: StyleSource,
    effective_styles: Vec<EntityId>,
    direct_styles: Vec<EntityId>,
    layer_styles: Vec<(EntityId, Vec<EntityId>)>,
}

impl ResolvedStyle {
    /// Which mechanism produced `effective_styles`.
    pub fn source(&self) -> StyleSource {
        self.source
    }

    /// The styles that actually apply after the cascade: the direct
    /// assignment's styles if one exists, otherwise the union of all
    /// covering layer styles.
    pub fn effective_styles(&self) -> &[EntityId] {
        &self.effective_styles
    }

    /// The styles from the unique direct `IfcStyledItem`, if any; empty
    /// when resolution fell back to layer styles.
    pub fn direct_styles(&self) -> &[EntityId] {
        &self.direct_styles
    }

    /// Every presentation layer assignment covering the item, paired with
    /// its (deduplicated) flattened styles, regardless of whether a direct
    /// assignment took precedence.
    pub fn layer_styles(&self) -> &[(EntityId, Vec<EntityId>)] {
        &self.layer_styles
    }
}

impl<'m, 's> StyleView<'m, 's> {
    /// Resolves the effective style for `item` by scanning every
    /// `IfcStyledItem` and `IfcPresentationLayerAssignment` in the model.
    ///
    /// A unique direct `IfcStyledItem` binding wins over any layer style.
    /// Returns `Err(StyleError::AmbiguousStyleAssignment)` if more than one
    /// direct `IfcStyledItem` targets the same item.
    pub fn resolve_item_style(&self, item: EntityId) -> StyleResult<ResolvedStyle> {
        let mut direct = Vec::new();
        for (id, entity) in self.model.iter() {
            if !self.schema.is_a(&entity.type_name, "IfcStyledItem") {
                continue;
            }
            let styled = StyledItem::from_record(self.record(id, "IfcStyledItem")?);
            if styled.item()? == Some(item) {
                direct.push((id, self.flatten_assignments(styled.styles()?)?));
            }
        }
        direct.sort_by_key(|(id, _)| *id);
        if direct.len() > 1 {
            return Err(StyleError::AmbiguousStyleAssignment {
                item,
                count: direct.len(),
            });
        }

        let mut layer_styles = Vec::new();
        for (id, entity) in self.model.iter() {
            if !self
                .schema
                .is_a(&entity.type_name, "IfcPresentationLayerAssignment")
            {
                continue;
            }
            let layer =
                PresentationLayer::from_record(self.record(id, "IfcPresentationLayerAssignment")?);
            if layer.assigned_items()?.contains(&item) {
                let mut styles = self.flatten_assignments(layer.layer_styles()?)?;
                styles.sort_unstable();
                styles.dedup();
                layer_styles.push((id, styles));
            }
        }
        layer_styles.sort_by_key(|(id, _)| *id);

        let (source, direct_styles, effective_styles) = if let Some((id, styles)) = direct.pop() {
            (StyleSource::DirectStyledItem(id), styles.clone(), styles)
        } else {
            let mut styles: Vec<_> = layer_styles
                .iter()
                .flat_map(|(_, styles)| styles.iter().copied())
                .collect();
            styles.sort_unstable();
            styles.dedup();
            let source = layer_styles
                .iter()
                .find(|(_, styles)| !styles.is_empty())
                .map_or(StyleSource::None, |(id, _)| {
                    StyleSource::PresentationLayer(*id)
                });
            (source, Vec::new(), styles)
        };

        Ok(ResolvedStyle {
            source,
            effective_styles,
            direct_styles,
            layer_styles,
        })
    }

    /// Replaces each IFC2x3 `IfcPresentationStyleAssignment` wrapper id in
    /// `ids` with its member styles; direct `IfcPresentationStyle` ids pass
    /// through unchanged.
    fn flatten_assignments(&self, ids: Vec<EntityId>) -> StyleResult<Vec<EntityId>> {
        let mut out = Vec::new();
        for id in ids {
            let entity = self.model.get(id).ok_or(StyleError::UnknownEntity { id })?;
            if entity.is_type("IfcPresentationStyleAssignment") {
                out.extend(self.presentation_style_assignment(id)?.styles()?);
            } else {
                out.push(id);
            }
        }
        Ok(out)
    }
}
