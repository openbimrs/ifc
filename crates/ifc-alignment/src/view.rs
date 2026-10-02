//! Pins the authoritative IFC4X3 profile and exposes bounded traversal.
//!
//! [`AlignmentView`] is the read-side entry point: it refuses any release
//! but IFC4X3, and [`AlignmentView::hierarchy`] answers what an
//! `IfcAlignment` is made of (its layouts, child alignments and referents)
//! without the caller re-walking `IfcRelNests`/`IfcRelAggregates`.
//!
//! ## Internal split
//!
//! - `hierarchy.rs`: the public alignment hierarchy.
//!
//! `IfcAlignment*` entities were introduced in IFC4X3; IFC2X3 and IFC4 ADD2
//! TC1 do not declare them at all. Unlike `ifc-resource`/`ifc-structural`,
//! there is therefore no cross-version dispatch table here -- exactly one
//! schema profile is authoritative, and any other declared schema is a typed
//! refusal rather than an approximation.

use std::collections::HashSet;

use ifc_model::{EntityId, Model};
use ifc_schema::{ifc4x3, Schema, SchemaVersion};

use crate::error::{AlignmentError, AlignmentResult};
use crate::slot;

mod hierarchy;

pub use hierarchy::AlignmentHierarchy;

/// Where a relationship keeps its two ends, and their schema names.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RelationSlots {
    relating: usize,
    relating_name: &'static str,
    related: usize,
    related_name: &'static str,
}

impl RelationSlots {
    /// `IfcRelNests` and `IfcRelAggregates` (`IfcRelDecomposes`).
    pub(crate) const DECOMPOSES: Self = Self {
        relating: slot::decomposes::RELATING_OBJECT,
        relating_name: "RelatingObject",
        related: slot::decomposes::RELATED_OBJECTS,
        related_name: "RelatedObjects",
    };
    /// `IfcRelPositions`.
    pub(crate) const POSITIONS: Self = Self {
        relating: slot::rel_positions::RELATING_POSITIONING_ELEMENT,
        relating_name: "RelatingPositioningElement",
        related: slot::rel_positions::RELATED_PRODUCTS,
        related_name: "RelatedProducts",
    };
}

/// A model whose declared schema has been pinned to IFC4X3 ADD2.
///
/// Holding the schema alongside the model lets traversal use `is_a` for
/// subtype-aware queries instead of exact type-name matching.
#[derive(Debug, Clone, Copy)]
pub struct AlignmentView<'m> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'static Schema,
}

