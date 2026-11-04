//! Checked authoring batches (#330): create entities by type and named
//! attributes, the spatial-structure, product, type and placement builders,
//! removal together with relationships, the removal of a product with
//! what it solely owns and the purge of what nothing uses (#478), the
//! propagation of a type's representations, material usage and
//! predefined type to the objects it is assigned to (#480), and
//! assembling, assigning, unassigning, copying and removing shape
//! representations (#481), and changing an object's class in place (#483),
//! applied as one transaction.
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
//! # SI input
//!
//! With `properties`, `apply_authoring_with` and `stage_authoring_with`
//! take `AuthoringOptions`: under `InputUnits::Si` the numbers an operation
//! states are SI values and are converted before the operation runs; the
//! default, `InputUnits::Raw`, is [`apply_authoring`]. Units are resolved by
//! `ifc-properties` against the model as it was before the batch, so a batch
//! cannot create the project or its units and read numbers in SI at once; a
//! raw batch creates those first.
//!
//! - **Converted:** the attribute values of `Create`, `Edit`, `ReassignClass`,
//!   `Project`, `Spatial`, `Product` and `TypeObject`, a `Placement`'s
//!   location (its axes are directions) and a `CopyProduct`'s
//!   [`CopyPlacement::Offset`]. Each goes into the unit that applies: the
//!   explicit unit of the property, quantity or `IfcMeasureWithUnit` the
//!   number belongs to, else the project unit of its measure. Numbers the
//!   planner derives or copies are not converted.
//! - **How a number is read:** a typed measure by its type name, an untyped
//!   number by the attribute's declared type (the single numeric member of a
//!   `SELECT`), as the release's tables classify it. Reviewed `REAL`
//!   attributes follow their role (a precision is a length, an attenuation an
//!   inverse length, a ratio is kept). An `IfcParameterValue` follows its
//!   curve's or surface's parameter space, read through the batch: lengths and
//!   plane angles convert, unitless parameters are kept. Monetary and
//!   logarithmic measures, and a compound plane angle, pass unchanged. A
//!   temperature difference (`IfcStructuralLoadTemperature.DeltaT*`) converts
//!   by the unit's factor alone, never its offset.
//! - **Refused:** `AuthoringFailure::UnitConversion` when no unit applies
//!   (a project without one of the type, an offset unit, a non-finite number,
//!   a measure with no unit mapping), and `AuthoringFailure::SiInput` with a
//!   `SiInputRefusal` for an unsupported or mismatched release, an
//!   `IfcProjectLibrary` with units, an operation that changes the units
//!   (the unit assignment, a unit, the measure behind a conversion-based
//!   unit, a value holder's `Unit` or enumeration, removing or retyping a
//!   context that states the units, any `Purge`), a model with an
//!   `IfcPcurve` off a plane, a unit created in the batch, a value
//!   whose unit another record states, a parameter space with no single unit
//!   (composite curves with arcs), an ambiguous untyped `SELECT` value,
//!   map conversions, polynomial and texture generator parameters. Nothing
//!   is staged on a refusal.
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
mod reassign;
mod rebase;
mod relate;
mod release;
mod remove;
mod report;
mod represent;
mod shape_type;
#[cfg(feature = "properties")]
mod si;
mod si_refusal;

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

pub use error::{AuthoringError, AuthoringFailure};
pub use op::{AuthorOp, NamedValues};
pub use options::{
    AppendOptions, AssignTypeOptions, CopyPlacement, CopyPolicy, CopyProductOptions, GlobalIdMode,
    LayerAxis, OwnerHistoryOp, PredefinedTypeMode, PurgeOptions, ReassignClassOptions,
    RelateOptions, RemoveProductOptions, ReuseOptions,
};
pub use report::{
    CopiedEntity, CopyReport, KeepReason, KeptEntity, ReassignReport, RelationshipConflict,
    RemovalReport, RemovedEntity, RemovedRelationship,
};

#[cfg(feature = "properties")]
use crate::input_units::{AuthoringOptions, InputUnits};
use overlay::Overlay;
use plan::Planner;

/// The SI input items the crate root re-exports by glob (#638), so the root
/// names none of them under a `properties` gate: the refusal type is plain
/// data and always exists, so the glob is never empty.
pub(crate) mod si_input {
    pub use super::si_refusal::SiInputRefusal;
    #[cfg(feature = "properties")]
    pub use super::{apply_authoring_with, stage_authoring_with};
    #[cfg(feature = "properties")]
    pub use crate::input_units::AuthoringOptions;
}

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
    /// Per operation, what an [`AuthorOp::ReassignClass`] would change and
    /// drop (#483).
    pub reassignments: Vec<Option<ReassignReport>>,
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
    /// Per operation, what an [`AuthorOp::ReassignClass`] changed and
    /// dropped (#483).
    pub reassignments: Vec<Option<ReassignReport>>,
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
    stage(model, schema, ops, seed, &mut |_, op| Ok(op))
}

/// [`stage_authoring`] reading the batch's numbers as `options` say (#638).
///
/// With `InputUnits::Si` every number the operations state is an SI value,
/// converted into the unit that applies before the operation runs (see the
/// module documentation); `InputUnits::Raw` is [`stage_authoring`].
///
/// # Errors
///
/// As [`stage_authoring`]; with `Si`, [`AuthoringFailure::UnitConversion`]
/// for a number that cannot be converted and [`AuthoringFailure::SiInput`]
/// for a batch or operation SI input refuses.
#[cfg(feature = "properties")]
pub fn stage_authoring_with(
    model: &Model,
    schema: &Schema,
    ops: &[AuthorOp],
    seed: u128,
    options: &AuthoringOptions,
) -> Result<StagedAuthoring, AuthoringError> {
    match options.input_units {
        InputUnits::Si => {
            let mut si = si::SiInput::new(model, schema)
                .map_err(|failure| AuthoringError::at(None, failure))?;
            stage(model, schema, ops, seed, &mut |view, op| {
                si.convert(view, op)
            })
        }
        InputUnits::Raw => stage_authoring(model, schema, ops, seed),
    }
}

/// The operations' conversion between handle resolution and planning.
type Convert<'a> = dyn FnMut(&Overlay<'_>, AuthorOp) -> Result<AuthorOp, AuthoringFailure> + 'a;

fn stage(
    model: &Model,
    schema: &Schema,
    ops: &[AuthorOp],
    seed: u128,
    convert: &mut Convert<'_>,
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
            .and_then(|op| convert(&planner.view, op))
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
        reassignments: planner.reassignments,
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
    commit(model, staged)
}

fn commit(model: &mut Model, staged: StagedAuthoring) -> Result<AuthoringOutcome, AuthoringError> {
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
        reassignments: staged.reassignments,
        revision: applied.revision,
    })
}

/// [`apply_authoring`] reading the batch's numbers as `options` say (#638):
/// [`stage_authoring_with`], then the commit.
///
/// # Errors
///
/// As [`stage_authoring_with`] and [`apply_authoring`]. The model is
/// unchanged on every error.
#[cfg(feature = "properties")]
pub fn apply_authoring_with(
    model: &mut Model,
    schema: &Schema,
    ops: &[AuthorOp],
    seed: u128,
    options: &AuthoringOptions,
) -> Result<AuthoringOutcome, AuthoringError> {
    let staged = stage_authoring_with(model, schema, ops, seed, options)?;
    commit(model, staged)
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
