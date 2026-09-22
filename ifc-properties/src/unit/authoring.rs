//! Transactional authoring of the project unit context.
//!
//! Reading lives in [`super::assignment`]; this module is the write side. The
//! slot layout is documented there and deliberately not repeated: both sides
//! resolve the same constants so a schema correction cannot update one and
//! leave the other stale.
//!
//! These helpers only stage records. [`ifc_model::Transaction::commit`] owns
//! atomic application, so a rejected draft cannot leave a half-built unit in
//! the model.
//!
//! # Why prefixes are validated, not defaulted
//!
//! An unrecognised `IfcSIPrefix` is refused rather than written through as
//! "no prefix". Silently dropping `MILLI` would rescale every length in the
//! file by a thousand, and the resulting model would still parse -- the worst
//! kind of authoring bug, because nothing downstream can detect it.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::assignment::prefix_exponent;
use crate::{PropertyError, PropertyResult};

/// Authored fields for `IfcSIUnit`.
#[derive(Debug, Clone, Copy)]
pub struct SiUnitDraft<'a> {
    /// `IfcNamedUnit.UnitType`, an `IfcUnitEnum` constant such as `LENGTHUNIT`.
    pub unit_type: &'a str,
    /// `IfcSIUnit.Name`, an `IfcSIUnitName` constant such as `METRE`.
    pub name: &'a str,
    /// `IfcSIUnit.Prefix`, an `IfcSIPrefix` constant such as `MILLI`.
    ///
    /// `None` writes an unprefixed unit. A prefix that is not a schema
    /// constant is refused; see the module note.
    pub prefix: Option<&'a str>,
}

/// Authored fields for `IfcMonetaryUnit`.
#[derive(Debug, Clone, Copy)]
pub struct MonetaryUnitDraft<'a> {
    /// `IfcMonetaryUnit.Currency`, an ISO 4217 code such as `EUR`.
    pub currency: &'a str,
}

/// Stage an `IfcSIUnit`.
///
/// `Dimensions` is written as `Value::Derived`, the STEP asterisk. The
/// schema declares it DERIVE on this subtype, computed from `Name`, and
/// a derived attribute is not an omitted one: `$` claims the value is
/// absent, while `*` states it is computed by the schema rule.
pub fn add_si_unit(tx: &mut Transaction, draft: SiUnitDraft<'_>) -> PropertyResult<EntityId> {
    require_enum("IFCSIUNIT", "UnitType", draft.unit_type)?;
    require_enum("IFCSIUNIT", "Name", draft.name)?;
    if let Some(prefix) = draft.prefix {
        if prefix_exponent(prefix).is_none() {
            return Err(authoring_invalid("IFCSIUNIT", "Prefix", prefix));
        }
    }
    Ok(tx.create(Entity::new(
        "IFCSIUNIT",
        vec![
            Value::Derived,
            enum_value(draft.unit_type),
            draft.prefix.map_or(Value::Null, enum_value),
            enum_value(draft.name),
        ],
    )))
}

/// Stage an `IfcMonetaryUnit`.
pub fn add_monetary_unit(
    tx: &mut Transaction,
    draft: MonetaryUnitDraft<'_>,
) -> PropertyResult<EntityId> {
    if draft.currency.trim().is_empty() {
        return Err(authoring_invalid(
            "IFCMONETARYUNIT",
            "Currency",
            "expected a currency code",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCMONETARYUNIT",
        vec![Value::Text(draft.currency.into())],
    )))
}

/// Authored fields for `IfcConversionBasedUnit`.
#[derive(Debug, Clone, Copy)]
pub struct ConversionBasedUnitDraft<'a> {
    /// `IfcConversionBasedUnit.UnitType`, an `IfcUnitEnum` constant.
    pub unit_type: &'a str,
    /// `IfcConversionBasedUnit.Name`.
    pub name: &'a str,
    /// `IfcConversionBasedUnit.ConversionFactor`, an `IfcMeasureWithUnit`.
    pub conversion_factor: EntityId,
    /// `IfcConversionBasedUnit.Dimensions`, an `IfcDimensionalExponents`.
    pub dimensions: EntityId,
}

/// Stage an `IfcConversionBasedUnit`.
///
/// The conversion factor and dimensions are references the caller must have
/// created; their types are checked by `ifc-validate`, not here, because this
/// crate has no schema handle.
pub fn add_conversion_based_unit(
    tx: &mut Transaction,
    draft: ConversionBasedUnitDraft<'_>,
) -> PropertyResult<EntityId> {
    require_enum("IFCCONVERSIONBASEDUNIT", "UnitType", draft.unit_type)?;
    if draft.name.trim().is_empty() {
        return Err(authoring_invalid(
            "IFCCONVERSIONBASEDUNIT",
            "Name",
            "expected a non-empty name",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCCONVERSIONBASEDUNIT",
        vec![
            Value::Ref(draft.dimensions),
            enum_value(draft.unit_type),
            Value::Text(draft.name.into()),
            Value::Ref(draft.conversion_factor),
        ],
    )))
}

/// Stage an `IfcDimensionalExponents`.
///
/// All seven SI base exponents are required by the schema, so they are taken
/// positionally rather than as options: a missing exponent is not "unset", it
/// is zero, and conflating the two silently changes the dimension.
pub fn add_dimensional_exponents(tx: &mut Transaction, exponents: [i64; 7]) -> EntityId {
    tx.create(Entity::new(
        "IFCDIMENSIONALEXPONENTS",
        exponents.iter().copied().map(Value::Integer).collect(),
    ))
}

