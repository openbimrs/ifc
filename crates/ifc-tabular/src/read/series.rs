//! Reading `IfcRegularTimeSeries` and `IfcIrregularTimeSeries`.
//!
//! Neither subtype nor `IfcTimeSeries` declares a WHERE rule in IFC4 ADD2
//! TC1 or IFC4X3 ADD2, so the defects a read can find are structural:
//! missing required attributes, malformed slots, empty `LIST [1:?]`s, and
//! `Values` members that dangle or name the other subtype's value record.
//! A regular series lists `IfcTimeSeriesValue`, which has no timestamp;
//! an irregular one lists `IfcIrregularTimeSeriesValue`, which does.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::decode::{Need, Slots};
use super::issue::TabularIssue;

pub(crate) const REGULAR: &str = "IFCREGULARTIMESERIES";
pub(crate) const IRREGULAR: &str = "IFCIRREGULARTIMESERIES";
const VALUE: &str = "IFCTIMESERIESVALUE";
const IRREGULAR_VALUE: &str = "IFCIRREGULARTIMESERIESVALUE";

/// Which `IfcTimeSeries` subtype was read.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TimeSeriesKind {
    /// `IfcRegularTimeSeries`: instants follow from `TimeStep`.
    Regular {
        /// `TimeStep` in seconds; `None` when missing or malformed.
        time_step: Option<f64>,
    },
    /// `IfcIrregularTimeSeries`: each value carries its own timestamp.
    Irregular,
}

/// A borrowed time series, its resolved value records, and every defect
/// found on the way.
#[derive(Debug, Clone)]
pub struct TimeSeries<'m> {
    id: EntityId,
    kind: TimeSeriesKind,
    name: Option<&'m str>,
    description: Option<&'m str>,
    start_time: Option<&'m str>,
    end_time: Option<&'m str>,
    data_type: Option<&'m str>,
    data_origin: Option<&'m str>,
    user_defined_data_origin: Option<&'m str>,
    unit: Option<EntityId>,
    values: Vec<SeriesValue<'m>>,
    issues: Vec<TabularIssue>,
}

impl<'m> TimeSeries<'m> {
    /// The series' id.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// Regular or irregular, with the regular step.
    #[must_use]
    pub const fn kind(&self) -> TimeSeriesKind {
        self.kind
    }

    /// `Name`.
    #[must_use]
    pub const fn name(&self) -> Option<&'m str> {
        self.name
    }

    /// `Description`.
    #[must_use]
    pub const fn description(&self) -> Option<&'m str> {
        self.description
    }

    /// `StartTime`, an `IfcDateTime` as written.
    #[must_use]
    pub const fn start_time(&self) -> Option<&'m str> {
        self.start_time
    }

    /// `EndTime`, an `IfcDateTime` as written.
    #[must_use]
    pub const fn end_time(&self) -> Option<&'m str> {
        self.end_time
    }

    /// `TimeSeriesDataType` token.
    #[must_use]
    pub const fn data_type(&self) -> Option<&'m str> {
        self.data_type
    }

    /// `DataOrigin` token.
    #[must_use]
    pub const fn data_origin(&self) -> Option<&'m str> {
        self.data_origin
    }

    /// `UserDefinedDataOrigin`.
    #[must_use]
    pub const fn user_defined_data_origin(&self) -> Option<&'m str> {
        self.user_defined_data_origin
    }

    /// `Unit`, an `IfcUnit` reference.
    #[must_use]
    pub const fn unit(&self) -> Option<EntityId> {
        self.unit
    }

    /// Value records in `Values` order that resolved to the subtype's
    /// own value entity.
    #[must_use]
    pub fn values(&self) -> &[SeriesValue<'m>] {
        &self.values
    }

    /// Every defect found.
    #[must_use]
    pub fn issues(&self) -> &[TabularIssue] {
        &self.issues
    }
}

/// A borrowed `IfcTimeSeriesValue` or `IfcIrregularTimeSeriesValue`.
#[derive(Debug, Clone, Copy)]
pub struct SeriesValue<'m> {
    id: EntityId,
    timestamp: Option<&'m str>,
    values: Option<&'m [Value]>,
}

impl<'m> SeriesValue<'m> {
    /// The value record's id.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// `TimeStamp`; always `None` in a regular series, whose value
    /// records declare none.
    #[must_use]
    pub const fn timestamp(&self) -> Option<&'m str> {
        self.timestamp
    }

    /// `ListValues` as written; `None` when missing or malformed.
    #[must_use]
    pub const fn values(&self) -> Option<&'m [Value]> {
        self.values
    }
}

/// Read a time series the caller has already type-checked as `type_name`.
pub(crate) fn read_series<'m>(
    model: &'m Model,
    schema: &'m Schema,
    id: EntityId,
    entity: &'m Entity,
    type_name: &'static str,
) -> TimeSeries<'m> {
    let regular = type_name == REGULAR;
    let mut issues = Vec::new();
    let mut slots = Slots::open(model, schema, id, entity, type_name, &mut issues);
    let name = slots.text("Name", Need::Required);
    let description = slots.text("Description", Need::Optional);
    let start_time = slots.text("StartTime", Need::Required);
    let end_time = slots.text("EndTime", Need::Required);
    let data_type = slots.enumeration("TimeSeriesDataType", Need::Required);
    let data_origin = slots.enumeration("DataOrigin", Need::Required);
    let user_defined_data_origin = slots.text("UserDefinedDataOrigin", Need::Optional);
    let unit = slots.reference("Unit", Need::Optional);
    let kind = if regular {
        TimeSeriesKind::Regular {
            time_step: slots.number("TimeStep", Need::Required),
        }
    } else {
        TimeSeriesKind::Irregular
    };
    let value_type = if regular { VALUE } else { IRREGULAR_VALUE };
    let members = slots.records("Values", Need::Required, value_type);

    let values = members
        .into_iter()
        .flatten()
        .map(|(value, record)| {
            let mut slots = Slots::open(model, schema, value, record, value_type, &mut issues);
            SeriesValue {
                id: value,
                timestamp: if regular {
                    None
                } else {
                    slots.text("TimeStamp", Need::Required)
                },
                values: slots.values("ListValues", Need::Required),
            }
        })
        .collect();

    TimeSeries {
        id,
        kind,
        name,
        description,
        start_time,
        end_time,
        data_type,
        data_origin,
        user_defined_data_origin,
        unit,
        values,
        issues,
    }
}
