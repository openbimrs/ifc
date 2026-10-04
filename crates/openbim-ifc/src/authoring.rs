//! Checked authoring batches (#330): create entities by type and named
//! attributes, the spatial-structure, product, type and placement builders,
//! and removal together with relationships, applied as one transaction.
//!
//! A batch is a list of [`AuthorOp`]s run in order against the model as the
//! operations before them leave it, then committed as one
//! [`Transaction`](ifc_model::Transaction): every operation, or, when any
//! is refused, none, and the model is unchanged.
//!
//! # What is checked
//!
//! Every record is built by attribute name through `ifc-author`'s
//! [`EntityBuilder`](ifc_author::EntityBuilder) against the release the
//! caller passes, the one the header declares (as for
//! [`attribute_slots`](crate::attribute_slots)): the type exists and is not
//! abstract; each name resolves, case-insensitively; values have the
//! declared type, form and aggregate shape; required attributes are present;
//! derived attributes are not set. Beyond one record: each aggregate keeps
//! its declared bounds and uniqueness, each reference resolves to an entity
//! of a type the attribute accepts, and a supplied `GlobalId` is not one
//! another entity holds. An edit checks the whole edited record again, as
//! `ifc-author`'s `EntityEditor` does.
//!
//! # Identity and ownership
//!
//! An `IfcRoot` created without a `GlobalId` gets one, name-based over a
//! seed and its entity id ([`fresh_seed`] draws a seed; a fixed seed
//! reproduces a file). `OwnerHistory` is never invented, the way
//! `ifc-author` handles it: a builder writes the one the caller names on the
//! entity and every relationship it creates, and IFC2X3, which requires it,
//! refuses a record without. [`AuthorOp::OwnerHistory`] builds one through
//! `ifc-author`.
//!
//! # Handles
//!
//! An operation refers to the entity an earlier operation of the same batch
//! produced by [`authoring_handle`]`(index)`: an id in a reserved range,
//! [`HANDLE_BASE`] plus the operation's position, usable anywhere an id is,
//! inside attribute values too. It resolves before the operation runs, so it
//! may only name an earlier operation that produced an entity. The outcome
//! reports, per operation, the id the produced entity received.
//!
//! ```
//! # #[cfg(feature = "ifc4")] {
//! use ifc::{apply_authoring, authoring_handle, AuthorOp, Model, Value};
//!
//! let mut model = Model::new();
//! let schema = ifc::schema::ifc4();
//! let ops = [
//!     AuthorOp::Project {
//!         attributes: vec![("Name".into(), Value::Text("Demo".into()))],
//!         owner_history: None,
//!     },
//!     AuthorOp::Spatial {
//!         type_name: "IfcSite".into(),
//!         parent: authoring_handle(0),
//!         attributes: vec![],
//!         placement: None,
//!         owner_history: None,
//!     },
//! ];
//! let outcome = apply_authoring(&mut model, schema, &ops, 7).unwrap();
//! let site = outcome.ids[1].unwrap();
//! assert_eq!(&*model.get(site).unwrap().type_name, "IFCSITE");
//! # }
//! ```
#![cfg(feature = "authoring")]

mod check;
mod error;
mod op;
mod overlay;
mod owner;
mod place;
mod plan;
mod relate;
mod remove;
mod seed;

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

pub use error::{AuthoringError, AuthoringFailure};
pub use op::{AuthorOp, NamedValues, OwnerHistoryOp};
pub use seed::fresh_seed;

use overlay::Overlay;
use plan::Planner;

/// The first id of the handle range: `HANDLE_BASE + i` names the entity
/// operation `i` of the batch produced. 2^62, so a handle fits every host's
/// signed 64-bit integer.
pub const HANDLE_BASE: u64 = 1 << 62;

/// The handle of the entity operation `op` of the batch produces.
#[must_use]
pub const fn authoring_handle(op: usize) -> EntityId {
    EntityId(HANDLE_BASE + op as u64)
}

/// A batch planned and staged but not committed.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StagedAuthoring {
    /// Every change, as one transaction against the model it was planned on.
    pub transaction: Transaction,
    /// Per operation, the id of the entity it produces.
    pub ids: Vec<Option<EntityId>>,
}

