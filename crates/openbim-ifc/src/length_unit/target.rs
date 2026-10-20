//! The new project units: what they are, their conversions from the old
//! ones, and writing them into the unit assignment.
//!
//! An SI target is an `IfcSIUnit` `METRE` with its prefix, and its area and
//! volume `SQUARE_METRE` and `CUBIC_METRE` with the same prefix. A named
//! target is an `IfcConversionBasedUnit` of that name whose
//! `ConversionFactor` is an `IfcMeasureWithUnit` over the unprefixed SI
//! unit, as IFC4's `IfcConversionBasedUnit` example builds the inch; its
//! area and volume are `square <name>` and `cubic <name>` the same way
//! (the IFC4 Table 673 designations where the table has them). An existing
//! unit with the same definition is reused.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, ReverseIndex, Transaction, Value};
use ifc_properties::{
    add_conversion_based_unit, add_dimensional_exponents, add_measure_with_unit, add_si_unit,
    named_units, unit_conversion, ConversionBasedUnitDraft, ConversionUnit, SiUnitDraft,
    UnitConversion,
};

use super::report::{LengthUnitError, LengthUnitOptions, Tally};

/// The kinds of project unit that change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Length,
    Area,
    Volume,
}

impl Kind {
    const fn unit_type(self) -> &'static str {
        match self {
            Self::Length => "LENGTHUNIT",
            Self::Area => "AREAUNIT",
            Self::Volume => "VOLUMEUNIT",
        }
    }

    const fn si_name(self) -> &'static str {
        match self {
            Self::Length => "METRE",
            Self::Area => "SQUARE_METRE",
            Self::Volume => "CUBIC_METRE",
        }
    }

    const fn measure(self) -> &'static str {
        match self {
            Self::Length => "IFCLENGTHMEASURE",
            Self::Area => "IFCAREAMEASURE",
            Self::Volume => "IFCVOLUMEMEASURE",
        }
    }

    const fn power(self) -> i32 {
        match self {
            Self::Length => 1,
            Self::Area => 2,
            Self::Volume => 3,
        }
    }
}

/// One new unit: how to write it.
#[derive(Debug, Clone, PartialEq)]
struct NewUnit {
    kind: Kind,
    /// `Some(prefix)` (or `Some(None)` unprefixed) for an SI unit.
    si_prefix: Option<Option<Arc<str>>>,
    /// The conversion-based unit's name and factor into the SI unit.
    named: Option<(Arc<str>, f64)>,
}

/// The planned units and conversions.
#[derive(Debug, Clone)]
pub(crate) struct TargetUnits {
    pub(crate) length: UnitConversion,
    pub(crate) area: Option<(ConversionUnit, UnitConversion)>,
    pub(crate) volume: Option<(ConversionUnit, UnitConversion)>,
    new: Vec<NewUnit>,
}

/// The SI prefix of an SI length target, `Some(None)` unprefixed; `None`
/// for a named target.
fn si_prefix(target: &ConversionUnit) -> Option<Option<Arc<str>>> {
    let name = target.name.as_ref();
    let (prefix, base) = match name.split_once(' ') {
        Some((prefix, base)) => (Some(prefix), base),
        None => (None, name),
    };
    (base == "METRE").then(|| prefix.map(Into::into))
}

/// A length unit this operation can write: SI metres or a named length.
pub(super) fn require_writable(target: &ConversionUnit) -> Result<(), LengthUnitError> {
    let named = named_units()
        .iter()
        .any(|unit| unit.name == target.name.as_ref() && unit.unit_type == "LENGTHUNIT");
    if si_prefix(target).is_some() || named {
        Ok(())
    } else {
        Err(LengthUnitError::Target {
            target: target.name.clone(),
            reason: "not an SI METRE or a named length unit; build it with \
                     ConversionUnit::parse or ConversionUnit::named",
        })
    }
}

impl TargetUnits {
    pub(super) fn plan(
        target: &ConversionUnit,
        old_length: &ConversionUnit,
        old_area: Option<&(EntityId, ConversionUnit)>,
        old_volume: Option<&(EntityId, ConversionUnit)>,
    ) -> Result<Self, LengthUnitError> {
        let failed = |unit_type: &str, error| LengthUnitError::ProjectUnit {
            unit_type: unit_type.into(),
            error,
        };
        let length =
            unit_conversion(old_length, target).map_err(|error| failed("LENGTHUNIT", error))?;
        let mut new = vec![new_unit(target, Kind::Length)];
        let mut follow = |old: Option<&(EntityId, ConversionUnit)>, kind: Kind| {
            let Some((_, old)) = old else {
                return Ok(None);
            };
            let unit = power_of(target, kind);
            let conversion =
                unit_conversion(old, &unit).map_err(|error| failed(kind.unit_type(), error))?;
            new.push(new_unit(target, kind));
            Ok::<_, LengthUnitError>(Some((unit, conversion)))
        };
        let area = follow(old_area, Kind::Area)?;
        let volume = follow(old_volume, Kind::Volume)?;
        Ok(Self {
            length,
            area,
            volume,
            new,
        })
    }

