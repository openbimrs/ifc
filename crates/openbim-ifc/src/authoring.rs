//! Checked authoring batches (#330): create entities by type and named
//! attributes, the spatial-structure, product, type and placement builders,
//! removal together with relationships, the removal of a product with
//! what it solely owns and the purge of what nothing uses (#478), the
//! propagation of a type's representations, material usage and
//! predefined type to the objects it is assigned to (#480), and
//! assembling, assigning, unassigning, copying and removing shape
//! representations (#481), applied as one transaction.
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
//! # Cost
//!
//! Staging a batch costs the batch, not the model, once the model has its
//! indexes: [`Model::reverse_index`](ifc_model::Model::reverse_index) and
//! the GlobalId index, which the model builds on the first request (one scan
//! of every entity) and keeps current through every write, undo and redo
//! (ADR 0023). The first batch against a model pays that scan; every later
//! one reads the indexes, and the planner indexes its own records. An
//! operation therefore costs what it creates, plus the length of any list
//! attribute it edits: editing a list rechecks the whole list, while
//! assigning objects to a type appends them. A caller that only wants the
//! memory back calls [`Model::drop_derived_indexes`](ifc_model::Model::drop_derived_indexes).
//!
//! # Identity and ownership
//!
//! An `IfcRoot` created without a `GlobalId` gets one derived from the seed,
//! the operation's position, the entity's type and its count in the
//! operation, not its id, so a fixed seed reproduces the `GlobalId`s after a
//! write/read cycle. A seed names one batch: a derived `GlobalId` the model
//! holds is refused with [`AuthoringFailure::SeedReused`], and
//! [`fresh_seed`](crate::fresh_seed) draws one per batch. `OwnerHistory` is never invented, the way
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

mod append;
mod cascade;
mod check;
mod copy;
mod copy_entity;
mod equivalence;
mod error;
mod identity;
mod kinds;
mod op;
mod options;
mod overlay;
mod owner;
mod place;
mod plan;
mod propagate;
mod purge;
mod rebase;
mod relate;
mod release;
mod remove;
mod report;
mod represent;
mod shape_type;

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

pub use error::{AuthoringError, AuthoringFailure};
pub use op::{AuthorOp, NamedValues};
pub use options::{
    AppendOptions, AssignTypeOptions, CopyPlacement, CopyPolicy, CopyProductOptions, GlobalIdMode,
    LayerAxis, OwnerHistoryOp, PredefinedTypeMode, PurgeOptions, RelateOptions,
    RemoveProductOptions, ReuseOptions,
};
pub use report::{CopiedEntity, CopyReport, KeepReason, KeptEntity, RemovalReport, RemovedEntity};

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
    /// Per operation, what a [`AuthorOp::RemoveProduct`],
    /// [`AuthorOp::Purge`], [`AuthorOp::UnassignRepresentation`] or
    /// [`AuthorOp::RemoveRepresentation`] would remove and keep: staging is
    /// the dry run.
    pub reports: Vec<Option<RemovalReport>>,
    /// Per operation, what an [`AuthorOp::CopyProduct`] or
    /// [`AuthorOp::CopyEntity`] would copy (#479).
    pub copies: Vec<Option<CopyReport>>,
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
    /// Per operation, what a [`AuthorOp::RemoveProduct`],
    /// [`AuthorOp::Purge`], [`AuthorOp::UnassignRepresentation`] or
    /// [`AuthorOp::RemoveRepresentation`] removed and kept, and why.
    pub reports: Vec<Option<RemovalReport>>,
    /// Per operation, what an [`AuthorOp::CopyProduct`] or
    /// [`AuthorOp::CopyEntity`] copied, original by original (#479).
    pub copies: Vec<Option<CopyReport>>,
    /// The model revision after the commit.
    pub revision: u64,
}

/// Plan `ops` against `model` and `schema` (the release the header
/// declares) and stage them on one transaction, without writing.
///
/// # Errors
///
/// The first refused operation, with its position; nothing is staged.
/// [`AuthoringFailure::SeedReused`] when a `GlobalId` derived from `seed`
/// is already held by an entity of the model.
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
            .run(index, op)
            .map_err(|failure| AuthoringError::at(Some(index), failure))?;
        planner.ids.push(id);
    }
    Ok(StagedAuthoring {
        ids: planner.ids,
        reports: planner.reports,
        copies: planner.copies,
        transaction: planner.view.into_transaction(),
    })
}

