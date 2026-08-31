//! `IfcStructuralResultGroup` projection.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::StructuralResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct ResultGroup<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ResultGroup<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        self.record.optional_text("Name")
    }

    pub fn theory_type(&self) -> StructuralResult<&'m str> {
        self.validate_semantics()?;
        self.record.required_enum("TheoryType")
    }

    pub fn result_for_load_group(&self) -> StructuralResult<Option<EntityId>> {
        self.validate_semantics()?;
        self.record
            .optional_ref("ResultForLoadGroup", "IfcStructuralLoadGroup")
    }

    pub fn is_linear(&self) -> StructuralResult<bool> {
        self.validate_semantics()?;
        self.record.required_bool("IsLinear")
    }

    fn validate_semantics(&self) -> StructuralResult<()> {
        if self.record.schema.version() == Some(SchemaVersion::Ifc2x3) {
            return Ok(());
        }
        let user_defined = self
            .record
            .required_enum("TheoryType")?
            .eq_ignore_ascii_case("USERDEFINED");
        self.record.require_object_type_if(
            user_defined,
            "USERDEFINED result theory requires ObjectType",
        )
    }
}
