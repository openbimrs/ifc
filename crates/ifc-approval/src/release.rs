//! The IFC release `IfcRelAssociatesApproval` is authored against (#202).
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 TC1 and `OPTIONAL` from IFC4
//! on, and `IfcRelAssociates.RelatedObjects` is `SET OF IfcRoot` with WR21 in
//! IFC2X3 where IFC4 and IFC4X3 declare `SET OF IfcDefinitionSelect`:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//!              RelatedObjects : SET [1:?] OF IfcRoot;   (WR21)
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  RelatedObjects : SET [1:?] OF IfcDefinitionSelect;
//! ```
//!
//! The record is laid out by attribute name from the bound release's table,
//! and reference targets are checked against that table. Binding, from
//! `FILE_SCHEMA`, as `ifc-material` (#77), `ifc-properties` (#191) and
//! `ifc-classification` (#194) bind their authoring:
//! - one recognised declaration binds that release's table;
//! - one unrecognised declaration fails with
//!   [`ApprovalError::UnsupportedSchema`];
//! - several fail with [`ApprovalError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Schema, SchemaVersion};

use crate::{ApprovalError, ApprovalResult};

/// The release a record is written in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Releases this crate's layouts are proven against.
///
/// `ifc-schema` also bundles IFC4X1 and IFC4X2, but nothing here is verified
/// against their tables, so a header declaring either is refused with the
/// unsupported-schema error rather than read through a neighbour's layout.
const fn proven(version: SchemaVersion) -> bool {
    matches!(
        version,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    )
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> ApprovalResult<Layout> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token)
            .filter(|version| proven(*version))
            .ok_or_else(|| ApprovalError::UnsupportedSchema {
                schema: token.clone(),
            })?,
        tokens => {
            return Err(ApprovalError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).ok_or_else(|| ApprovalError::UnsupportedSchema {
        schema: format!("{version:?}"),
    })?;
    Ok(Layout { version, schema })
}

impl Layout {
    /// The bound release's bundled table.
    pub(crate) fn schema(self) -> &'static Schema {
        self.schema
    }

    /// Does `actual` satisfy `RelatedObjects` of an `IfcRelAssociates` in
    /// this release? `IfcDefinitionSelect` where the release declares it,
    /// otherwise (IFC2X3) WR21: an `IfcObjectDefinition` or an
    /// `IfcPropertyDefinition`, the same two branches.
    pub(crate) fn is_definition(self, actual: &str) -> bool {
        if self.schema.type_def("IfcDefinitionSelect").is_some() {
            return self.schema.accepts_type("IfcDefinitionSelect", actual);
        }
        self.schema.is_a(actual, "IFCOBJECTDEFINITION")
            || self.schema.is_a(actual, "IFCPROPERTYDEFINITION")
    }

    /// Build a record of `entity` in this release's layout from values named
    /// by attribute; unnamed slots are `$`.
    ///
    /// Refuses, before anything is staged, a required attribute left `$`
    /// with [`ApprovalError::AuthoringRequired`], such as the IFC2X3
    /// `OwnerHistory`, and a value for an attribute the release does not
    /// declare with [`ApprovalError::AuthoringInvalid`].
    pub(crate) fn named_record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> ApprovalResult<Entity> {
        let declared = self.schema.attributes(entity);
        let mut attributes = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let slot = declared
                .iter()
                .position(|found| found.name.eq_ignore_ascii_case(attribute));
            match slot {
                Some(slot) => attributes[slot] = value,
                None if value == Value::Null => {}
                None => {
                    return Err(ApprovalError::AuthoringInvalid {
                        entity,
                        attribute,
                        value: format!("not declared by {:?}", self.version),
                    })
                }
            }
        }
        for (declaration, value) in declared.iter().zip(&attributes) {
            if *value == Value::Null && !declaration.optional {
                return Err(ApprovalError::AuthoringRequired {
                    entity,
                    attribute: declaration.name.as_str(),
                    schema: self.version,
                });
            }
        }
        Ok(Entity::new(entity, attributes))
    }
}

/// Fail unless `id` is an `IfcOwnerHistory` in the model or staged on `tx`.
///
/// A missing one is [`ApprovalError::UnknownEntity`], another entity
/// [`ApprovalError::AuthoringReferenceType`]. None is ever invented.
pub(crate) fn require_owner_history(
    tx: &Transaction,
    model: &Model,
    id: EntityId,
) -> ApprovalResult<()> {
    let actual = projected_type(tx, model, id).ok_or(ApprovalError::UnknownEntity { id })?;
    if actual.eq_ignore_ascii_case("IFCOWNERHISTORY") {
        return Ok(());
    }
    Err(ApprovalError::AuthoringReferenceType {
        target: id,
        expected: "IfcOwnerHistory",
        actual,
    })
}

/// The type `id` has once the transaction's staged edits are applied.
fn projected_type(tx: &Transaction, model: &Model, id: EntityId) -> Option<String> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(entity.type_name.to_string()),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name.to_string()),
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.to_string())
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
                matches!(bind(&model), Err(ApprovalError::UnsupportedSchema { schema }) if schema == token),
                "{token} must be refused"
            );
        }
    }
}
