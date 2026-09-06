//! Pins the authoritative IFC4X3 profile and exposes bounded traversal.
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
        let mut result = Vec::new();
        for relation in self.ids_of_ancestor("IfcRelNests") {
            let entity = self
                .model
                .get(relation)
                .ok_or(AlignmentError::MissingEntity { entity: relation })?;
            let relating = entity
                .attributes
                .get(4)
                .and_then(|value| value.as_ref_id())
                .ok_or(AlignmentError::InvalidAttribute {
                    entity: relation,
                    index: 4,
                    name: "RelatingObject",
                })?;
            if relating != parent {
                continue;
            }
            let related = entity
                .attributes
                .get(5)
                .and_then(|value| value.as_list())
                .ok_or(AlignmentError::InvalidAttribute {
                    entity: relation,
                    index: 5,
                    name: "RelatedObjects",
                })?;
            for value in related {
                let child = value.as_ref_id().ok_or(AlignmentError::InvalidAttribute {
                    entity: relation,
                    index: 5,
                    name: "RelatedObjects",
                })?;
                let child_entity =
                    self.model
                        .get(child)
                        .ok_or(AlignmentError::DanglingReference {
                            entity: relation,
                            attribute: "RelatedObjects",
                            target: child,
                        })?;
                if self.schema.is_a(&child_entity.type_name, expected) {
                    result.push(child);
                }
            }
        }
        Ok(result)
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
            // `IfcProduct`; `DesignParameters` is the sole attribute this
            // crate's declaration contributes, past the inherited slots.
            let design_parameters = entity
                .attributes
                .last()
                .and_then(|value| value.as_ref_id())
                .ok_or(AlignmentError::InvalidAttribute {
                    entity: segment,
                    index: entity.attributes.len().saturating_sub(1),
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
