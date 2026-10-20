//! The walk: every attribute of every entity, read against the release's
//! layout, rescaled, kept or refused.

use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_properties::{ExactPropertyError, SchemaVersion, UnitConversion};
use ifc_schema::Schema;

use super::classify::{
    lookup, Classifier, Declared, Host, Measure, RealRole, PARAMETER_HOSTS, REVIEWED_REALS,
};
use super::georef::{self, GeorefPlan};
use super::report::{
    ConvertedKind, KeptReason, LengthUnitError, LengthUnitRefusal, RefusalReason, Tally,
};
use super::scope::{rule, Governed, Rule, Scope};
use super::space::{self, reference, Direction, Space};
use super::target::TargetUnits;

/// What one slot of an entity type is, computed once per type.
#[derive(Clone)]
struct SlotInfo {
    name: Arc<str>,
    declared: Declared,
    real: Option<RealRole>,
    unit: Rule,
    host: Option<Host>,
}

/// The conversions the walk applies.
struct Factors {
    length: UnitConversion,
    area: Option<UnitConversion>,
    volume: Option<UnitConversion>,
    inverse_length: UnitConversion,
    inverse_area: UnitConversion,
}

/// Plan the conversion of every value and stage it on `transaction`.
pub(super) fn plan(
    model: &Model,
    version: SchemaVersion,
    units: &TargetUnits,
    tally: &mut Tally,
    transaction: &mut Transaction,
) -> Result<(), LengthUnitError> {
    let schema = ifc_schema::for_version(version).map_err(|_| {
        LengthUnitError::Schema(ExactPropertyError::UnsupportedSchema {
            schema: format!("{version:?}"),
        })
    })?;
    let mut walk = Walk {
        model,
        schema,
        classifier: Classifier::new(version, schema),
        layouts: HashMap::new(),
        refusals: Vec::new(),
        factors: Factors {
            length: units.length,
            area: units.area.as_ref().map(|(_, conversion)| *conversion),
            volume: units.volume.as_ref().map(|(_, conversion)| *conversion),
            inverse_length: units.length.inverse(),
            inverse_area: units
                .length
                .powi(2)
                .expect("a length conversion has no offset")
                .inverse(),
        },
        edits: Vec::new(),
    };
    walk.contexts();
    let scope = Scope::build(schema, model, &mut walk.refusals);
    let georef = georef::plan(
        version,
        &mut walk.classifier,
        model,
        &units.length,
        &mut walk.refusals,
    );
    for (id, entity) in model.iter() {
        walk.entity(
            id,
            &entity.type_name,
            &entity.attributes,
            &scope,
            &georef,
            tally,
        );
    }
    for (id, name, value) in &georef.edits {
        let Some(record) = model.get(*id) else {
            continue;
        };
        let layout = walk.layout(&record.type_name);
        if let Some(slot) = layout
            .iter()
            .position(|info| info.name.eq_ignore_ascii_case(name))
        {
            walk.edits.push((*id, slot, value.clone()));
            tally.converted(
                &record.type_name,
                &layout[slot].name,
                ConvertedKind::MapScale,
                1,
            );
        }
    }
    if !walk.refusals.is_empty() {
        return Err(LengthUnitError::Refused(walk.refusals));
    }
    for (id, slot, value) in walk.edits {
        transaction.set_attribute(id, slot, value);
    }
    georef::stage_new_scales(
        schema,
        model,
        &georef,
        transaction,
        &mut |ty, attr, kind| {
            tally.converted(ty, attr, kind, 1);
        },
    );
    Ok(())
}

struct Walk<'m> {
    model: &'m Model,
    schema: &'static Schema,
    classifier: Classifier,
    layouts: HashMap<Arc<str>, Option<Arc<[SlotInfo]>>>,
    refusals: Vec<LengthUnitRefusal>,
    factors: Factors,
    edits: Vec<(EntityId, usize, Value)>,
}

