//! The numbers an operation's attributes state, from SI (#638).
//!
//! Every attribute is read against the release's table, as length
//! conversion reads it (`unit_rules`): a reviewed `REAL` follows its role, a
//! measure converts into the unit that applies, an `IfcParameterValue`
//! follows its curve's or surface's parameter space. The unit that applies is
//! the explicit unit of the property, quantity or measure the number belongs
//! to, else the project unit, resolved against the model as it was before the
//! batch.

use ifc_model::{EntityId, Value};
use ifc_properties::{UnitConversion, UnitConversionError};
use ifc_schema::Schema;

use super::{SiInput, LENGTH};
use crate::authoring::overlay::Overlay;
use crate::authoring::si_refusal::SiInputRefusal;
use crate::authoring::{AuthoringFailure as F, NamedValues};
use crate::unit_rules::classify::{
    lookup, Declared, Host, RealRole, SiMeasure, PARAMETER_HOSTS, REVIEWED_REALS,
};
use crate::unit_rules::governing::{is_temperature_difference, rule, Rule};
use crate::unit_rules::space::{self, reference, Direction, Space};

const PLANE_ANGLE: &str = "IFCPLANEANGLEMEASURE";

/// Where one attribute value sits: what it needs to find its unit or host.
struct Slot<'a, 'v> {
    view: &'a Overlay<'v>,
    /// The entity type written, upper case.
    entity_type: &'a str,
    /// The attribute, as the release names it.
    attribute: &'a str,
    /// The attribute's declared type (the element type of an aggregate).
    declared: &'a str,
    /// The operation's own attributes, before conversion.
    own: &'a NamedValues,
    /// The record being edited, if any.
    record: Option<EntityId>,
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Real(x) => Some(*x),
        #[allow(clippy::cast_precision_loss)]
        Value::Integer(x) => Some(*x as f64),
        _ => None,
    }
}

fn has_number(value: &Value) -> bool {
    match value {
        Value::List(items) => items.iter().any(has_number),
        Value::Typed { value, .. } => has_number(value),
        other => number(other).is_some(),
    }
}

fn refuse(slot: &Slot<'_, '_>, reason: SiInputRefusal) -> F {
    F::SiInput {
        entity_type: Some(slot.entity_type.to_owned()),
        attribute: Some(slot.attribute.to_owned()),
        reason,
    }
}

