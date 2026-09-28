//! The IFC release approval records are read and written against (#202,
//! #212).
//!
//! The layouts differ, from the EXPRESS sources:
//!
//! ```text
//! IfcApproval
//!   IFC2X3_TC1 (7)   Description, ApprovalDateTime (IfcDateTimeSelect),
//!                    ApprovalStatus, ApprovalLevel, ApprovalQualifier,
//!                    Name (required), Identifier (required)
//!   IFC4, IFC4X3 (9) Identifier, Name, Description, TimeOfApproval
//!                    (IfcDateTime), Status, Level, Qualifier,
//!                    RequestingApproval, GivingApproval
//! IfcApprovalRelationship
//!   IFC2X3_TC1 (4)   RelatedApproval (one), RelatingApproval, Description,
//!                    Name (required)
//!   IFC4, IFC4X3 (4) Name, Description, RelatingApproval,
//!                    RelatedApprovals (SET [1:?])
//! IfcResourceApprovalRelationship   IFC4 and IFC4X3 only
//! IfcRelAssociatesApproval
//!   IFC2X3_TC1       OwnerHistory required; RelatedObjects SET OF IfcRoot
//!   IFC4, IFC4X3     OwnerHistory OPTIONAL; SET OF IfcDefinitionSelect
//! ```
//!
//! Every slot is found by attribute name in the bound release's table, so
//! no record is read or written with another release's positions. Two
//! IFC4 names differ from IFC2X3's for the same attribute, per the IFC4
//! ADD2 TC1 documentation: `IfcApproval.TimeOfApproval` was "renamed from
//! ApprovalDateTime", and `IfcApprovalRelationship.RelatedApprovals` is
//! IFC2X3's `RelatedApproval` whose "cardinality ... has been changed to
//! SET". No other name is aliased: IFC2X3's `ApprovalStatus`,
//! `ApprovalLevel` and `ApprovalQualifier` are not documented as the IFC4
//! `Status`, `Level` and `Qualifier`, so those accessors refuse an IFC2X3
//! record with [`ApprovalError::NotInSchema`].
//!
//! Binding, from `FILE_SCHEMA`, as `ifc-material` (#77), `ifc-properties`
//! (#191) and `ifc-classification` (#194) bind:
//! - one declaration of IFC2X3, IFC4 or IFC4X3 binds that release's table;
//! - any other single declaration, IFC4X1 and IFC4X2 included, fails with
//!   [`ApprovalError::UnsupportedSchema`];
//! - several fail with [`ApprovalError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use crate::{ApprovalError, ApprovalResult};

/// The release a record is read or written in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Releases this crate's layouts are verified against.
const fn proven(version: SchemaVersion) -> bool {
    matches!(
        version,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    )
}

/// A model's binding, kept by a view until a projection needs it.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Binding<'m> {
    /// A verified release.
    Bound(Layout),
    /// The header declares this many schemas.
    Multiple(usize),
    /// The header declares one schema this crate is not verified against.
    Unsupported(&'m str),
}

impl<'m> Binding<'m> {
    /// The binding `model`'s header declares.
    pub(crate) fn of(model: &'m Model) -> Self {
        match model.header().schema.as_slice() {
            [] => Layout::of_version(SchemaVersion::Ifc4)
                .map_or(Self::Unsupported("IFC4"), Self::Bound),
            [token] => SchemaVersion::from_header_token(token)
                .and_then(|version| Layout::of_version(version).ok())
                .map_or(Self::Unsupported(token.as_str()), Self::Bound),
            tokens => Self::Multiple(tokens.len()),
        }
    }

    /// The bound layout, or the refusal.
    pub(crate) fn layout(self) -> ApprovalResult<Layout> {
        match self {
            Self::Bound(layout) => Ok(layout),
            Self::Multiple(schemas) => Err(ApprovalError::MultipleSchemas { schemas }),
            Self::Unsupported(schema) => Err(ApprovalError::UnsupportedSchema {
                schema: schema.to_owned(),
            }),
        }
    }
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> ApprovalResult<Layout> {
    Binding::of(model).layout()
}

impl Layout {
    /// Bind `version`, refusing a release this crate is not verified for.
    pub(crate) fn of_version(version: SchemaVersion) -> ApprovalResult<Self> {
        let unsupported = || ApprovalError::UnsupportedSchema {
            schema: version.release_id().to_owned(),
        };
        if !proven(version) {
            return Err(unsupported());
        }
        let schema = for_version(version).map_err(|_| unsupported())?;
        Ok(Self { version, schema })
    }

