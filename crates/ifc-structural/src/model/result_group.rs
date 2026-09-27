//! `IfcStructuralResultGroup` projection.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use crate::error::StructuralResult;
use crate::view::Record;

/// Borrowed projection of an `IfcStructuralResultGroup`.
#[derive(Debug, Clone, Copy)]
pub struct ResultGroup<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ResultGroup<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    #[must_use]
    /// The `IfcStructuralResultGroup` entity id.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// `Name`, inherited from `IfcRoot`. Legally absent.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.validate_semantics()?;
        self.record.optional_text("Name")
    }

    /// `TheoryType`, always mandatory.
    pub fn theory_type(&self) -> StructuralResult<&'m str> {
        self.validate_semantics()?;
        self.record.required_enum("TheoryType")
    }

    /// `ResultForLoadGroup`, the `IfcStructuralLoadGroup` these results were computed for. Legally absent.
    pub fn result_for_load_group(&self) -> StructuralResult<Option<EntityId>> {
        self.validate_semantics()?;
        self.record
            .optional_ref("ResultForLoadGroup", "IfcStructuralLoadGroup")
    }

    /// `IsLinear`, always mandatory: whether the results came from a linear analysis.
    pub fn is_linear(&self) -> StructuralResult<bool> {
        self.validate_semantics()?;
        self.record.required_bool("IsLinear")
    }

    /// Validates the `USERDEFINED`-requires-`ObjectType` rule for IFC4/IFC4X3.
    ///
    /// Fails with [`crate::StructuralError::SemanticViolation`] if
    /// `TheoryType` is `USERDEFINED` but `ObjectType` is unset. IFC2X3
    /// declares no such constraint here.
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
