//! What the reader reported, as unit resolution reads it (#684).
//!
//! [`RecordDamage`] classifies the diagnostics; this module maps its answers
//! to [`GeometryError`], with the same verdicts `ifc-properties` gives for the
//! same model.

use ifc_model::{EntityId, Model, RecordDamage};

use super::UnitScale;
use crate::error::{GeometryError, GeometryResult};

/// The reader's diagnostics, sorted into what resolution must refuse.
pub(super) struct Damage(RecordDamage);

impl Damage {
    /// Classify `model`'s diagnostics by kind.
    pub(super) fn of(model: &Model) -> GeometryResult<Self> {
        RecordDamage::of(model)
            .map(Self)
            .map_err(|_| GeometryError::UnitsIncomplete {
                diagnostics: model.diagnostics().len(),
            })
    }

    /// Refuse a record the reader reported by id.
    pub(super) fn check(&self, id: EntityId) -> GeometryResult<()> {
        match self.0.reported(id) {
            Some(diagnostic) => Err(GeometryError::UnitRecordDamaged {
                entity: id,
                diagnostic,
            }),
            None => Ok(()),
        }
    }

    /// The refusal for a model with no `IfcProject`: a skipped record may
    /// have been the project, so the project is not shown to be absent.
    pub(super) fn no_project(&self) -> GeometryError {
        if self.0.records_lost() {
            self.incomplete()
        } else {
            GeometryError::LengthUnitUndeclared {
                project: None,
                assignment: None,
            }
        }
    }

    /// Pass a resolved scale through unless a skipped record names an id a
    /// kept record holds: the skip may have swallowed records resolution
    /// would have read (#668).
    pub(super) fn settle(&self, scale: UnitScale) -> GeometryResult<UnitScale> {
        if self.0.held_id_skipped() {
            return Err(self.incomplete());
        }
        Ok(scale)
    }

    fn incomplete(&self) -> GeometryError {
        GeometryError::UnitsIncomplete {
            diagnostics: self.0.diagnostics(),
        }
    }
}
