//! Attributes read by name from one release's table.
//!
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 declare `IfcSimplePropertyTemplate`,
//! `IfcComplexPropertyTemplate`, `IfcPropertySetTemplate` and
//! `IfcRelDefinesByTemplate` with the same attributes in the same order
//! (`references/ifc-spec/*/*.exp`); IFC2X3 TC1 declares none of them.
//! Positions are still looked up by attribute name in the bound release's
//! table rather than assumed, so a record is read with its own release's
//! layout, and an entity that release does not declare is not read at all.
//!
//! The two templates differ after `IfcRoot`, which is why one fixed layout
//! cannot read both (#108):
//!
//! ```text
//! IfcSimplePropertyTemplate   TemplateType PrimaryMeasureType SecondaryMeasureType
//!                             Enumerators PrimaryUnit SecondaryUnit Expression
//!                             AccessState
//! IfcComplexPropertyTemplate  UsageName TemplateType HasPropertyTemplates
//! ```

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{for_version, ifc4, Schema, SchemaVersion, TypeKind};

use crate::error::{PropertyAnomaly, TemplateError};
use crate::exact_schema;

/// A release's table, through which template records are read.
#[derive(Clone, Copy)]
pub(crate) struct Layout {
    version: SchemaVersion,
    schema: &'static Schema,
}

impl Layout {
    /// The table for a permissive read.
    ///
    /// The header's single `FILE_SCHEMA`, when it names a bundled release;
    /// otherwise the IFC4 ADD2 TC1 baseline, so that a model built in memory
    /// without a header still reads. An IFC2X3 header binds the IFC2X3
    /// table, which declares no templates, so nothing reads as one.
    pub(crate) fn permissive(model: &Model) -> Self {
        let declared = match model.header().schema.as_slice() {
            [token] => SchemaVersion::from_header_token(token)
                .and_then(|version| for_version(version).map(|schema| (version, schema))),
            _ => None,
        };
        let (version, schema) = declared.unwrap_or((SchemaVersion::Ifc4, ifc4()));
        Self { version, schema }
    }

    /// The table of the release the model declares, as
    /// [`exact_schema`] binds it.
    ///
    /// # Errors
    ///
    /// [`TemplateError::Release`] when the model binds to no single release,
    /// and [`TemplateError::NoTemplates`] when that release declares no
    /// `IfcPropertySetTemplate`.
    pub(crate) fn declared(model: &Model) -> Result<Self, TemplateError> {
        let version = exact_schema(model).map_err(TemplateError::Release)?;
        let schema = for_version(version).ok_or(TemplateError::NoTemplates { schema: version })?;
        if schema.entity("IFCPROPERTYSETTEMPLATE").is_none() {
            return Err(TemplateError::NoTemplates { schema: version });
        }
        Ok(Self { version, schema })
    }

    /// The release whose table this is.
    pub(crate) fn version(self) -> SchemaVersion {
        self.version
    }

    pub(crate) fn schema(self) -> &'static Schema {
        self.schema
    }

    /// Whether `name` is `ancestor` or one of its subtypes in this release.
    pub(crate) fn is_a(self, name: &str, ancestor: &str) -> bool {
        self.schema.is_a(name, ancestor)
    }

    /// Declared position and type of `entity`'s attribute `attribute`.
    fn declared_attribute(self, entity: &str, attribute: &str) -> Option<(usize, &'static str)> {
        self.schema
            .attributes(entity)
            .into_iter()
            .enumerate()
            .find(|(_, declared)| declared.name.eq_ignore_ascii_case(attribute))
            .map(|(slot, declared)| (slot, declared.type_name.as_str()))
    }

    /// The value of attribute `attribute`, or `None` when the release does
    /// not declare it for this entity or the record is too short to hold it.
    pub(crate) fn get<'e>(self, entity: &'e Entity, attribute: &str) -> Option<&'e Value> {
        let (slot, _) = self.declared_attribute(&entity.type_name, attribute)?;
        entity.attributes.get(slot)
    }

    /// Report a record whose attribute count is not the release's arity.
    pub(crate) fn check_arity(self, id: EntityId, entity: &Entity, out: &mut Vec<PropertyAnomaly>) {
        let expected = self.schema.attributes(&entity.type_name).len();
        if expected != entity.attributes.len() {
            out.push(PropertyAnomaly::SlotCountMismatch {
                entity: id,
                type_name: entity.type_name.to_string(),
                expected,
                actual: entity.attributes.len(),
            });
        }
    }

    /// The members of enumeration type `name`, empty when it is none.
    pub(crate) fn enum_members(self, name: &str) -> &'static [String] {
        match self
            .schema
            .type_def(name)
            .map(|definition| &definition.kind)
        {
            Some(TypeKind::Enumeration(members)) => members,
            _ => &[],
        }
    }

    /// Whether `token` is a member of the enumeration that `entity`'s
    /// attribute `attribute` declares.
    pub(crate) fn enum_accepts(self, entity: &str, attribute: &str, token: &str) -> bool {
        self.declared_attribute(entity, attribute)
            .is_some_and(|(_, declared)| {
                self.enum_members(declared)
                    .iter()
                    .any(|member| member.eq_ignore_ascii_case(token))
            })
    }

    /// A label or text attribute, reporting a value of another kind.
    pub(crate) fn text(
        self,
        id: EntityId,
        entity: &Entity,
        attribute: &'static str,
        out: &mut Vec<PropertyAnomaly>,
    ) -> Option<std::sync::Arc<str>> {
        match self.get(entity, attribute)?.unwrap_typed() {
            Value::Text(text) => Some(text.clone()),
            Value::Null => None,
            other => {
                out.push(malformed(id, attribute, other));
                None
            }
        }
    }

    /// An enumeration attribute as written, reporting a constant that is
    /// not a member of the declared enumeration in this release.
    pub(crate) fn enumeration(
        self,
        id: EntityId,
        entity: &Entity,
        attribute: &'static str,
        out: &mut Vec<PropertyAnomaly>,
    ) -> Option<std::sync::Arc<str>> {
        match self.get(entity, attribute)?.unwrap_typed() {
            Value::Enum(token) => {
                if !self.enum_accepts(&entity.type_name, attribute, token) {
                    out.push(malformed(id, attribute, &Value::Enum(token.clone())));
                }
                Some(token.clone())
            }
            Value::Null => None,
            other => {
                out.push(malformed(id, attribute, other));
                None
            }
        }
    }

    /// An entity-reference attribute as written, reporting a target absent
    /// from the model or outside the declared type.
    pub(crate) fn reference(
        self,
        model: &Model,
        id: EntityId,
        entity: &Entity,
        attribute: &'static str,
        out: &mut Vec<PropertyAnomaly>,
    ) -> Option<EntityId> {
        let (_, declared) = self.declared_attribute(&entity.type_name, attribute)?;
        match self.get(entity, attribute)?.unwrap_typed() {
            Value::Ref(target) => {
                let admitted = model
                    .get(*target)
                    .is_some_and(|found| self.schema.accepts_type(declared, &found.type_name));
                if !admitted {
                    out.push(malformed(id, attribute, &Value::Ref(*target)));
                }
                Some(*target)
            }
            Value::Null => None,
            other => {
                out.push(malformed(id, attribute, other));
                None
            }
        }
    }
}

fn malformed(entity: EntityId, attribute: &'static str, found: &Value) -> PropertyAnomaly {
    PropertyAnomaly::MalformedAttribute {
        entity,
        attribute,
        found: format!("{found:?}"),
    }
}