/// What a committed batch did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthoringOutcome {
    /// Per operation, the id of the entity it produced; `None` for a
    /// removal.
    pub ids: Vec<Option<EntityId>>,
    /// Entities created, in commit order.
    pub created: Vec<EntityId>,
    /// Entities removed, the targets and the relationships removed with
    /// them.
    pub removed: Vec<EntityId>,
    /// The model revision after the commit.
    pub revision: u64,
}

/// Plan `ops` against `model` and `schema` (the release the header
/// declares) and stage them on one transaction, without writing.
///
/// # Errors
///
/// The first refused operation, with its position; nothing is staged.
pub fn stage_authoring(
    model: &Model,
    schema: &Schema,
    ops: &[AuthorOp],
    seed: u128,
) -> Result<StagedAuthoring, AuthoringError> {
    let next_id = model.next_id().0;
    if next_id > HANDLE_BASE {
        return Err(AuthoringError::at(
            None,
            AuthoringFailure::IdRangeExhausted { next_id },
        ));
    }
    let mut planner = Planner::new(schema, Overlay::new(model), seed);
    for (index, op) in ops.iter().enumerate() {
        let op = resolve(op.clone(), index, &planner.ids)
            .map_err(|failure| AuthoringError::at(Some(index), failure))?;
        let id = planner
            .run(op)
            .map_err(|failure| AuthoringError::at(Some(index), failure))?;
        planner.ids.push(id);
    }
    Ok(StagedAuthoring {
        ids: planner.ids,
        transaction: planner.view.into_transaction(),
    })
}

/// Plan, stage and commit `ops` as one transaction: all of them or none.
///
/// # Errors
///
/// As [`stage_authoring`], and [`AuthoringFailure::Conflict`] when the
/// planned transaction fails its preflight. The model is unchanged on every
/// error.
pub fn apply_authoring(
    model: &mut Model,
    schema: &Schema,
    ops: &[AuthorOp],
    seed: u128,
) -> Result<AuthoringOutcome, AuthoringError> {
    let staged = stage_authoring(model, schema, ops, seed)?;
    let applied = staged
        .transaction
        .commit(model)
        .map_err(|conflicts| AuthoringError::at(None, AuthoringFailure::Conflict(conflicts)))?;
    Ok(AuthoringOutcome {
        ids: staged.ids,
        created: applied.created,
        removed: applied.removed.into_iter().map(|(id, _)| id).collect(),
        revision: applied.revision,
    })
}

/// `op` with every handle replaced by the id it names.
fn resolve(
    mut op: AuthorOp,
    index: usize,
    ids: &[Option<EntityId>],
) -> Result<AuthorOp, AuthoringFailure> {
    let real = |id: &mut EntityId| -> Result<(), AuthoringFailure> {
        if id.0 < HANDLE_BASE {
            return Ok(());
        }
        let named = id.0 - HANDLE_BASE;
        let produced = usize::try_from(named)
            .ok()
            .filter(|named| *named < index)
            .ok_or_else(|| AuthoringFailure::InvalidHandle {
                handle: id.0,
                detail: format!("names op {named}, which does not run before op {index}"),
            })?;
        *id = ids[produced].ok_or_else(|| AuthoringFailure::InvalidHandle {
            handle: id.0,
            detail: format!("op {produced} produced no entity"),
        })?;
        Ok(())
    };
    for id in op.ids_mut() {
        real(id)?;
    }
    if let Some(values) = op.values_mut() {
        for (_, value) in values.iter_mut() {
            resolve_value(value, &real)?;
        }
    }
    Ok(op)
}

fn resolve_value(
    value: &mut Value,
    real: &impl Fn(&mut EntityId) -> Result<(), AuthoringFailure>,
) -> Result<(), AuthoringFailure> {
    match value {
        Value::Ref(id) => real(id),
        Value::List(items) => items.iter_mut().try_for_each(|item| resolve_value(item, real)),
        Value::Typed { value, .. } => resolve_value(value, real),
        _ => Ok(()),
    }
}

