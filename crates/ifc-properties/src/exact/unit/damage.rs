//! What the reader reported, as unit resolution reads it (#615).
//!
//! Unit resolution reads a handful of records. A reader diagnostic matters to
//! it only when it names one of those records, or when the diagnostic is of a
//! kind this module does not classify.

use std::collections::BTreeMap;

use ifc_model::{DiagnosticKind, EntityId, Model};

use super::super::ExactPropertyError;
use super::ExactUnitError;

/// The reader's diagnostics, sorted into what resolution must refuse.
pub(super) struct Damage {
    /// The first diagnostic naming each reported record id.
    reported: BTreeMap<EntityId, usize>,
    /// Whether a record was dropped, so an absent record may have existed.
    lost: bool,
    /// How many diagnostics the model carries.
    total: usize,
}

impl Damage {
    /// Classify `model`'s diagnostics by kind.
    ///
    /// A REAL without its point and a dangling reference do not change what a
    /// unit record says. A skipped record or a duplicate id is remembered by
    /// id. Any other kind, or a skipped record or duplicate with no id, refuses the model.
    pub(super) fn of(model: &Model) -> Result<Self, ExactUnitError> {
        let total = model.diagnostics().len();
        let incomplete = || {
            ExactUnitError::Structure(ExactPropertyError::IncompleteModel { diagnostics: total })
        };
        let mut damage = Self {
            reported: BTreeMap::new(),
            lost: false,
            total,
        };
        for (index, diagnostic) in model.diagnostics().iter().enumerate() {
            match diagnostic.kind() {
                DiagnosticKind::RealWithoutPoint | DiagnosticKind::DanglingReference => {}
                DiagnosticKind::SkippedRecord => {
                    damage.lost = true;
                    // The reader drops the id when the model already holds
                    // it, so a record skipped without an id may have been a
                    // damaged duplicate of one this resolution reads.
                    // TODO(#627): attribute such a skip in the reader.
                    let id = diagnostic.instance().ok_or_else(incomplete)?;
                    damage.reported.entry(id).or_insert(index);
                }
                DiagnosticKind::DuplicateId => {
                    let id = diagnostic.instance().ok_or_else(incomplete)?;
                    damage.reported.entry(id).or_insert(index);
                }
                _ => return Err(incomplete()),
            }
        }
        Ok(damage)
    }

    /// Refuse a record the reader reported by id.
    pub(super) fn check(&self, id: EntityId) -> Result<(), ExactUnitError> {
        match self.reported.get(&id) {
            Some(&diagnostic) => Err(ExactUnitError::DamagedRecord {
                entity: id,
                diagnostic,
            }),
            None => Ok(()),
        }
    }

    /// The refusal for a model with no `IfcProject`: a skipped record may
    /// have been the project, so the project is not shown to be absent.
    pub(super) fn no_project(&self) -> ExactUnitError {
        if self.lost {
            ExactUnitError::Structure(ExactPropertyError::IncompleteModel {
                diagnostics: self.total,
            })
        } else {
            ExactUnitError::NoProject
        }
    }
}
