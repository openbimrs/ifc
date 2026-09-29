//! Cost values, monetary units and currency relationships, laid out by name
//! in the model's declared release (#213).
//!
//! The three records differ between releases, from the EXPRESS sources:
//!
//! ```text
//! IfcCostValue
//!   IFC2X3_TC1 (8)   Name, Description, AppliedValue, UnitBasis,
//!                    ApplicableDate (IfcDateTimeSelect),
//!                    FixedUntilDate (IfcDateTimeSelect), CostType (required),
//!                    Condition
//!   IFC4, IFC4X3 (10) Name, Description, AppliedValue, UnitBasis,
//!                    ApplicableDate (IfcDate), FixedUntilDate (IfcDate),
//!                    Category, Condition, ArithmeticOperator, Components
//! IfcMonetaryUnit
//!   IFC2X3_TC1       Currency : IfcCurrencyEnum
//!   IFC4, IFC4X3     Currency : IfcLabel
//! IfcCurrencyRelationship
//!   IFC2X3_TC1 (5)   RelatingMonetaryUnit, RelatedMonetaryUnit, ExchangeRate,
//!                    RateDateTime (IfcDateAndTime, required), RateSource
//!   IFC4, IFC4X3 (7) Name, Description, RelatingMonetaryUnit,
//!                    RelatedMonetaryUnit, ExchangeRate,
//!                    RateDateTime (IfcDateTime, optional), RateSource
//! ```
//!
//! IFC4 renamed `CostType` to `Category` (see `release.rs`). IFC2X3 composes
//! cost values through `IfcAppliedValueRelationship`, which this crate does
//! not author, so a [`CostValueKind::Components`] draft is refused there.
//! IFC2X3 dates are record forms of [`DateTimeValue`], staged by the writer.

use ifc_model::{EntityId, Model, Transaction, Value};

use crate::ArithmeticOperator;

use super::datetime::{patch, DateTimeValue, PLACEHOLDERS};
use super::draft::{CostValueDraft, CostValueKind};
use super::release::{bind, Release};
use super::validate::{invalid, reference_type};
use super::{CostAuthoringError, CostAuthoringResult};

/// Validate and stage one scalar or explicitly composed cost value in the
/// model's declared release.
///
/// # Errors
///
/// A non-finite amount, an empty or non-`IfcCostValue` component list; a
/// header binding no single verified release (`MultipleSchemas`,
/// `UnsupportedSchema`); a date record form the schema's rules refuse
/// (`InvalidValue`); a value in the form the release does not declare
/// (`AuthoringValueType`: IFC2X3 dates are `IfcDateTimeSelect` records,
/// IFC4 and IFC4X3 dates `IfcDate` text); a composed value in IFC2X3
/// (`AuthoringNotInSchema`: it declares no `ArithmeticOperator` or
/// `Components`); and an IFC2X3 value without a `category`
/// (`AuthoringRequired`: `CostType` is required there). Nothing is staged
/// on an error: date records are staged only once the value is accepted.
pub fn create_cost_value(
    tx: &mut Transaction,
    model: &Model,
    draft: CostValueDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCCOSTVALUE";
    let release = bind(model)?;
    let (applied_value, operator, components) = match draft.kind {
        CostValueKind::Monetary(amount) => {
            if !amount.is_finite() {
                return Err(invalid(
                    ENTITY,
                    "AppliedValue",
                    "expected a finite monetary amount",
                ));
            }
            (
                Value::Typed {
                    type_name: "IFCMONETARYMEASURE".into(),
                    value: Box::new(Value::Real(amount)),
                },
                Value::Null,
                Value::Null,
            )
        }
        CostValueKind::Components {
            operator,
            components,
        } => {
            if components.is_empty() {
                return Err(invalid(
                    ENTITY,
                    "Components",
                    "expected at least one component",
                ));
            }
            for target in components {
                reference_type(tx, model, ENTITY, "Components", *target, ENTITY)?;
            }
            (
                Value::Null,
                Value::Enum(operator_token(operator).into()),
                refs(components),
            )
        }
    };
    let dates = [draft.applicable_date, draft.fixed_until_date];
    for (date, attribute) in dates.iter().zip(["ApplicableDate", "FixedUntilDate"]) {
        if let Some(date) = date {
            check_date(release, ENTITY, attribute, *date)?;
        }
    }
    let provisional = |slot: usize| {
        dates[slot].map_or(Value::Null, |date: DateTimeValue<'_>| {
            date.provisional(PLACEHOLDERS[slot])
        })
    };
    let mut record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("AppliedValue", applied_value),
            ("ApplicableDate", provisional(0)),
            ("FixedUntilDate", provisional(1)),
            ("Category", optional_text(draft.category)),
            ("Condition", optional_text(draft.condition)),
            ("ArithmeticOperator", operator),
            ("Components", components),
        ],
    )?;
    patch(tx, &mut record, &dates);
    Ok(tx.create(record))
}

