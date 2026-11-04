//! The reader's diagnostics, sorted into what a georeferencing read must
//! refuse (#686, #690).

use ifc_model::{EntityId, Model, RecordDamage};

use crate::error::{GeorefError, GeorefResult};

/// The reader's diagnostics for one entry point, built once and shared by
/// every record it reads: units, CRS, operation and property sets.
pub(crate) struct Damage(RecordDamage);

impl Damage {
    pub(crate) fn of(model: &Model) -> GeorefResult<Self> {
        RecordDamage::of(model)
            .map(Self)
            .map_err(|_| GeorefError::RecordsIncomplete {
                diagnostics: model.diagnostics().len(),
            })
    }

    /// Refuse a record the reader reported by id.
    pub(crate) fn check(&self, id: EntityId) -> GeorefResult<()> {
        match self.0.reported(id) {
            Some(diagnostic) => Err(GeorefError::RecordDamaged {
                entity: id,
                diagnostic,
            }),
            None => Ok(()),
        }
    }

    /// Refuse a unit record the reader reported by id.
    pub(crate) fn check_unit(&self, id: EntityId) -> GeorefResult<()> {
        match self.0.reported(id) {
            Some(diagnostic) => Err(GeorefError::UnitRecordDamaged {
                entity: id,
                diagnostic,
            }),
            None => Ok(()),
        }
    }

    /// Whether a record was skipped, so an absent record may have existed.
    pub(crate) fn records_lost(&self) -> bool {
        self.0.records_lost()
    }

    /// How many diagnostics the model carries.
    pub(crate) fn diagnostics(&self) -> usize {
        self.0.diagnostics()
    }

    /// Pass a resolved value through unless a skipped record names an id a
    /// kept record holds: the skip may have swallowed records the reading
    /// would have used.
    pub(crate) fn settle<T>(&self, value: T) -> GeorefResult<T> {
        if self.0.held_id_skipped() {
            return Err(GeorefError::RecordsIncomplete {
                diagnostics: self.0.diagnostics(),
            });
        }
        Ok(value)
    }
}