    /// The bound release.
    pub(crate) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// The bound release's bundled table.
    pub(crate) fn schema(self) -> &'static Schema {
        self.schema
    }

    /// Fail with [`ApprovalError::EntityNotInSchema`] unless the release
    /// instantiates `entity`.
    pub(crate) fn require_entity(self, entity: &'static str) -> ApprovalResult<()> {
        if self.schema.entity(entity).is_some_and(|e| !e.abstract_) {
            Ok(())
        } else {
            Err(ApprovalError::EntityNotInSchema {
                entity,
                schema: self.version,
            })
        }
    }

    /// Position and declaration of the attribute this crate knows by its
    /// IFC4 name, in this release.
    pub(crate) fn declared(
        self,
        entity: &str,
        attribute: &'static str,
    ) -> Option<(usize, &'static Attribute)> {
        let name = release_name(self.version, entity, attribute);
        self.schema
            .attributes(entity)
            .into_iter()
            .enumerate()
            .find(|(_, declared)| declared.name.eq_ignore_ascii_case(name))
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
    /// by their IFC4 attribute names; unnamed slots are `$`.
    ///
    /// Refuses, before anything is staged, an entity the release does not
    /// declare ([`ApprovalError::EntityNotInSchema`]); a value for an
    /// attribute it does not declare ([`ApprovalError::AuthoringNotInSchema`]);
    /// text where it declares a record, a list where it declares a single
    /// reference, or a reference where it declares text
    /// ([`ApprovalError::AuthoringValueType`]); and a required attribute
    /// left `$` ([`ApprovalError::AuthoringRequired`]), such as the IFC2X3
    /// `OwnerHistory`.
    pub(crate) fn named_record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> ApprovalResult<Entity> {
        self.require_entity(entity)?;
        let declared = self.schema.attributes(entity);
        let mut attributes = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let Some((slot, declaration)) = self.declared(entity, attribute) else {
                if value == Value::Null {
                    continue;
                }
                return Err(ApprovalError::AuthoringNotInSchema {
                    entity,
                    attribute,
                    schema: self.version,
                });
            };
            if !self.holds(declaration, &value) {
                return Err(ApprovalError::AuthoringValueType {
                    entity,
                    attribute,
                    declared: declaration.type_name.as_str(),
                    schema: self.version,
                });
            }
            attributes[slot] = value;
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

    /// Whether `value`'s shape fits `declaration`: text only in a string
    /// type, references only where an entity is reachable, a list only in
    /// an aggregate. Reference targets are checked by the caller.
    fn holds(self, declaration: &Attribute, value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::List(items) => {
                declaration.aggregate
                    && items
                        .iter()
                        .all(|item| self.holds_scalar(&declaration.type_name, item))
            }
            other => !declaration.aggregate && self.holds_scalar(&declaration.type_name, other),
        }
    }

    fn holds_scalar(self, declared: &str, value: &Value) -> bool {
        match value {
            Value::Text(_) => self.is_text(declared),
            Value::Ref(_) => self.admits_entity(declared, 8),
            _ => true,
        }
    }

    /// Whether `declared` resolves to a STRING type.
    pub(crate) fn is_text(self, declared: &str) -> bool {
        self.schema
            .resolve_defined(declared)
            .to_ascii_uppercase()
            .starts_with("STRING")
    }

    /// Whether `declared` is an entity, or a SELECT that reaches one.
    pub(crate) fn admits_entity(self, declared: &str, depth: usize) -> bool {
        if self.schema.entity(declared).is_some() {
            return true;
        }
        match self.schema.type_def(declared).map(|t| &t.kind) {
            Some(TypeKind::Select(members)) if depth > 0 => members
                .iter()
                .any(|member| self.admits_entity(member, depth - 1)),
            _ => false,
        }
    }
}

/// The name `release` gives the attribute this crate knows by its IFC4
/// name; see the module documentation for the two renames.
fn release_name(release: SchemaVersion, entity: &str, attribute: &'static str) -> &'static str {
    match (release, entity, attribute) {
        (SchemaVersion::Ifc2x3, "IFCAPPROVAL", "TimeOfApproval") => "ApprovalDateTime",
        (SchemaVersion::Ifc2x3, "IFCAPPROVALRELATIONSHIP", "RelatedApprovals") => "RelatedApproval",
        _ => attribute,
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
