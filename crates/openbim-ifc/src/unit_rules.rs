//! The release tables that length conversion (#469) and SI input (#638)
//! share, so one reviewed table serves both.
//!
//! - [`classify`]: which declared types are measures, and the reviewed
//!   `REAL` and parameter attributes.
//! - [`governing`]: which values state their own unit.
//! - [`space`]: the parameter space of a curve or surface instance.
//!
//! Each reads records through [`Records`], so it runs over a [`Model`] and
//! over an authoring overlay alike.

#![cfg(feature = "properties")]

use ifc_model::{Entity, EntityId, Model};

pub(crate) mod classify;
pub(crate) mod governing;
pub(crate) mod space;

/// A source of entity records by id.
pub(crate) trait Records {
    /// The record of `id`, if it exists.
    fn record(&self, id: EntityId) -> Option<&Entity>;
}

impl Records for Model {
    fn record(&self, id: EntityId) -> Option<&Entity> {
        self.get(id)
    }
}