impl SiInput<'_> {
    /// Convert the numbers of `attributes`, the named values of an operation
    /// writing an entity of `entity_type`; `record` is the entity an edit
    /// changes.
    pub(super) fn attributes(
        &mut self,
        view: &Overlay<'_>,
        entity_type: &str,
        record: Option<EntityId>,
        attributes: &mut NamedValues,
    ) -> Result<(), F> {
        let schema = self.classifier.schema();
        let upper = entity_type.to_ascii_uppercase();
        let definitions = schema.attributes(&upper);
        if definitions.is_empty() {
            // An unknown type: the planner refuses it.
            return Ok(());
        }
        let own = attributes.clone();
        if schema.is_a(&upper, "IFCPCURVE") {
            self.pcurve(view, &upper, record, &own)?;
        }
        for (name, value) in attributes.iter_mut() {
            let Some(definition) = definitions
                .iter()
                .find(|definition| definition.name.eq_ignore_ascii_case(name))
            else {
                continue;
            };
            if matches!(value, Value::Null | Value::Derived) {
                continue;
            }
            let slot = Slot {
                view,
                entity_type: &upper,
                attribute: &definition.name,
                declared: &definition.type_name,
                own: &own,
                record,
            };
            if let Some(role) = lookup(schema, REVIEWED_REALS, &upper, &definition.name) {
                self.real(&slot, role, value)?;
            } else if schema.is_a(&upper, "IFCCOORDINATEOPERATION") && has_number(value) {
                return Err(refuse(&slot, SiInputRefusal::MapConversion));
            } else {
                self.walk(&slot, value)?;
            }
        }
        Ok(())
    }

    /// An `IfcPcurve` whose reference curve states `(u, v)` as lengths needs
    /// a surface parameterised by lengths in both directions.
    fn pcurve(
        &self,
        view: &Overlay<'_>,
        entity_type: &str,
        record: Option<EntityId>,
        own: &NamedValues,
    ) -> Result<(), F> {
        let schema = self.classifier.schema();
        let Some(surface) = host(schema, view, record, own, "BasisSurface") else {
            return Ok(());
        };
        space::length_surface(schema, view, surface).map_err(|(surface, surface_type)| F::SiInput {
            entity_type: Some(entity_type.to_owned()),
            attribute: Some("BasisSurface".to_owned()),
            reason: SiInputRefusal::ParameterSpaceCurve {
                surface,
                surface_type: surface_type.to_string(),
            },
        })
    }

    /// A reviewed `REAL` attribute.
    fn real(&mut self, slot: &Slot<'_, '_>, role: RealRole, value: &mut Value) -> Result<(), F> {
        let length = |this: &mut Self| this.conversion(LENGTH, None, slot);
        match role {
            RealRole::Unitless => Ok(()),
            RealRole::MapScale => Err(refuse(slot, SiInputRefusal::MapConversion)),
            RealRole::Refuse => {
                if matches!(value, Value::List(items) if items.is_empty()) {
                    return Ok(());
                }
                let reason = if self
                    .classifier
                    .schema()
                    .is_a(slot.entity_type, "IFCPOLYNOMIALCURVE")
                {
                    SiInputRefusal::PolynomialCoefficients
                } else {
                    SiInputRefusal::TextureGeneratorParameters
                };
                Err(refuse(slot, reason))
            }
            RealRole::Length => {
                let conversion = length(self)?;
                self.map(slot, LENGTH, value, &|x| conversion.apply(x))
            }
            RealRole::InverseLength => {
                let conversion = length(self)?.inverse();
                self.map(slot, LENGTH, value, &|x| conversion.apply(x))
            }
            RealRole::InverseArea => {
                let conversion = length(self)?.inverse();
                self.map(slot, LENGTH, value, &|x| {
                    conversion.apply(conversion.apply(x))
                })
            }
        }
    }

    /// Apply `convert` to every number of `value`; a number that is not
    /// finite is refused.
    fn map(
        &mut self,
        slot: &Slot<'_, '_>,
        measure: &str,
        value: &mut Value,
        convert: &dyn Fn(f64) -> f64,
    ) -> Result<(), F> {
        match value {
            Value::List(items) => items
                .iter_mut()
                .try_for_each(|item| self.map(slot, measure, item, convert)),
            Value::Typed { value, .. } => self.map(slot, measure, value, convert),
            other => {
                if let Some(x) = number(other) {
                    if !x.is_finite() {
                        return Err(self.failed(
                            slot,
                            measure,
                            UnitConversionError::NonFinite { value: x },
                        ));
                    }
                    *other = Value::Real(convert(x));
                }
                Ok(())
            }
        }
    }

    /// Walk a value of the slot's declared type, converting its numbers.
    fn walk(&mut self, slot: &Slot<'_, '_>, value: &mut Value) -> Result<(), F> {
        match value {
            Value::List(items) => items.iter_mut().try_for_each(|item| self.walk(slot, item)),
            Value::Typed {
                type_name,
                value: payload,
            } => {
                let measure = type_name.clone();
                if number(payload).is_some() {
                    self.number(slot, &measure, payload)?;
                } else if matches!(payload.as_ref(), Value::List(_)) {
                    // A list-valued measure (`IfcComplexNumber`): kept only
                    // where the reviewed table says so.
                    match self.classifier.si_measure(&measure) {
                        SiMeasure::Unchanged => {}
                        SiMeasure::Refuse(error) => {
                            return Err(self.failed(
                                slot,
                                &measure,
                                UnitConversionError::Resolve(error),
                            ))
                        }
                        _ => {
                            let error = ifc_properties::ExactUnitError::UnmappedMeasureType {
                                measure_type: measure.clone(),
                            };
                            return Err(self.failed(
                                slot,
                                &measure,
                                UnitConversionError::Resolve(error),
                            ));
                        }
                    }
                }
                Ok(())
            }
            Value::Real(_) | Value::Integer(_) => {
                let measure = match self.classifier.declared(slot.declared) {
                    Declared::Select {
                        member: Some(member),
                        ..
                    } => member.to_string(),
                    Declared::Select { name, .. } => {
                        return Err(refuse(
                            slot,
                            SiInputRefusal::UntypedSelectValue {
                                select: name.to_string(),
                            },
                        ))
                    }
                    Declared::Plain(_) => slot.declared.to_owned(),
                };
                self.number(slot, &measure, value)
            }
            _ => Ok(()),
        }
    }

    /// Convert the number `value` of `measure`.
    fn number(&mut self, slot: &Slot<'_, '_>, measure: &str, value: &mut Value) -> Result<(), F> {
        match self.classifier.si_measure(measure) {
            SiMeasure::Unchanged => Ok(()),
            SiMeasure::Refuse(error) => {
                Err(self.failed(slot, measure, UnitConversionError::Resolve(error)))
            }
            SiMeasure::Parameter => match self.parameter_space(slot)? {
                Space::Unitless => Ok(()),
                Space::Length => self.convert_in(slot, LENGTH, None, value),
                Space::PlaneAngle => self.convert_in(slot, PLANE_ANGLE, None, value),
                Space::AngleRanges => Err(self.parameter_refusal(
                    slot,
                    "ranges of angles and of other units along one curve".to_owned(),
                )),
            },
            SiMeasure::Convert(converted) => {
                let schema = self.classifier.schema();
                if matches!(value, Value::Integer(_))
                    && schema
                        .resolve_defined(&converted)
                        .eq_ignore_ascii_case("INTEGER")
                {
                    return Err(refuse(
                        slot,
                        SiInputRefusal::IntegerMeasure {
                            measure: converted.to_string(),
                        },
                    ));
                }
                let unit = self.unit_of(slot)?;
                self.convert_in(slot, &converted, unit, value)
            }
        }
    }

    fn convert_in(
        &mut self,
        slot: &Slot<'_, '_>,
        measure: &str,
        unit: Option<EntityId>,
        value: &mut Value,
    ) -> Result<(), F> {
        let mut conversion = self.conversion(measure, unit, slot)?;
        if is_temperature_difference(self.classifier.schema(), slot.entity_type, slot.attribute) {
            // A difference has no offset: 10 K is 10 degrees Celsius.
            conversion.offset = 0.0;
        }
        self.map(slot, measure, value, &|x| {
            if conversion.is_identity() {
                x
            } else {
                conversion.apply(x)
            }
        })
    }

    fn conversion(
        &mut self,
        measure: &str,
        unit: Option<EntityId>,
        slot: &Slot<'_, '_>,
    ) -> Result<UnitConversion, F> {
        self.resolve(measure, unit, slot.entity_type, slot.attribute)
    }

    fn failed(&self, slot: &Slot<'_, '_>, measure: &str, error: UnitConversionError) -> F {
        F::UnitConversion {
            entity_type: slot.entity_type.to_owned(),
            attribute: slot.attribute.to_owned(),
            measure: measure.to_owned(),
            error: Box::new(error),
        }
    }

    /// The explicit unit the value is stated in, if any.
    fn unit_of(&self, slot: &Slot<'_, '_>) -> Result<Option<EntityId>, F> {
        let schema = self.classifier.schema();
        let unit = match rule(schema, slot.entity_type, slot.attribute) {
            Rule::Project => None,
            Rule::UnitAttribute(name) => host(schema, slot.view, slot.record, slot.own, name),
            Rule::Enumeration => host(
                schema,
                slot.view,
                slot.record,
                slot.own,
                "EnumerationReference",
            )
            .and_then(|enumeration| reference(schema, slot.view, enumeration, "Unit")),
            Rule::Series | Rule::Rows => {
                return Err(refuse(slot, SiInputRefusal::UnitStatedElsewhere))
            }
        };
        if let Some(unit) = unit {
            if self.model.get(unit).is_none() && slot.view.get(unit).is_some() {
                return Err(refuse(slot, SiInputRefusal::UnitCreatedInBatch { unit }));
            }
        }
        Ok(unit)
    }

    /// The parameter space of the curve or surface the slot's parameter
    /// values are in.
    fn parameter_space(&self, slot: &Slot<'_, '_>) -> Result<Space, F> {
        let schema = self.classifier.schema();
        let row = lookup(schema, PARAMETER_HOSTS, slot.entity_type, slot.attribute);
        let by = |name: &str| host(schema, slot.view, slot.record, slot.own, name);
        let (name, direction) = match row {
            None | Some(Host::Unitless) => return Ok(Space::Unitless),
            Some(Host::Undefined(detail)) => {
                return Err(self.parameter_refusal(slot, detail.to_owned()))
            }
            Some(Host::Curve(name)) => (name, None),
            Some(Host::SurfaceU(name)) => (name, Some(Direction::U)),
            Some(Host::SurfaceV(name)) => (name, Some(Direction::V)),
        };
        let Some(id) = by(name) else {
            return Err(self.parameter_refusal(slot, "the host is not a reference".to_owned()));
        };
        let found = match direction {
            None => space::curve(schema, slot.view, id),
            Some(direction) => space::surface(schema, slot.view, id, direction),
        };
        found.map_err(|(host, host_type, detail)| {
            refuse(
                slot,
                SiInputRefusal::ParameterSpace {
                    host: Some(host),
                    host_type: Some(host_type.to_string()),
                    detail: detail.to_string(),
                },
            )
        })
    }

    fn parameter_refusal(&self, slot: &Slot<'_, '_>, detail: String) -> F {
        refuse(
            slot,
            SiInputRefusal::ParameterSpace {
                host: None,
                host_type: None,
                detail,
            },
        )
    }
}

/// The entity attribute `name` refers to: the operation's own value, else the
/// edited record's.
fn host(
    schema: &Schema,
    view: &Overlay<'_>,
    record: Option<EntityId>,
    own: &NamedValues,
    name: &str,
) -> Option<EntityId> {
    match own.iter().find(|(own, _)| own.eq_ignore_ascii_case(name)) {
        Some((_, Value::Ref(id))) => Some(*id),
        Some(_) => None,
        None => reference(schema, view, record?, name),
    }
}