/// Per-slot state while converting one attribute.
struct SlotState {
    governed: Governed,
    kept: Option<KeptReason>,
    /// The parameter decision, once made: `Some(true)` rescale, `Some(false)`
    /// keep.
    parameter: Option<Option<bool>>,
    counts: HashMap<ConvertedKind, usize>,
    kept_counts: HashMap<KeptReason, usize>,
    refused: bool,
}

impl Walk<'_> {
    fn refuse(
        &mut self,
        entity: EntityId,
        entity_type: &Arc<str>,
        attribute: Option<&Arc<str>>,
        reason: RefusalReason,
    ) {
        self.refusals.push(LengthUnitRefusal {
            entity,
            entity_type: entity_type.clone(),
            attribute: attribute.cloned(),
            reason,
        });
    }

    /// A context other than the project with units of its own.
    fn contexts(&mut self) {
        let model = self.model;
        for (id, entity) in model.iter() {
            if self.schema.is_a(&entity.type_name, "IFCCONTEXT")
                && !self.schema.is_a(&entity.type_name, "IFCPROJECT")
                && !matches!(
                    space::attribute(self.schema, model, id, "UnitsInContext"),
                    None | Some(Value::Null)
                )
            {
                let name: Arc<str> = "UnitsInContext".into();
                self.refuse(
                    id,
                    &entity.type_name,
                    Some(&name),
                    RefusalReason::ContextUnits,
                );
            }
        }
    }

    fn layout(&mut self, entity_type: &Arc<str>) -> Arc<[SlotInfo]> {
        if let Some(Some(layout)) = self.layouts.get(entity_type) {
            return layout.clone();
        }
        let schema = self.schema;
        let layout: Arc<[SlotInfo]> = (0..schema.attribute_count(entity_type))
            .filter_map(|slot| schema.attribute_at(entity_type, slot))
            .map(|attribute| SlotInfo {
                name: attribute.name.as_str().into(),
                declared: self.classifier.declared(&attribute.type_name),
                real: lookup(schema, REVIEWED_REALS, entity_type, &attribute.name),
                unit: rule(schema, entity_type, &attribute.name),
                host: lookup(schema, PARAMETER_HOSTS, entity_type, &attribute.name),
            })
            .collect();
        self.layouts
            .insert(entity_type.clone(), Some(layout.clone()));
        layout
    }

    fn entity(
        &mut self,
        id: EntityId,
        entity_type: &Arc<str>,
        values: &[Value],
        scope: &Scope,
        georef: &GeorefPlan,
        tally: &mut Tally,
    ) {
        let schema = self.schema;
        if schema.entity(entity_type).is_none() {
            self.refuse(id, entity_type, None, RefusalReason::NotInSchema);
            return;
        }
        let layout = self.layout(entity_type);
        if layout.len() != values.len() {
            self.refuse(id, entity_type, None, RefusalReason::NotInSchema);
            return;
        }
        if schema.is_a(entity_type, "IFCPCURVE") {
            if let Err((surface, surface_type)) = space::pcurve_surface(schema, self.model, id) {
                self.refuse(
                    id,
                    entity_type,
                    None,
                    RefusalReason::ParameterSpaceCurve {
                        surface,
                        surface_type,
                    },
                );
            }
        }
        let mut changed = false;
        for (slot, (value, info)) in values.iter().zip(layout.iter()).enumerate() {
            if matches!(value, Value::Null | Value::Derived) {
                continue;
            }
            if let Some(role) = info.real {
                if let Some(new) = self.real(id, entity_type, info, role, value, tally) {
                    self.edits.push((id, slot, new));
                    changed = true;
                }
                continue;
            }
            let kept = georef
                .kept_slots
                .get(&(id, info.name.clone()))
                .or_else(|| georef.kept_entities.get(&id))
                .copied();
            let mut state = SlotState {
                governed: scope.governed(info.unit, schema, self.model, id),
                kept,
                parameter: None,
                counts: HashMap::new(),
                kept_counts: HashMap::new(),
                refused: false,
            };
            let new = self.value(id, entity_type, info, value, &mut state, None);
            for (kind, count) in state.counts {
                tally.converted(entity_type, &info.name, kind, count);
            }
            for (reason, count) in state.kept_counts {
                tally.kept(entity_type, &info.name, reason, count);
            }
            if let Some(new) = new {
                self.edits.push((id, slot, new));
                changed = true;
            }
        }
        if changed {
            tally.entity_changed();
        }
    }

    /// A reviewed `REAL` attribute.
    fn real(
        &mut self,
        id: EntityId,
        entity_type: &Arc<str>,
        info: &SlotInfo,
        role: RealRole,
        value: &Value,
        tally: &mut Tally,
    ) -> Option<Value> {
        let (conversion, kind) = match role {
            RealRole::Unitless | RealRole::MapScale => return None,
            RealRole::Refuse => {
                let present = !matches!(value, Value::List(items) if items.is_empty());
                if present {
                    let reason = if schema_is(self.schema, entity_type, "IFCPOLYNOMIALCURVE") {
                        RefusalReason::PolynomialCoefficients
                    } else {
                        RefusalReason::TextureGeneratorParameters
                    };
                    self.refuse(id, entity_type, Some(&info.name), reason);
                }
                return None;
            }
            RealRole::Length => (self.factors.length, ConvertedKind::Length),
            RealRole::InverseLength => (self.factors.inverse_length, ConvertedKind::InverseLength),
            RealRole::InverseArea => (self.factors.inverse_area, ConvertedKind::InverseArea),
        };
        let mut count = 0;
        let new = map_numbers(value, &mut |x| {
            count += 1;
            conversion.apply(x)
        });
        if count > 0 {
            tally.converted(entity_type, &info.name, kind, count);
        }
        new
    }

    /// Convert one value of a slot; `element` is its index in the slot's
    /// top-level list.
    fn value(
        &mut self,
        id: EntityId,
        entity_type: &Arc<str>,
        info: &SlotInfo,
        value: &Value,
        state: &mut SlotState,
        element: Option<usize>,
    ) -> Option<Value> {
        match value {
            Value::List(items) => {
                let mut changed = false;
                let converted: Vec<Value> = items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let at = element.or(Some(index));
                        match self.value(id, entity_type, info, item, state, at) {
                            Some(new) => {
                                changed = true;
                                new
                            }
                            None => item.clone(),
                        }
                    })
                    .collect();
                changed.then_some(Value::List(converted))
            }
            Value::Typed {
                type_name,
                value: inner,
            } => {
                let measure = self.classifier.measure(type_name);
                let new = self.number(id, entity_type, info, inner, measure, state, element)?;
                Some(Value::Typed {
                    type_name: type_name.clone(),
                    value: Box::new(new),
                })
            }
            Value::Real(_) | Value::Integer(_) => {
                let measure = match &info.declared {
                    Declared::Plain(measure) => *measure,
                    Declared::Select {
                        numeric: Some(measure),
                        ..
                    } => *measure,
                    Declared::Select {
                        name,
                        numeric: None,
                    } => {
                        if !state.refused {
                            state.refused = true;
                            let reason = RefusalReason::UntypedSelectValue {
                                select: name.clone(),
                            };
                            self.refuse(id, entity_type, Some(&info.name), reason);
                        }
                        return None;
                    }
                };
                self.number(id, entity_type, info, value, measure, state, element)
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn number(
        &mut self,
        id: EntityId,
        entity_type: &Arc<str>,
        info: &SlotInfo,
        value: &Value,
        measure: Option<Measure>,
        state: &mut SlotState,
        element: Option<usize>,
    ) -> Option<Value> {
        let x = match value {
            Value::Real(x) => *x,
            #[allow(clippy::cast_precision_loss)]
            Value::Integer(x) => *x as f64,
            _ => return None,
        };
        let explicit = match &state.governed {
            Governed::Project => false,
            Governed::Explicit => true,
            Governed::PerElement(flags) => {
                element.and_then(|i| flags.get(i).copied()).unwrap_or(false)
            }
        };
        let (conversion, kind) = match measure? {
            Measure::Parameter => {
                if !self.parameter_scales(id, entity_type, info, state) {
                    *state
                        .kept_counts
                        .entry(KeptReason::UnitlessParameter)
                        .or_default() += 1;
                    return None;
                }
                (self.factors.length, ConvertedKind::Length)
            }
            _ if explicit => {
                *state
                    .kept_counts
                    .entry(KeptReason::ExplicitUnit)
                    .or_default() += 1;
                return None;
            }
            _ if state.kept.is_some() => {
                let reason = state.kept.expect("checked");
                *state.kept_counts.entry(reason).or_default() += 1;
                return None;
            }
            Measure::Length => (self.factors.length, ConvertedKind::Length),
            Measure::Area => match self.factors.area {
                Some(area) => (area, ConvertedKind::Area),
                None => {
                    *state
                        .kept_counts
                        .entry(KeptReason::UnitNotFollowed)
                        .or_default() += 1;
                    return None;
                }
            },
            Measure::Volume => match self.factors.volume {
                Some(volume) => (volume, ConvertedKind::Volume),
                None => {
                    *state
                        .kept_counts
                        .entry(KeptReason::UnitNotFollowed)
                        .or_default() += 1;
                    return None;
                }
            },
        };
        if conversion.is_identity() {
            return None;
        }
        *state.counts.entry(kind).or_default() += 1;
        Some(Value::Real(conversion.apply(x)))
    }

    /// Whether this slot's parameter values are lengths; refuses (once per
    /// slot) when their space is undefined.
    fn parameter_scales(
        &mut self,
        id: EntityId,
        entity_type: &Arc<str>,
        info: &SlotInfo,
        state: &mut SlotState,
    ) -> bool {
        if let Some(decision) = state.parameter {
            return decision.unwrap_or(false);
        }
        let schema = self.schema;
        let model = self.model;
        let host_of = |name: &str| {
            reference(schema, model, id, name).ok_or_else(|| {
                (
                    id,
                    entity_type.clone(),
                    Arc::<str>::from("the host is not a reference"),
                )
            })
        };
        let decision = match info.host {
            None | Some(Host::Unitless) => Ok(Space::Unitless),
            Some(Host::Undefined(detail)) => Err((id, entity_type.clone(), detail.into())),
            Some(Host::Curve(name)) => {
                host_of(name).and_then(|curve| space::curve(schema, model, curve))
            }
            Some(Host::SurfaceU(name)) => host_of(name)
                .and_then(|surface| space::surface(schema, model, surface, Direction::U)),
            Some(Host::SurfaceV(name)) => host_of(name)
                .and_then(|surface| space::surface(schema, model, surface, Direction::V)),
        };
        let scales = match decision {
            Ok(space) => Some(space == Space::Length),
            Err((host, host_type, detail)) => {
                let reason = RefusalReason::ParameterSpace {
                    host,
                    host_type,
                    detail,
                };
                self.refuse(id, entity_type, Some(&info.name), reason);
                None
            }
        };
        state.parameter = Some(scales);
        scales.unwrap_or(false)
    }
}

fn schema_is(schema: &Schema, entity_type: &str, ancestor: &str) -> bool {
    schema.is_a(entity_type, ancestor)
}

/// Apply `f` to every number in `value`; `None` when there is none.
fn map_numbers(value: &Value, f: &mut dyn FnMut(f64) -> f64) -> Option<Value> {
    match value {
        Value::Real(x) => Some(Value::Real(f(*x))),
        #[allow(clippy::cast_precision_loss)]
        Value::Integer(x) => Some(Value::Real(f(*x as f64))),
        Value::List(items) => {
            let mut any = false;
            let items = items
                .iter()
                .map(|item| match map_numbers(item, f) {
                    Some(new) => {
                        any = true;
                        new
                    }
                    None => item.clone(),
                })
                .collect();
            any.then_some(Value::List(items))
        }
        Value::Typed { type_name, value } => map_numbers(value, f).map(|new| Value::Typed {
            type_name: type_name.clone(),
            value: Box::new(new),
        }),
        _ => None,
    }
}
