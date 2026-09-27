//! Borrowed read views over tables and time series.
//!
//! A read borrows the model and returns what the file says, including
//! what it says wrongly: every malformed slot, dangling or mistyped
//! reference, and WR1/WR2 violation is reported as a [`TabularIssue`]
//! beside whatever did decode, never silently dropped.
//!
//! # Declared schema
//!
//! Slots are located by attribute name in the schema the caller passes,
//! so arity and positions come from IFC4 ADD2 TC1 or IFC4X3 ADD2 exactly
//! as declared. IFC2x3 is refused rather than approximated: its
//! `IfcTimeSeries` times are `IfcDateTimeSelect` references, not the
//! `IfcDateTime` strings this view decodes, and its `IfcTable` has no
//! `Columns`.

mod decode;
mod issue;
mod series;
mod table;

use ifc_model::{EntityId, Model};
use ifc_schema::{Schema, SchemaVersion};

use crate::error::{TabularReadError, TabularReadResult};

pub use issue::TabularIssue;
pub use series::{SeriesValue, TimeSeries, TimeSeriesKind};
pub use table::{Table, TableColumn, TableRow};

/// Borrowed entry point for reading tables and time series.
#[derive(Debug, Clone, Copy)]
pub struct TabularView<'m> {
    model: &'m Model,
    schema: &'m Schema,
}

impl<'m> TabularView<'m> {
    /// Read `model` under the declared `schema`.
    ///
    /// # Errors
    ///
    /// Refuses a schema other than IFC4 or IFC4X3.
    pub fn new(model: &'m Model, schema: &'m Schema) -> TabularReadResult<Self> {
        match schema.version() {
            Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3) => Ok(Self { model, schema }),
            _ => Err(TabularReadError::UnsupportedSchema {
                schema: schema.name().to_owned(),
            }),
        }
    }

    /// Ids of every `IfcTable` in the model.
    #[must_use]
    pub fn table_ids(self) -> &'m [EntityId] {
        self.model.ids_of_type(table::TABLE)
    }

    /// Ids of every regular, then every irregular, time series.
    #[must_use]
    pub fn series_ids(self) -> Vec<EntityId> {
        let mut ids = self.model.ids_of_type(series::REGULAR).to_vec();
        ids.extend_from_slice(self.model.ids_of_type(series::IRREGULAR));
        ids
    }

    /// Read the `IfcTable` at `id`.
    ///
    /// # Errors
    ///
    /// Refuses an absent id and a record of another type. Defects inside
    /// the table are not errors; they are in [`Table::issues`].
    pub fn table(self, id: EntityId) -> TabularReadResult<Table<'m>> {
        let entity = self
            .model
            .get(id)
            .ok_or(TabularReadError::UnknownEntity { id })?;
        if !entity.is_type(table::TABLE) {
            return Err(TabularReadError::WrongEntityType {
                id,
                expected: "IFCTABLE",
                actual: entity.type_name.to_string(),
            });
        }
        Ok(table::read_table(self.model, self.schema, id, entity))
    }

    /// Read the regular or irregular time series at `id`.
    ///
    /// # Errors
    ///
    /// Refuses an absent id and a record that is neither subtype. Defects
    /// inside the series are in [`TimeSeries::issues`].
    pub fn time_series(self, id: EntityId) -> TabularReadResult<TimeSeries<'m>> {
        let entity = self
            .model
            .get(id)
            .ok_or(TabularReadError::UnknownEntity { id })?;
        let type_name = if entity.is_type(series::REGULAR) {
            series::REGULAR
        } else if entity.is_type(series::IRREGULAR) {
            series::IRREGULAR
        } else {
            return Err(TabularReadError::WrongEntityType {
                id,
                expected: "IFCREGULARTIMESERIES or IFCIRREGULARTIMESERIES",
                actual: entity.type_name.to_string(),
            });
        };
        Ok(series::read_series(
            self.model,
            self.schema,
            id,
            entity,
            type_name,
        ))
    }
}
