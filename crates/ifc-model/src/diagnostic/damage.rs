//! A model's reader diagnostics, classified for a resolution that reads a
//! known set of records (#615, #684).
//!
//! Such a resolution (units, say) reads a handful of records through
//! [`Model::get`] and refuses a missing one. A reader diagnostic matters to it
//! only when it names one of those records, or when the diagnostic is of a
//! kind this module does not classify:
//!
//! - a REAL without its decimal point and a dangling reference do not change
//!   what a kept record says, so they are ignored;
//! - a skipped record or a duplicate id is remembered by the id the reader
//!   named, so a resolution can refuse exactly the record it reads;
//! - a skipped record or duplicate with no id, and every other kind
//!   (including kinds added later), cannot be shown harmless, so
//!   [`RecordDamage::of`] refuses the model.
//!
//! The rule reads only the model's diagnostics and ids. It carries no IFC
//! meaning and names no codec, so every domain crate shares it.

use std::collections::BTreeMap;
use std::fmt;

use crate::{DiagnosticKind, EntityId, Model};

/// A model's reader diagnostics, sorted into what a resolution must refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDamage {
    /// The position of the first diagnostic naming each reported record id.
    reported: BTreeMap<EntityId, usize>,
    /// Whether a record was dropped, so an absent record may have existed.
    records_lost: bool,
    /// Whether a skipped record names an id a kept record holds.
    held_id_skipped: bool,
    /// How many diagnostics the model carries.
    diagnostics: usize,
}

/// A diagnostic [`RecordDamage::of`] cannot classify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnclassifiedDiagnostic {
    /// Position in [`Model::diagnostics`].
    pub index: usize,
    /// The kind of the diagnostic.
    pub kind: DiagnosticKind,
}

impl fmt::Display for UnclassifiedDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "diagnostic {} ({:?}) names no record a resolution can check",
            self.index, self.kind
        )
    }
}

impl std::error::Error for UnclassifiedDiagnostic {}

impl RecordDamage {
    /// Classify `model`'s diagnostics by kind.
    ///
    /// # Errors
    /// [`UnclassifiedDiagnostic`] for the first diagnostic of a kind other
    /// than skipped record, duplicate id, dangling reference or REAL without
    /// a point, and for a skipped record or duplicate id that names no
    /// record.
    pub fn of(model: &Model) -> Result<Self, UnclassifiedDiagnostic> {
        let mut damage = Self {
            reported: BTreeMap::new(),
            records_lost: false,
            held_id_skipped: false,
            diagnostics: model.diagnostics().len(),
        };
        for (index, diagnostic) in model.diagnostics().iter().enumerate() {
            let kind = diagnostic.kind();
            let unclassified = UnclassifiedDiagnostic { index, kind };
            match kind {
                DiagnosticKind::RealWithoutPoint | DiagnosticKind::DanglingReference => {}
                DiagnosticKind::SkippedRecord => {
                    damage.records_lost = true;
                    // The reader names the id it read, also when a kept
                    // record holds it; such a record is reported. A skip
                    // whose `#id` was not read may have held a duplicate of a
                    // record the resolution reads.
                    // TODO(#668): a skip that swallowed several records names
                    // only the first.
                    let id = diagnostic.instance().ok_or(unclassified)?;
                    damage.held_id_skipped |= model.contains(id);
                    damage.reported.entry(id).or_insert(index);
                }
                DiagnosticKind::DuplicateId => {
                    let id = diagnostic.instance().ok_or(unclassified)?;
                    damage.reported.entry(id).or_insert(index);
                }
                _ => return Err(unclassified),
            }
        }
        Ok(damage)
    }

    /// Position in [`Model::diagnostics`] of the first diagnostic naming `id`.
    #[must_use]
    pub fn reported(&self, id: EntityId) -> Option<usize> {
        self.reported.get(&id).copied()
    }

    /// Whether a record was skipped, so an absent record may have existed.
    #[must_use]
    pub fn records_lost(&self) -> bool {
        self.records_lost
    }

    /// Whether a skipped record names an id a kept record holds. Such a skip
    /// may have swallowed later records (#668).
    #[must_use]
    pub fn held_id_skipped(&self) -> bool {
        self.held_id_skipped
    }

    /// How many diagnostics the model carries.
    #[must_use]
    pub fn diagnostics(&self) -> usize {
        self.diagnostics
    }
}
