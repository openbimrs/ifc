//! What the reader reported, as unit resolution reads it (#615, #684).
//!
//! [`RecordDamage`] classifies the diagnostics; this module maps its answers
//! to [`ExactUnitError`].

use ifc_model::{EntityId, Model, RecordDamage};

use super::super::ExactPropertyError;
use super::ExactUnitError;

/// The reader's diagnostics, sorted into what resolution must refuse.
pub(super) struct Damage {
    record: RecordDamage,
}

impl Damage {
    /// Classify `model`'s diagnostics by kind.
    pub(super) fn of(model: &Model) -> Result<Self, ExactUnitError> {
        RecordDamage::of(model)
            .map(|record| Self { record })
            .map_err(|_| incomplete(model.diagnostics().len()))
    }

    /// Refuse a record the reader reported by id.
    pub(super) fn check(&self, id: EntityId) -> Result<(), ExactUnitError> {
        match self.record.reported(id) {
            Some(diagnostic) => Err(ExactUnitError::DamagedRecord {
                entity: id,
                diagnostic,
            }),
            None => Ok(()),
        }
    }

    /// Pass a successful resolution through unless a skipped record names an
    /// id a kept record holds. Resolution refuses such a record when it
    /// reads it ([`Damage::check`]); when it does not, the skip may still
    /// have swallowed records it would have read, so the model is
    /// incomplete (#668).
    pub(super) fn settle<T>(&self, resolved: T) -> Result<T, ExactUnitError> {
        if self.record.held_id_skipped() {
            return Err(incomplete(self.record.diagnostics()));
        }
        Ok(resolved)
    }

    /// The refusal for a model with no `IfcProject`: a skipped record may
    /// have been the project, so the project is not shown to be absent.
    pub(super) fn no_project(&self) -> ExactUnitError {
        if self.record.records_lost() {
            incomplete(self.record.diagnostics())
        } else {
            ExactUnitError::NoProject
        }
    }
}

fn incomplete(diagnostics: usize) -> ExactUnitError {
    ExactUnitError::Structure(ExactPropertyError::IncompleteModel { diagnostics })
}