/// Plan, stage and commit `ops` as one transaction: all of them or none.
///
/// # Errors
///
/// As [`stage_authoring`] (including [`AuthoringFailure::SeedReused`]), and
/// [`AuthoringFailure::Conflict`] when the
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
        reports: staged.reports,
        copies: staged.copies,
        revision: applied.revision,
    })
}

/// An append planned and staged but not committed (#479).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StagedAppend {
    /// Every change, as one transaction against the target model.
    pub transaction: Transaction,
    /// What the append would copy, reuse and skip: staging is the dry run.
    pub report: CopyReport,
}

/// What a committed append did (#479).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AppendOutcome {
    /// Every source entity copied with its copy, the resources reused, and
    /// the relationships skipped.
    pub report: CopyReport,
    /// Entities created, in commit order.
    pub created: Vec<EntityId>,
    /// The target model's revision after the commit.
    pub revision: u64,
}

/// Plan copying `roots` of `source` (read in `source_schema`, the release
/// its header declares) into `target` (in `target_schema`), and stage it
/// on one transaction, without writing. See [`apply_append`].
///
/// # Errors
///
/// As [`apply_append`] (including [`AuthoringFailure::SeedReused`]); nothing
/// is staged.
#[allow(clippy::too_many_arguments)]
pub fn stage_append(
    target: &Model,
    target_schema: &Schema,
    source: &Model,
    source_schema: &Schema,
    roots: &[EntityId],
    options: &AppendOptions,
    seed: u128,
) -> Result<StagedAppend, AuthoringError> {
    let next_id = target.next_id().0;
    if next_id > HANDLE_BASE {
        return Err(AuthoringError::at(
            None,
            AuthoringFailure::IdRangeExhausted { next_id },
        ));
    }
    let mut planner = Planner::new(target_schema, Overlay::new(target), seed);
    let from = append::Source::new(source, source_schema);
    let report = planner
        .append(&from, roots, options)
        .map_err(|failure| AuthoringError::at(None, failure))?;
    Ok(StagedAppend {
        transaction: planner.view.into_transaction(),
        report,
    })
}

/// Copy `roots` of `source` into `target` with what belongs to them, as
/// one checked transaction (#479): IfcOpenShell's `append_asset` and
/// merge. A type, a material, a profile, a property set template or a
/// product comes with everything it references, its property sets, type,
/// material and classification, its openings, fillings and ports, and
/// what attaches to its geometry; units, contexts, the owner history,
/// materials and styles equal to the target's are reused
/// ([`ReuseOptions`]). Relationships to objects not copied (a spatial
/// container) are skipped and reported. New `GlobalId`s are drawn from
/// `seed` where [`GlobalIdMode`] asks.
///
/// # Errors
///
/// [`AuthoringFailure::UnitMismatch`] when the models' length or plane
/// angle units differ; [`AuthoringFailure::IncompatibleRelease`] for an
/// entity the target release cannot hold; [`AuthoringFailure::DuplicateGlobalId`]
/// for a kept `GlobalId` the target holds; [`AuthoringFailure::SeedReused`]
/// for a regenerated `GlobalId` derived from `seed` that the target holds;
/// [`AuthoringFailure::MissingEntity`]
/// for a root `source` lacks; [`AuthoringFailure::Conflict`] when the
/// planned transaction fails its preflight. The target is unchanged on
/// every error.
#[allow(clippy::too_many_arguments)]
pub fn apply_append(
    target: &mut Model,
    target_schema: &Schema,
    source: &Model,
    source_schema: &Schema,
    roots: &[EntityId],
    options: &AppendOptions,
    seed: u128,
) -> Result<AppendOutcome, AuthoringError> {
    let staged = stage_append(
        target,
        target_schema,
        source,
        source_schema,
        roots,
        options,
        seed,
    )?;
    let applied = staged
        .transaction
        .commit(target)
        .map_err(|conflicts| AuthoringError::at(None, AuthoringFailure::Conflict(conflicts)))?;
    Ok(AppendOutcome {
        report: staged.report,
        created: applied.created,
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
        Value::List(items) => items
            .iter_mut()
            .try_for_each(|item| resolve_value(item, real)),
        Value::Typed { value, .. } => resolve_value(value, real),
        _ => Ok(()),
    }
}