/// Stage an `IfcDerivedUnitElement`: one base unit raised to an exponent.
pub fn add_derived_unit_element(
    tx: &mut Transaction,
    unit: EntityId,
    exponent: i64,
) -> PropertyResult<EntityId> {
    if exponent == 0 {
        return Err(authoring_invalid(
            "IFCDERIVEDUNITELEMENT",
            "Exponent",
            "expected a non-zero exponent",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCDERIVEDUNITELEMENT",
        vec![Value::Ref(unit), Value::Integer(exponent)],
    )))
}

/// Stage an `IfcDerivedUnit` over previously staged elements.
pub fn add_derived_unit(
    tx: &mut Transaction,
    elements: &[EntityId],
    unit_type: &str,
    user_defined_type: Option<&str>,
) -> PropertyResult<EntityId> {
    if elements.is_empty() {
        return Err(authoring_invalid(
            "IFCDERIVEDUNIT",
            "Elements",
            "expected at least one derived unit element",
        ));
    }
    require_enum("IFCDERIVEDUNIT", "UnitType", unit_type)?;
    Ok(tx.create(Entity::new(
        "IFCDERIVEDUNIT",
        vec![
            Value::List(elements.iter().copied().map(Value::Ref).collect()),
            enum_value(unit_type),
            user_defined_type.map_or(Value::Null, |v| Value::Text(v.into())),
        ],
    )))
}

/// Stage an `IfcUnitAssignment`: the project-wide unit context.
///
/// Assigning no units is refused. An empty assignment parses, but leaves every
/// measure in the file dimensionless by omission, which is never intended.
pub fn assign_units(tx: &mut Transaction, units: &[EntityId]) -> PropertyResult<EntityId> {
    if units.is_empty() {
        return Err(authoring_invalid(
            "IFCUNITASSIGNMENT",
            "Units",
            "expected at least one unit",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCUNITASSIGNMENT",
        vec![Value::List(units.iter().copied().map(Value::Ref).collect())],
    )))
}

/// Stage an `IfcMeasureWithUnit`.
///
/// A magnitude paired with the unit it is stated in. Both attributes
/// are required: this is the entity `IfcConversionBasedUnit` points at
/// for its conversion factor, and a factor missing either half states
/// no conversion at all.
///
/// # Errors
///
/// Refuses a null value component; the schema types it as a required
/// `IfcValue`.
pub fn add_measure_with_unit(
    tx: &mut Transaction,
    value: Value,
    unit: EntityId,
) -> PropertyResult<EntityId> {
    if matches!(value, Value::Null) {
        return Err(authoring_invalid(
            "IFCMEASUREWITHUNIT",
            "ValueComponent",
            "expected a value",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCMEASUREWITHUNIT",
        vec![value, Value::Ref(unit)],
    )))
}

/// Stage an `IfcContextDependentUnit`.
///
/// A unit with no SI conversion, named by the project that defines it.
/// Unlike `IfcSIUnit`, `Dimensions` is a real attribute here rather
/// than a derived one, so the caller supplies it.
///
/// # Errors
///
/// Refuses a blank name or a malformed `UnitType` token.
pub fn add_context_dependent_unit(
    tx: &mut Transaction,
    dimensions: EntityId,
    unit_type: &str,
    name: &str,
) -> PropertyResult<EntityId> {
    require_enum("IFCCONTEXTDEPENDENTUNIT", "UnitType", unit_type)?;
    if name.trim().is_empty() {
        return Err(authoring_invalid(
            "IFCCONTEXTDEPENDENTUNIT",
            "Name",
            "expected a non-empty name",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCCONTEXTDEPENDENTUNIT",
        vec![
            Value::Ref(dimensions),
            enum_value(unit_type),
            Value::Text(name.into()),
        ],
    )))
}

fn require_enum(entity: &'static str, attribute: &'static str, value: &str) -> PropertyResult<()> {
    if value.trim().is_empty() || !value.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
        return Err(authoring_invalid(entity, attribute, value));
    }
    Ok(())
}

fn enum_value(value: &str) -> Value {
    Value::Enum(value.into())
}

fn authoring_invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> PropertyError {
    PropertyError::AuthoringInvalid {
        entity,
        attribute,
        value: value.into(),
    }
}

/// Stage an `IfcConversionBasedUnitWithOffset`.
///
/// The offset form exists for scales whose zero is not the SI zero:
/// degrees Celsius convert to kelvin by a factor of one and an offset
/// of 273.15. Writing that as a plain conversion unit would place
/// absolute zero at the freezing point of water, so the offset is a
/// separate entity rather than an optional slot on the base form.
///
/// # Errors
///
/// Refuses a `UnitType` outside `IfcUnitEnum`, a blank name, and a
/// non-finite offset.
pub fn add_conversion_based_unit_with_offset(
    tx: &mut Transaction,
    draft: ConversionBasedUnitDraft<'_>,
    conversion_offset: f64,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCCONVERSIONBASEDUNITWITHOFFSET";
    require_enum(ENTITY, "UnitType", draft.unit_type)?;
    if draft.name.trim().is_empty() {
        return Err(authoring_invalid(
            ENTITY,
            "Name",
            "expected a non-empty name",
        ));
    }
    if !conversion_offset.is_finite() {
        return Err(authoring_invalid(
            ENTITY,
            "ConversionOffset",
            "expected a finite offset",
        ));
    }
    Ok(tx.create(Entity::new(
        ENTITY,
        vec![
            Value::Ref(draft.dimensions),
            enum_value(draft.unit_type),
            Value::Text(draft.name.into()),
            Value::Ref(draft.conversion_factor),
            Value::Real(conversion_offset),
        ],
    )))
}
