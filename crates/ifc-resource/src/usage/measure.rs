//! `IfcMeasureWithUnit` -- the IFC2X3 resource `BaseQuantity` form.
//!
//! IFC2X3 TC1 declares `IfcConstructionResource.BaseQuantity` as an
//! `IfcMeasureWithUnit` (`ValueComponent : IfcValue`, `UnitComponent :
//! IfcUnit`). The entity exists unchanged in IFC4 and IFC4X3, so the
//! projection reads it under every supported release. It preserves the
//! authored value; it converts no units.

use ifc_model::{EntityId, Value};

use crate::error::{ResourceError, ResourceResult};
use crate::view::Record;

/// A borrowed, schema-resolved `IfcMeasureWithUnit` projection.
#[derive(Debug, Clone, Copy)]
pub struct MeasureWithUnit<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> MeasureWithUnit<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of the projected `IfcMeasureWithUnit`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `ValueComponent` attribute: the typed `IfcValue` as authored, for
    /// example `IFCCOUNTMEASURE(4.)`.
    ///
    /// The result is always a [`Value::Typed`] whose type name the release
    /// accepts as an `IfcValue`; an untyped literal, or a wrapper naming a
    /// type outside `IfcValue`, is refused with
    /// [`ResourceError::InvalidValue`] rather than guessed.
    pub fn value_component(&self) -> ResourceResult<&'m Value> {
        let value = self.record.value("ValueComponent")?;
        match value {
            Value::Typed { type_name, .. }
                if self.record.schema.accepts_type("IfcValue", type_name) =>
            {
                Ok(value)
            }
            _ => Err(ResourceError::InvalidValue {
                entity: self.record.id,
                attribute: "ValueComponent",
                expected: "typed IfcValue",
            }),
        }
    }

    /// The `UnitComponent` attribute: an `IfcDerivedUnit`, `IfcNamedUnit`
    /// or `IfcMonetaryUnit` reference.
    pub fn unit_component(&self) -> ResourceResult<EntityId> {
        self.record.required_ref_select(
            "UnitComponent",
            "IfcUnit",
            &["IfcDerivedUnit", "IfcNamedUnit", "IfcMonetaryUnit"],
        )
    }
}
