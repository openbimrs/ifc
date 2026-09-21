//! `IfcTimeSeries` subtypes and their value records.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::{TabularError, TabularResult};

const REGULAR: &str = "IFCREGULARTIMESERIES";
const IRREGULAR: &str = "IFCIRREGULARTIMESERIES";
const VALUE: &str = "IFCTIMESERIESVALUE";
const IRREGULAR_VALUE: &str = "IFCIRREGULARTIMESERIESVALUE";

/// Stage an `IfcTimeSeriesValue`.
///
/// # Errors
///
/// Refuses an empty value list: `ListValues` is `LIST [1:?]`.
pub fn add_time_series_value(tx: &mut Transaction, values: Vec<Value>) -> TabularResult<EntityId> {
    if values.is_empty() {
        return Err(TabularError::EmptyList {
            entity: VALUE,
            attribute: "ListValues",
        });
    }
    Ok(tx.create(Entity::new(VALUE, vec![Value::List(values)])))
}

/// Stage an `IfcIrregularTimeSeriesValue`.
///
/// Carries its own `TimeStamp`: an irregular series has no step to derive
/// the instant from, so each value states when it happened.
///
/// # Errors
///
/// Refuses a blank timestamp and an empty value list.
pub fn add_irregular_value(
    tx: &mut Transaction,
    timestamp: &str,
    values: Vec<Value>,
) -> TabularResult<EntityId> {
    if timestamp.trim().is_empty() {
        return Err(TabularError::BlankRequired {
            entity: IRREGULAR_VALUE,
            attribute: "TimeStamp",
        });
    }
    if values.is_empty() {
        return Err(TabularError::EmptyList {
            entity: IRREGULAR_VALUE,
            attribute: "ListValues",
        });
    }
    let attributes = vec![Value::Text(timestamp.into()), Value::List(values)];
    Ok(tx.create(Entity::new(IRREGULAR_VALUE, attributes)))
}

/// The `IfcTimeSeries` supertype attributes, shared by both subtypes.
#[derive(Debug, Clone, Copy, Default)]
pub struct SeriesDraft<'a> {
    /// `Name`. Required.
    pub name: &'a str,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `StartTime`. Required.
    pub start_time: &'a str,
    /// `EndTime`. Required.
    pub end_time: &'a str,
    /// `TimeSeriesDataType`. Required enumeration token.
    pub data_type: &'a str,
    /// `DataOrigin`. Required enumeration token.
    pub data_origin: &'a str,
    /// `UserDefinedDataOrigin`.
    pub user_defined_data_origin: Option<&'a str>,
    /// `Unit`, an `IfcUnit` reference.
    pub unit: Option<EntityId>,
}

fn supertype_slots(entity: &'static str, draft: SeriesDraft<'_>) -> TabularResult<Vec<Value>> {
    for (attribute, text) in [
        ("Name", draft.name),
        ("StartTime", draft.start_time),
        ("EndTime", draft.end_time),
        ("TimeSeriesDataType", draft.data_type),
        ("DataOrigin", draft.data_origin),
    ] {
        if text.trim().is_empty() {
            return Err(TabularError::BlankRequired { entity, attribute });
        }
    }
    Ok(vec![
        Value::Text(draft.name.into()),
        draft
            .description
            .map_or(Value::Null, |text| Value::Text(text.into())),
        Value::Text(draft.start_time.into()),
        Value::Text(draft.end_time.into()),
        Value::Enum(draft.data_type.into()),
        Value::Enum(draft.data_origin.into()),
        draft
            .user_defined_data_origin
            .map_or(Value::Null, |text| Value::Text(text.into())),
        draft.unit.map_or(Value::Null, Value::Ref),
    ])
}

/// Stage an `IfcRegularTimeSeries`.
///
/// A regular series states one `TimeStep` and derives every instant from
/// it, so its values carry no timestamps of their own.
///
/// # Errors
///
/// Refuses blank required text, a non-finite or non-positive step, and an
/// empty value list.
pub fn add_regular_time_series(
    tx: &mut Transaction,
    draft: SeriesDraft<'_>,
    time_step: f64,
    values: &[EntityId],
) -> TabularResult<EntityId> {
    let mut attributes = supertype_slots(REGULAR, draft)?;
    if !time_step.is_finite() || time_step <= 0.0 {
        return Err(TabularError::NotFinite {
            entity: REGULAR,
            attribute: "TimeStep",
        });
    }
    if values.is_empty() {
        return Err(TabularError::EmptyList {
            entity: REGULAR,
            attribute: "Values",
        });
    }
    attributes.push(Value::Real(time_step));
    attributes.push(Value::List(
        values.iter().copied().map(Value::Ref).collect(),
    ));
    Ok(tx.create(Entity::new(REGULAR, attributes)))
}

/// Stage an `IfcIrregularTimeSeries`.
///
/// # Errors
///
/// Refuses blank required text and an empty value list.
pub fn add_irregular_time_series(
    tx: &mut Transaction,
    draft: SeriesDraft<'_>,
    values: &[EntityId],
) -> TabularResult<EntityId> {
    let mut attributes = supertype_slots(IRREGULAR, draft)?;
    if values.is_empty() {
        return Err(TabularError::EmptyList {
            entity: IRREGULAR,
            attribute: "Values",
        });
    }
    attributes.push(Value::List(
        values.iter().copied().map(Value::Ref).collect(),
    ));
    Ok(tx.create(Entity::new(IRREGULAR, attributes)))
}
