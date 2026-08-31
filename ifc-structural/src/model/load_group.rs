//! `IfcStructuralLoadGroup` projection.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::StructuralResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct LoadGroup<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LoadGroup<'m, 's> {
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

    pub fn predefined_type(&self) -> StructuralResult<&'m str> {
        self.validate_semantics()?;
        self.record.required_enum("PredefinedType")
    }

    pub fn action_type(&self) -> StructuralResult<&'m str> {
        self.validate_semantics()?;
        self.record.required_enum("ActionType")
    }

    pub fn action_source(&self) -> StructuralResult<&'m str> {
        self.validate_semantics()?;
        self.record.required_enum("ActionSource")
    }

    pub fn coefficient(&self) -> StructuralResult<Option<f64>> {
        self.validate_semantics()?;
        self.record.optional_number("Coefficient")
    }

    pub fn purpose(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        self.record.optional_text("Purpose")
    }

    fn validate_semantics(&self) -> StructuralResult<()> {
        if self.record.schema.version() == Some(SchemaVersion::Ifc2x3) {
            return Ok(());
        }
        let user_defined = ["PredefinedType", "ActionType", "ActionSource"]
            .into_iter()
            .try_fold(false, |found, attribute| {
                self.record
                    .required_enum(attribute)
                    .map(|value| found || value.eq_ignore_ascii_case("USERDEFINED"))
            })?;
        self.record.require_object_type_if(
            user_defined,
            "USERDEFINED load group metadata requires ObjectType",
        )
    }
}