    /// Whether the length unit stays: then nothing changes, and areas and
    /// volumes keep their units too.
    pub(super) fn is_identity(&self) -> bool {
        self.length.is_identity()
    }
}

/// The target raised to the power of `kind`, as a conversion unit.
fn power_of(target: &ConversionUnit, kind: Kind) -> ConversionUnit {
    let mut unit = target.clone();
    let power = kind.power();
    unit.dimensions = [power, 0, 0, 0, 0, 0, 0];
    unit.mantissa = target.mantissa.powi(power);
    unit.decimal_exponent = target.decimal_exponent * power;
    unit.unit_type = Some(kind.unit_type().into());
    unit.name = match (si_prefix(target), kind) {
        (_, Kind::Length) => target.name.clone(),
        (Some(Some(prefix)), _) => format!("{prefix} {}", kind.si_name()).into(),
        (Some(None), _) => kind.si_name().into(),
        (None, Kind::Area) => format!("square {}", target.name).into(),
        (None, Kind::Volume) => format!("cubic {}", target.name).into(),
    };
    unit
}

fn new_unit(target: &ConversionUnit, kind: Kind) -> NewUnit {
    let unit = power_of(target, kind);
    match si_prefix(target) {
        Some(prefix) => NewUnit {
            kind,
            si_prefix: Some(prefix),
            named: None,
        },
        None => NewUnit {
            kind,
            si_prefix: None,
            named: Some((unit.name.clone(), unit.scale())),
        },
    }
}

/// Stage the new units, swap them into the assignment, and remove the
/// replaced ones that nothing else uses.
#[allow(clippy::too_many_arguments)]
pub(super) fn stage(
    model: &Model,
    units: &TargetUnits,
    old_length: EntityId,
    old_area: Option<(EntityId, ConversionUnit)>,
    old_volume: Option<(EntityId, ConversionUnit)>,
    options: &LengthUnitOptions,
    transaction: &mut Transaction,
    tally: &mut Tally,
) -> Result<(), LengthUnitError> {
    let mut writer = Writer::new(model);
    let mut replaced = HashMap::new();
    let mut created = [None, None, None];
    for new in &units.new {
        let id = writer.unit(new, transaction)?;
        let old = match new.kind {
            Kind::Length => Some(old_length),
            Kind::Area => old_area.as_ref().map(|(id, _)| *id),
            Kind::Volume => old_volume.as_ref().map(|(id, _)| *id),
        };
        if let Some(old) = old {
            replaced.insert(old, id);
        }
        created[new.kind.power() as usize - 1] = Some(id);
    }
    let length = created[0].expect("the length unit is always planned");
    tally.units(length, created[1], created[2]);
    let assignment = assignment(model)?;
    let record = model
        .get(assignment.0)
        .expect("the assignment was found in the model");
    let Some(Value::List(members)) = record.attributes.get(assignment.1) else {
        return Err(unresolved("IfcUnitAssignment.Units is not a list"));
    };
    let members: Vec<Value> = members
        .iter()
        .map(|member| match member {
            Value::Ref(id) => Value::Ref(replaced.get(id).copied().unwrap_or(*id)),
            other => other.clone(),
        })
        .collect();
    transaction.set_attribute(assignment.0, assignment.1, Value::List(members));
    if options.remove_replaced_units {
        let removed = orphans(model, assignment.0, &replaced, &writer.reused);
        for id in &removed {
            transaction.remove(*id);
        }
        tally.removing(removed);
    }
    Ok(())
}

fn unresolved(detail: &str) -> LengthUnitError {
    LengthUnitError::ProjectUnit {
        unit_type: "LENGTHUNIT".into(),
        error: ifc_properties::UnitConversionError::Resolve(
            ifc_properties::ExactUnitError::Structure(
                ifc_properties::ExactPropertyError::UnsupportedSchema {
                    schema: detail.into(),
                },
            ),
        ),
    }
}

/// The project's `IfcUnitAssignment` and the slot of its `Units`.
fn assignment(model: &Model) -> Result<(EntityId, usize), LengthUnitError> {
    let project = *model
        .ids_of_type("IFCPROJECT")
        .first()
        .ok_or_else(|| unresolved("no IfcProject"))?;
    let schema = ifc_properties::exact_schema(model)
        .ok()
        .and_then(|version| ifc_schema::for_version(version).ok())
        .ok_or_else(|| unresolved("no bundled schema"))?;
    let assignment = super::space::reference(schema, model, project, "UnitsInContext")
        .ok_or_else(|| unresolved("IfcProject.UnitsInContext is not a reference"))?;
    Ok((assignment, 0))
}

/// Writes units, reusing ones the model already has.
struct Writer<'m> {
    model: &'m Model,
    /// Existing entities referenced by the new units.
    reused: HashSet<EntityId>,
    staged_si: HashMap<(Kind, Option<Arc<str>>), EntityId>,
}

