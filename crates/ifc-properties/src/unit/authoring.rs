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

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::TypeKind;

use super::assignment::prefix_exponent;
use crate::quantity::release::bind;
use crate::{PropertyError, PropertyResult};

/// Authored fields for `IfcSIUnit`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> SiUnitDraft<'a> {
    /// Starts a draft from its required `unit_type`, `name`; every other field
    /// is unset.
    #[must_use]
    pub fn new(unit_type: &'a str, name: &'a str) -> Self {
        Self {
            unit_type,
            name,
            prefix: None,
        }
    }

    /// Sets `prefix`.
    ///
    /// `IfcSIUnit.Prefix`, an `IfcSIPrefix` constant such as `MILLI`.
    ///
    /// `None` writes an unprefixed unit. A prefix that is not a schema
    /// constant is refused; see the module note.
    #[must_use]
    pub fn prefix(mut self, value: &'a str) -> Self {
        self.prefix = Some(value);
        self
    }
}

/// Authored fields for `IfcMonetaryUnit`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MonetaryUnitDraft<'a> {
    /// `IfcMonetaryUnit.Currency`, an ISO 4217 code such as `EUR`.
    ///
    /// An `IfcLabel` from IFC4 on, an `IfcCurrencyEnum` enumerator in
    /// IFC2X3; [`create_monetary_unit`] writes the declared form.
    pub currency: &'a str,
}

impl<'a> MonetaryUnitDraft<'a> {
    /// Starts a draft from its required `currency`; every other field is unset.
    #[must_use]
    pub fn new(currency: &'a str) -> Self {
        Self { currency }
    }
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

/// Stage an `IfcMonetaryUnit` as IFC4 text, whatever the model's release.
///
/// Takes no model, so it cannot know the release, and always writes
/// `Currency` as an `IfcLabel`. That is the IFC4, IFC4X1, IFC4X2 and
/// IFC4X3 form; in IFC2X3 `Currency` is an `IfcCurrencyEnum`, so the
/// record this writes there is schema-invalid.
///
/// # Errors
///
/// [`PropertyError::AuthoringInvalid`] for a blank currency.
#[deprecated(
    note = "writes IFC4 text even in IFC2X3; use `create_monetary_unit`, which binds the model's release (#232)"
)]
pub fn add_monetary_unit(
    tx: &mut Transaction,
    draft: MonetaryUnitDraft<'_>,
) -> PropertyResult<EntityId> {
    require_currency(draft.currency)?;
    Ok(tx.create(Entity::new(
        MONETARY_UNIT,
        vec![Value::Text(draft.currency.into())],
    )))
}

/// Stage an `IfcMonetaryUnit` in `model`'s declared release.
///
/// `Currency` changed type between releases:
///
/// ```text
/// IFC2X3 TC1                         Currency : IfcCurrencyEnum
/// IFC4, IFC4X1, IFC4X2, IFC4X3 ADD2  Currency : IfcLabel
/// ```
///
/// In IFC2X3 the currency must name an `IfcCurrencyEnum` enumerator (matched
/// ignoring ASCII case) and is written as that token, `.EUR.`; elsewhere it
/// is written as the text given, `'EUR'`. The release binds as for
/// [`create_quantity`](crate::create_quantity): IFC2X3, IFC4 and IFC4X3,
/// with an empty header binding IFC4.
///
/// # Errors
///
/// Refused before anything is staged:
/// - [`PropertyError::AuthoringInvalid`]: a blank currency, or in IFC2X3 a
///   currency `IfcCurrencyEnum` does not list. It is never written as text
///   there, nor mapped to another enumerator.
/// - [`PropertyError::MultipleSchemas`] and
///   [`PropertyError::UnsupportedSchema`]: the header binds no single
///   release this writer is verified against.
pub fn create_monetary_unit(
    tx: &mut Transaction,
    model: &Model,
    draft: MonetaryUnitDraft<'_>,
) -> PropertyResult<EntityId> {
    let layout = bind(model)?;
    require_currency(draft.currency)?;
    let schema = layout.schema();
    let declared = schema
        .attributes(MONETARY_UNIT)
        .into_iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case("Currency"))
        .ok_or(PropertyError::AuthoringNotInSchema {
            entity: MONETARY_UNIT,
            attribute: "Currency",
            schema: layout.version(),
        })?;
    let currency = draft.currency.trim();
    let value = match schema.type_def(&declared.type_name).map(|t| &t.kind) {
        Some(TypeKind::Enumeration(members)) => {
            let token = members
                .iter()
                .find(|member| member.eq_ignore_ascii_case(currency))
                .ok_or_else(|| {
                    authoring_invalid(
                        MONETARY_UNIT,
                        "Currency",
                        format!(
                            "{currency:?} is not an {} member in {:?}",
                            declared.type_name,
                            layout.version()
                        ),
                    )
                })?;
            Value::Enum(token.as_str().into())
        }
        _ => Value::Text(draft.currency.into()),
    };
    let record = layout.named_record(MONETARY_UNIT, vec![("Currency", value)])?;
    Ok(tx.create(record))
}

const MONETARY_UNIT: &str = "IFCMONETARYUNIT";

fn require_currency(currency: &str) -> PropertyResult<()> {
    if currency.trim().is_empty() {
        return Err(authoring_invalid(
            MONETARY_UNIT,
            "Currency",
            "expected a currency code",
        ));
    }
    Ok(())
}

/// Authored fields for `IfcConversionBasedUnit`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> ConversionBasedUnitDraft<'a> {
    /// Starts a draft from its required `unit_type`, `name`,
    /// `conversion_factor`, `dimensions`; every other field is unset.
    #[must_use]
    pub fn new(
        unit_type: &'a str,
        name: &'a str,
        conversion_factor: EntityId,
        dimensions: EntityId,
    ) -> Self {
        Self {
            unit_type,
            name,
            conversion_factor,
            dimensions,
        }
    }
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
/// `IfcValue`. A bare literal is refused with
/// [`PropertyError::ValueForm`](crate::PropertyError::ValueForm): `IfcValue`
/// is a SELECT, so the value names its measure as a typed parameter.
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
    crate::pset::value_form::require_ifc_value("IFCMEASUREWITHUNIT", "ValueComponent", &value)?;
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
