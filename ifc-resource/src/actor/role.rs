//! `IfcActorRole` — bounded IFC4 role projection.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::view::Record;

/// A borrowed, schema-resolved `IfcActorRole` projection.
///
/// Enforces `IfcActorRole.WR1`: a `USERDEFINED` role must carry
/// `UserDefinedRole`.
#[derive(Debug, Clone, Copy)]
pub struct ActorRole<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ActorRole<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        let role = Self { record };
        role.role()?;
        Ok(role)
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn role(&self) -> ResourceResult<&'m str> {
        let value = self.record.required_enum("Role")?;
        if value.eq_ignore_ascii_case("USERDEFINED")
            && self
                .record
                .optional_text("UserDefinedRole")?
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(ResourceError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "IfcActorRole.WR1 requires UserDefinedRole for USERDEFINED",
            });
        }
        Ok(value)
    }

    pub fn user_defined_role(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("UserDefinedRole")
    }

    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }
}