/// Check a date's record form against the schema's rules, and that the
/// release declares the form it takes: text for `IfcDate`/`IfcDateTime`,
/// the record entity for an IFC2X3 `IfcDateTimeSelect` or `IfcDateAndTime`.
fn check_date(
    release: Release,
    entity: &'static str,
    attribute: &'static str,
    date: DateTimeValue<'_>,
) -> CostAuthoringResult<()> {
    date.check()?;
    let fits = match date.record_type() {
        // Text is checked against the declared type by `record`.
        None => true,
        Some(record) => release.accepts_record(entity, attribute, record)?,
    };
    if fits {
        return Ok(());
    }
    let declared = release
        .schema()
        .attributes(entity)
        .into_iter()
        .find(|declared| declared.name.eq_ignore_ascii_case(attribute))
        .map_or("", |declared| declared.type_name.as_str());
    Err(CostAuthoringError::AuthoringValueType {
        entity,
        attribute,
        declared,
        schema: release.version(),
    })
}

fn operator_token(operator: ArithmeticOperator) -> &'static str {
    match operator {
        ArithmeticOperator::Add => "ADD",
        ArithmeticOperator::Divide => "DIVIDE",
        ArithmeticOperator::Multiply => "MULTIPLY",
        ArithmeticOperator::Subtract => "SUBTRACT",
    }
}

pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |value| Value::Text(value.into()))
}
pub(crate) fn optional_enum(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |value| Value::Enum(value.into()))
}
pub(crate) fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// Stage an `IfcMonetaryUnit` in the model's declared release.
///
/// IFC4 and IFC4X3 declare `Currency` an `IfcLabel`, written as given. The
/// reader resolves a file's currency by matching this text, which is why a
/// blank one is refused rather than written: it would leave every cost
/// value denominated in nothing. IFC2X3 declares an `IfcCurrencyEnum`: the
/// label must name one of its enumerators and is written as that
/// enumerator, never as text.
///
/// # Errors
///
/// A blank currency; a header binding no single verified release
/// (`MultipleSchemas`, `UnsupportedSchema`); and in IFC2X3 a currency
/// `IfcCurrencyEnum` does not list (`AuthoringValueType`).
pub fn create_monetary_unit(
    tx: &mut Transaction,
    model: &Model,
    currency: &str,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCMONETARYUNIT";
    let release = bind(model)?;
    if currency.trim().is_empty() {
        return Err(invalid(ENTITY, "Currency", "expected a currency label"));
    }
    let (_, declaration) = release.declared(ENTITY, "Currency")?;
    let value = match release
        .schema()
        .type_def(&declaration.type_name)
        .map(|t| &t.kind)
    {
        Some(ifc_schema::TypeKind::Enumeration(_)) => {
            let token = release.enumerator(ENTITY, "Currency", currency)?.ok_or(
                CostAuthoringError::AuthoringValueType {
                    entity: ENTITY,
                    attribute: "Currency",
                    declared: declaration.type_name.as_str(),
                    schema: release.version(),
                },
            )?;
            Value::Enum(token.into())
        }
        _ => Value::Text(currency.into()),
    };
    let record = release.record(ENTITY, vec![("Currency", value)])?;
    Ok(tx.create(record))
}

/// Stage an `IfcCurrencyRelationship`: an exchange rate between two
/// monetary units, in the model's declared release.
///
/// The rate is an `IfcPositiveRatioMeasure`, so zero and negative rates are
/// refused: a rate of zero would value every converted cost at nothing,
/// which is a conversion nobody meant to state. `rate_date_time` is
/// `IfcDateTime` text in IFC4 and IFC4X3, where it is optional; IFC2X3
/// requires it as an `IfcDateAndTime` record
/// ([`DateTimeValue::DateAndTime`]), which the writer stages.
///
/// # Errors
///
/// A reference that is not an `IfcMonetaryUnit`, a unit related to itself,
/// a non-positive or non-finite rate; a header binding no single verified
/// release; a date the schema's rules refuse (`InvalidValue`) or in a form
/// the release does not declare (`AuthoringValueType`); and in IFC2X3 a
/// missing `rate_date_time` (`AuthoringRequired`). Nothing is staged on an
/// error.
pub fn create_currency_relationship(
    tx: &mut Transaction,
    model: &Model,
    relating: EntityId,
    related: EntityId,
    exchange_rate: f64,
    rate_date_time: Option<DateTimeValue<'_>>,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCCURRENCYRELATIONSHIP";
    let release = bind(model)?;
    for (target, attribute) in [
        (relating, "RelatingMonetaryUnit"),
        (related, "RelatedMonetaryUnit"),
    ] {
        reference_type(tx, model, ENTITY, attribute, target, "IFCMONETARYUNIT")?;
    }
    if relating == related {
        return Err(invalid(
            ENTITY,
            "RelatedMonetaryUnit",
            "expected a unit other than the relating one",
        ));
    }
    if !exchange_rate.is_finite() || exchange_rate <= 0.0 {
        return Err(invalid(
            ENTITY,
            "ExchangeRate",
            "expected a positive finite ratio",
        ));
    }
    if let Some(date) = rate_date_time {
        check_date(release, ENTITY, "RateDateTime", date)?;
    }
    let dates = [rate_date_time];
    let mut record = release.record(
        ENTITY,
        vec![
            ("RelatingMonetaryUnit", Value::Ref(relating)),
            ("RelatedMonetaryUnit", Value::Ref(related)),
            ("ExchangeRate", Value::Real(exchange_rate)),
            (
                "RateDateTime",
                rate_date_time.map_or(Value::Null, |date| date.provisional(PLACEHOLDERS[0])),
            ),
        ],
    )?;
    patch(tx, &mut record, &dates);
    Ok(tx.create(record))
}