impl<'m> AlignmentView<'m> {
    /// Pin the model's declared schema and confirm it is IFC4X3.
    pub fn for_model(model: &'m Model) -> AlignmentResult<Self> {
        let token = match model.header().schema.as_slice() {
            [] => return Err(AlignmentError::MissingSchema),
            [token] => token,
            tokens => {
                return Err(AlignmentError::AmbiguousSchema {
                    tokens: tokens.to_vec(),
                });
            }
        };
        let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
            AlignmentError::UnsupportedSchema {
                token: token.clone(),
            }
        })?;
        if version != SchemaVersion::Ifc4x3 {
            return Err(AlignmentError::UnsupportedSchema {
                token: token.clone(),
            });
        }
        Ok(Self {
            model,
            schema: ifc4x3(),
        })
    }

    /// Entity ids whose declared type is `ancestor` or a subtype of it.
    pub(crate) fn ids_of_ancestor(&self, ancestor: &str) -> Vec<EntityId> {
        self.model
            .iter()
            .filter_map(|(id, entity)| self.schema.is_a(&entity.type_name, ancestor).then_some(id))
            .collect()
    }

    /// Directly nested children of `parent` (`IfcRelNests.RelatedObjects` in
    /// `LIST` order), filtered to entities whose type satisfies `expected`.
    ///
    /// Nesting order is load-bearing for alignment: segment order along a
    /// horizontal/vertical/cant layout is defined by nesting order, not by
    /// any numeric field on the segment itself.
    pub(crate) fn nested_children(
        &self,
        parent: EntityId,
        expected: &str,
    ) -> AlignmentResult<Vec<EntityId>> {
        let children = self.related_by("IfcRelNests", RelationSlots::DECOMPOSES, parent)?;
        Ok(self.filter_is_a(children, expected))
    }

    /// Every object a relationship of type `relation` relates to `relating`,
    /// in relationship order and then in each `RelatedObjects` order.
    ///
    /// Each related reference must resolve: a dangling one is refused, not
    /// skipped, because skipping it would silently shorten a layout.
    pub(crate) fn related_by(
        &self,
        relation: &str,
        slots: RelationSlots,
        relating: EntityId,
    ) -> AlignmentResult<Vec<EntityId>> {
        let mut result = Vec::new();
        for id in self.ids_of_ancestor(relation) {
            if self.relating_end(id, slots)? == relating {
                result.extend(self.related_end(id, slots)?);
            }
        }
        Ok(result)
    }

    /// Every object that relates `related` through a relationship of type
    /// `relation`: the inverse of [`Self::related_by`].
    ///
    /// Only relationships that list `related` are read in full, so a
    /// malformed relationship elsewhere in the file does not refuse this
    /// one's question.
    pub(crate) fn relating_of(
        &self,
        relation: &str,
        slots: RelationSlots,
        related: EntityId,
    ) -> AlignmentResult<Vec<EntityId>> {
        let mut result = Vec::new();
        for id in self.ids_of_ancestor(relation) {
            let lists_it = self
                .model
                .get(id)
                .and_then(|entity| entity.attributes.get(slots.related))
                .and_then(|value| value.as_list())
                .is_some_and(|values| values.iter().any(|v| v.as_ref_id() == Some(related)));
            if lists_it {
                result.push(self.relating_end(id, slots)?);
            }
        }
        Ok(result)
    }

    /// `ids` restricted to entities whose declared type is `expected` or a
    /// subtype of it.
    pub(crate) fn filter_is_a(&self, ids: Vec<EntityId>, expected: &str) -> Vec<EntityId> {
        ids.into_iter()
            .filter(|id| {
                self.model
                    .get(*id)
                    .is_some_and(|entity| self.schema.is_a(&entity.type_name, expected))
            })
            .collect()
    }

    /// The relating object of one relationship instance.
    fn relating_end(&self, relation: EntityId, slots: RelationSlots) -> AlignmentResult<EntityId> {
        self.model
            .get(relation)
            .ok_or(AlignmentError::MissingEntity { entity: relation })?
            .attributes
            .get(slots.relating)
            .and_then(|value| value.as_ref_id())
            .ok_or(AlignmentError::InvalidAttribute {
                entity: relation,
                index: slots.relating,
                name: slots.relating_name,
            })
    }

    /// The related objects of one relationship instance, each resolved.
    fn related_end(
        &self,
        relation: EntityId,
        slots: RelationSlots,
    ) -> AlignmentResult<Vec<EntityId>> {
        let invalid = AlignmentError::InvalidAttribute {
            entity: relation,
            index: slots.related,
            name: slots.related_name,
        };
        let values = self
            .model
            .get(relation)
            .ok_or(AlignmentError::MissingEntity { entity: relation })?
            .attributes
            .get(slots.related)
            .and_then(|value| value.as_list())
            .ok_or_else(|| invalid.clone())?;
        let mut related = Vec::with_capacity(values.len());
        for value in values {
            let child = value.as_ref_id().ok_or_else(|| invalid.clone())?;
            if self.model.get(child).is_none() {
                return Err(AlignmentError::DanglingReference {
                    entity: relation,
                    attribute: slots.related_name,
                    target: child,
                });
            }
            related.push(child);
        }
        Ok(related)
    }

    /// The `IfcAlignmentSegment` instances nested under `parent`, together
    /// with the `IfcAlignmentParameterSegment` each one's `DesignParameters`
    /// resolves to, in authored order.
    ///
    /// `IfcAlignmentSegment` is the geometry-bearing node in the nesting
    /// tree; `DesignParameters` is a direct forward reference to the typed
    /// parameter entity (`IfcAlignmentHorizontalSegment`, etc), not itself
    /// nested. Both indirections are resolved here so callers get the
    /// concrete parameter entity id directly.
    pub(crate) fn segment_chain(
        &self,
        parent: EntityId,
        design_parameters_type: &str,
    ) -> AlignmentResult<Vec<EntityId>> {
        let segments = self.nested_children(parent, "IfcAlignmentSegment")?;
        let mut seen = HashSet::with_capacity(segments.len());
        let mut result = Vec::with_capacity(segments.len());
        for segment in segments {
            if !seen.insert(segment) {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(segment),
                    rule: "IfcRelNests must not list the same segment twice",
                });
            }
            let entity = self
                .model
                .get(segment)
                .ok_or(AlignmentError::MissingEntity { entity: segment })?;
            // `IfcAlignmentSegment` subtype of `IfcLinearElement` subtype of
            // `IfcProduct`; `DesignParameters` is its sole own attribute.
            let design_parameters = entity
                .attributes
                .get(slot::segment::DESIGN_PARAMETERS)
                .and_then(|value| value.as_ref_id())
                .ok_or(AlignmentError::InvalidAttribute {
                    entity: segment,
                    index: slot::segment::DESIGN_PARAMETERS,
                    name: "DesignParameters",
                })?;
            let parameters_entity =
                self.model
                    .get(design_parameters)
                    .ok_or(AlignmentError::DanglingReference {
                        entity: segment,
                        attribute: "DesignParameters",
                        target: design_parameters,
                    })?;
            if !self
                .schema
                .is_a(&parameters_entity.type_name, design_parameters_type)
            {
                return Err(AlignmentError::WrongType {
                    entity: design_parameters,
                    expected: "matches the requested design-parameters family",
                    actual: parameters_entity.type_name.to_string(),
                });
            }
            result.push(design_parameters);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Header;

    fn model_with_schema(tokens: &[&str]) -> Model {
        let mut model = Model::default();
        *model.header_mut() = Header {
            schema: tokens.iter().map(|s| s.to_string()).collect(),
            ..Header::default()
        };
        model
    }

    #[test]
    fn refuses_a_missing_schema_declaration() {
        let model = model_with_schema(&[]);
        assert!(matches!(
            AlignmentView::for_model(&model),
            Err(AlignmentError::MissingSchema)
        ));
    }

    #[test]
    fn refuses_an_ambiguous_schema_declaration() {
        let model = model_with_schema(&["IFC4X3_ADD2", "IFC4"]);
        assert!(matches!(
            AlignmentView::for_model(&model),
            Err(AlignmentError::AmbiguousSchema { .. })
        ));
    }

    #[test]
    fn refuses_ifc2x3_because_alignment_entities_do_not_exist_there() {
        let model = model_with_schema(&["IFC2X3"]);
        assert!(matches!(
            AlignmentView::for_model(&model),
            Err(AlignmentError::UnsupportedSchema { token }) if token == "IFC2X3"
        ));
    }

    #[test]
    fn refuses_ifc4_because_alignment_entities_do_not_exist_there() {
        let model = model_with_schema(&["IFC4"]);
        assert!(matches!(
            AlignmentView::for_model(&model),
            Err(AlignmentError::UnsupportedSchema { token }) if token == "IFC4"
        ));
    }

    #[test]
    fn accepts_ifc4x3_add2() {
        let model = model_with_schema(&["IFC4X3_ADD2"]);
        assert!(AlignmentView::for_model(&model).is_ok());
    }

    #[test]
    fn accepts_the_bare_ifc4x3_token() {
        let model = model_with_schema(&["IFC4X3"]);
        assert!(AlignmentView::for_model(&model).is_ok());
    }

    #[test]
    fn refuses_an_unrecognized_token() {
        let model = model_with_schema(&["IFC5"]);
        assert!(matches!(
            AlignmentView::for_model(&model),
            Err(AlignmentError::UnsupportedSchema { token }) if token == "IFC5"
        ));
    }
}

#[cfg(test)]
mod intermediate_release_tests {
    use super::*;

    /// IFC4X1 and IFC4X2 have bundled tables but no verified layout here:
    /// refused with the unsupported-schema error, never read as IFC4/IFC4X3.
    #[test]
    fn ifc4x1_and_ifc4x2_are_refused_not_aliased() {
        for token in ["IFC4X1", "IFC4X2"] {
            let mut model = Model::new();
            model.header_mut().schema = vec![token.to_owned()];
            assert!(
                matches!(
                    AlignmentView::for_model(&model),
                    Err(AlignmentError::UnsupportedSchema { token: found }) if found == token
                ),
                "{token} must be refused"
            );
        }
    }
}
