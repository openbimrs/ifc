//! Deterministic direct-item and presentation-layer style resolution.

use ifc_model::EntityId;

use crate::assignment::StyledItem;
use crate::error::{StyleError, StyleResult};
use crate::layer::PresentationLayer;
use crate::StyleView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleSource {
    None,
    DirectStyledItem(EntityId),
    PresentationLayer(EntityId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStyle {
    source: StyleSource,
    effective_styles: Vec<EntityId>,
    direct_styles: Vec<EntityId>,
    layer_styles: Vec<(EntityId, Vec<EntityId>)>,
}

impl ResolvedStyle {
    pub fn source(&self) -> StyleSource {
        self.source
    }

    pub fn effective_styles(&self) -> &[EntityId] {
        &self.effective_styles
    }

    pub fn direct_styles(&self) -> &[EntityId] {
        &self.direct_styles
    }

    pub fn layer_styles(&self) -> &[(EntityId, Vec<EntityId>)] {
        &self.layer_styles
    }
}

impl<'m, 's> StyleView<'m, 's> {
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
