//! IFC2X3 varying linear and planar actions.
//!
//! IFC2X3 declares `IfcStructuralLinearActionVarying` and
//! `IfcStructuralPlanarActionVarying`: the inherited `AppliedLoad` is the
//! first load, `SubsequentAppliedLoads` the rest, located along the
//! activity by `VaryingAppliedLoadLocation`. IFC4 and IFC4X3 declare
//! neither entity, so every accessor here returns `None` for them.

use ifc_model::{EntityId, Value};

use crate::action::StructuralAction;
use crate::error::{StructuralError, StructuralResult};
use crate::load::StaticLoad;
use crate::view::Record;

const LINEAR_VARYING: &str = "IfcStructuralLinearActionVarying";
const PLANAR_VARYING: &str = "IfcStructuralPlanarActionVarying";

impl<'m, 's> StructuralAction<'m, 's> {
    /// Whether the entity is an IFC2X3 `IfcStructuralLinearActionVarying`
    /// or `IfcStructuralPlanarActionVarying`.
    ///
    /// Always `false` under IFC4 and IFC4X3, which declare neither entity.
    #[must_use]
    pub fn is_varying(&self) -> bool {
        self.subsequent_minimum().is_some()
    }

    /// `VaryingAppliedLoadLocation`, the `IfcShapeAspect` locating the
    /// varying loads along the activity.
    ///
    /// `None` for a non-varying action and under IFC4/IFC4X3. Fails with
    /// [`StructuralError::InvalidValue`] when the mandatory reference is
    /// unset, and with the reference errors when it dangles or names
    /// another type.
    pub fn varying_applied_load_location(&self) -> StructuralResult<Option<EntityId>> {
        if !self.is_varying() {
            return Ok(None);
        }
        self.validate_semantics()?;
        self.record
            .required_ref("VaryingAppliedLoadLocation", "IfcShapeAspect")
            .map(Some)
    }

    /// `SubsequentAppliedLoads`, the loads after the inherited `AppliedLoad`,
    /// in list order and projected as [`StaticLoad`].
    ///
    /// `None` for a non-varying action and under IFC4/IFC4X3. The schema
    /// minimum is enforced: `LIST [1:?]` on the linear form, `LIST [2:?]`
    /// on the planar form, otherwise [`StructuralError::InvalidCardinality`].
    /// A list member that dangles fails with
    /// [`StructuralError::DanglingReference`]; one that is not an
    /// `IfcStructuralLoad` fails with [`StructuralError::WrongReferenceType`].
    /// The derived `VaryingAppliedLoads` is `AppliedLoad` followed by this
    /// list.
    pub fn subsequent_applied_loads(&self) -> StructuralResult<Option<Vec<StaticLoad<'m, 's>>>> {
        let Some(minimum) = self.subsequent_minimum() else {
            return Ok(None);
        };
        self.validate_semantics()?;
        const ATTRIBUTE: &str = "SubsequentAppliedLoads";
        const EXPECTED: &str = "LIST of IfcStructuralLoad references";
        let values = match self.record.value(ATTRIBUTE)?.unwrap_typed() {
            Value::List(values) => values,
            _ => {
                return Err(StructuralError::InvalidValue {
                    entity: self.record.id,
                    attribute: ATTRIBUTE,
                    expected: EXPECTED,
                })
            }
        };
        if values.len() < minimum {
            return Err(StructuralError::InvalidCardinality {
                entity: self.record.id,
                attribute: ATTRIBUTE,
                minimum,
                maximum: None,
                actual: values.len(),
            });
        }
        values
            .iter()
            .map(|value| {
                let Value::Ref(target) = value.unwrap_typed() else {
                    return Err(StructuralError::InvalidValue {
                        entity: self.record.id,
                        attribute: ATTRIBUTE,
                        expected: EXPECTED,
                    });
                };
                self.subsequent_load(ATTRIBUTE, *target)
            })
            .collect::<StructuralResult<Vec<_>>>()
            .map(Some)
    }

    fn subsequent_load(
        &self,
        attribute: &'static str,
        target: EntityId,
    ) -> StructuralResult<StaticLoad<'m, 's>> {
        let record = Record::new(
            self.record.model,
            self.record.schema,
            target,
            "IfcStructuralLoad",
        )
        .map_err(|error| match error {
            StructuralError::EntityNotFound { .. } => StructuralError::DanglingReference {
                entity: self.record.id,
                attribute,
                target,
            },
            StructuralError::WrongType { actual, .. } => StructuralError::WrongReferenceType {
                entity: self.record.id,
                attribute,
                target,
                expected: "IfcStructuralLoad",
                actual,
            },
            other => other,
        })?;
        StaticLoad::from_record(record)
    }

    /// The `SubsequentAppliedLoads` list minimum, or `None` when this is not
    /// a varying action in the view's schema.
    fn subsequent_minimum(&self) -> Option<usize> {
        let schema = self.record.schema;
        let type_name = &self.record.entity.type_name;
        if schema.entity(LINEAR_VARYING).is_some() && schema.is_a(type_name, LINEAR_VARYING) {
            Some(1)
        } else if schema.entity(PLANAR_VARYING).is_some() && schema.is_a(type_name, PLANAR_VARYING)
        {
            Some(2)
        } else {
            None
        }
    }
}