impl<'m> Writer<'m> {
    fn new(model: &'m Model) -> Self {
        Self {
            model,
            reused: HashSet::new(),
            staged_si: HashMap::new(),
        }
    }

    fn unit(
        &mut self,
        new: &NewUnit,
        transaction: &mut Transaction,
    ) -> Result<EntityId, LengthUnitError> {
        if let Some(prefix) = &new.si_prefix {
            return self.si(new.kind, prefix.clone(), transaction);
        }
        let (name, factor) = new.named.clone().expect("a named unit when not SI");
        let base = self.si(new.kind, None, transaction)?;
        let power = i64::from(new.kind.power());
        let dimensions = add_dimensional_exponents(transaction, [power, 0, 0, 0, 0, 0, 0]);
        let measure = add_measure_with_unit(
            transaction,
            Value::Typed {
                type_name: new.kind.measure().into(),
                value: Box::new(Value::Real(factor)),
            },
            base,
        )
        .map_err(LengthUnitError::Authoring)?;
        add_conversion_based_unit(
            transaction,
            ConversionBasedUnitDraft::new(new.kind.unit_type(), &name, measure, dimensions),
        )
        .map_err(LengthUnitError::Authoring)
    }

    /// An `IfcSIUnit` of `kind` with `prefix`: an existing one, else new.
    fn si(
        &mut self,
        kind: Kind,
        prefix: Option<Arc<str>>,
        transaction: &mut Transaction,
    ) -> Result<EntityId, LengthUnitError> {
        if let Some(id) = self.staged_si.get(&(kind, prefix.clone())) {
            return Ok(*id);
        }
        let existing = self
            .model
            .ids_of_type("IFCSIUNIT")
            .iter()
            .copied()
            .find(|id| {
                let attributes = &self.model.get(*id).expect("indexed").attributes;
                let is =
                    |slot: usize, expected: Option<&str>| match (attributes.get(slot), expected) {
                        (Some(Value::Enum(value)), Some(expected)) => {
                            value.eq_ignore_ascii_case(expected)
                        }
                        (Some(Value::Null), None) => true,
                        _ => false,
                    };
                attributes.len() == 4
                    && is(1, Some(kind.unit_type()))
                    && is(2, prefix.as_deref())
                    && is(3, Some(kind.si_name()))
            });
        let id = match existing {
            Some(id) => {
                self.reused.insert(id);
                id
            }
            None => {
                let mut draft = SiUnitDraft::new(kind.unit_type(), kind.si_name());
                if let Some(prefix) = prefix.as_deref() {
                    draft = draft.prefix(prefix);
                }
                add_si_unit(transaction, draft).map_err(LengthUnitError::Authoring)?
            }
        };
        self.staged_si.insert((kind, prefix), id);
        Ok(id)
    }
}

/// The replaced units nothing but the assignment uses, with the unit
/// resources only they used, transitively. Reused entities stay.
fn orphans(
    model: &Model,
    assignment: EntityId,
    replaced: &HashMap<EntityId, EntityId>,
    reused: &HashSet<EntityId>,
) -> Vec<EntityId> {
    let index = ReverseIndex::build(model);
    let mut removed: HashSet<EntityId> = HashSet::new();
    let mut queue: Vec<EntityId> = replaced
        .keys()
        .filter(|old| !replaced.values().any(|new| new == *old))
        .copied()
        .collect();
    let unit_resource = |entity: &Entity| {
        [
            "IFCSIUNIT",
            "IFCCONVERSIONBASEDUNIT",
            "IFCMEASUREWITHUNIT",
            "IFCDIMENSIONALEXPONENTS",
        ]
        .iter()
        .any(|ty| entity.type_name.eq_ignore_ascii_case(ty))
    };
    while let Some(id) = queue.pop() {
        if removed.contains(&id) || reused.contains(&id) {
            continue;
        }
        let Some(entity) = model.get(id) else {
            continue;
        };
        if !unit_resource(entity) {
            continue;
        }
        // The assignment's reference to a replaced unit is being swapped.
        let swapped = |from: EntityId| from == assignment && replaced.contains_key(&id);
        let used = index
            .referrers(id)
            .iter()
            .any(|referrer| !(removed.contains(&referrer.from) || swapped(referrer.from)));
        if used {
            continue;
        }
        removed.insert(id);
        let mut children = Vec::new();
        collect_refs(&entity.attributes, &mut children);
        queue.extend(children);
    }
    let mut removed: Vec<EntityId> = removed.into_iter().collect();
    removed.sort();
    removed
}

fn collect_refs(values: &[Value], out: &mut Vec<EntityId>) {
    for value in values {
        match value {
            Value::Ref(id) => out.push(*id),
            Value::List(items) => collect_refs(items, out),
            Value::Typed { value, .. } => collect_refs(std::slice::from_ref(value), out),
            _ => {}
        }
    }
}
