//! Schema-checked entity creation (#330, feature `author`).
//!
//! A batch of [`AuthorOp`]s runs as one checked transaction through the
//! facade's `ifc::apply_authoring`, against the release the header
//! declares: every operation, in order, or, when any is refused, none, and
//! the model is unchanged. The operations: create an entity by type and
//! named attributes; edit named attributes; remove an entity with its
//! relationships; the builders -- the project, a spatial element
//! aggregated under its parent, a product placed, contained and typed, a
//! type object, a type assignment, a containment, an aggregation, a local
//! placement and an owner history; and the shape representation operations
//! (#481) -- assemble one from items in a context, its type checked
//! against them or inferred, assign it to a product or (as a
//! representation map) a type, unassign, copy and remove it; and the copy
//! operations (#479) -- a product with what it solely owns, and an entity
//! by an explicit share/copy policy ([`IfcModel::copy_product`],
//! [`IfcModel::append`] from another model).
//!
//! An operation names an entity an earlier one produced by its handle,
//! [`HANDLE_BASE`] plus the operation's position ([`handle`]), anywhere an id
//! goes, attribute values included. The result reports per operation the id
//! the produced entity received.
//!
//! Hosts carry an operation in their own idiom and convert it field by
//! field, guided by [`OPS`], into the tape form [`AuthorOp::from_tagged`]
//! reads (see the `ops` module).
//!
//! Refusals: `unsupported-schema` for a release this build does not bundle,
//! or a type it does not declare; `unknown-attribute`; `derived-attribute`;
//! `missing-attribute` for a required attribute left unset; `invalid-value`
//! for a value of the wrong type, form or cardinality, a duplicate
//! `GlobalId`, a seed reused for a second batch whose derived `GlobalId`s
//! the model already holds, a malformed operation or handle, or a placement the schema
//! cannot hold; `wrong-entity-type` for an abstract type, a type of the
//! wrong kind for its builder, or a reference to an entity the attribute
//! does not accept; `missing-entity` for an edited or removed entity that
//! does not exist; `missing-reference` for a reference to one;
//! `invalid-model` for an object given a second containment, decomposition
//! or type, or a second `IfcProject`, a representation that already has its
//! one use, a product definition shape other products share, or a
//! representation not assigned where it is unassigned from;
//! `invalid-value` also for a representation type its items do not fit,
//! items no type fits, or a missing representation identifier;
//! `unsupported` for a type check needing a curve dimension that cannot be
//! derived; `still-referenced` for a removal an entity other than a
//! relationship still needs; `feature-disabled` without the `author`
//! feature. The message names the refused operation's
//! position.

mod copying;
mod ops;
mod removal;

pub use copying::{AppendOptions, CopiedEntity, CopyProductOptions, CopyReport};
pub use ops::{op_spec, FieldKind, FieldSpec, OpSpec, OPS};
pub use removal::{KeptEntity, PurgeOptions, RemovalReport, RemoveProductOptions, RemovedEntity};

use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// The first id of the handle range: `HANDLE_BASE + i` names the entity
/// operation `i` of a batch produced. 2^62, inside every host's signed
/// 64-bit integer.
pub const HANDLE_BASE: u64 = 1 << 62;

// Restated so builds without the feature still have it; it is the facade's.
#[cfg(feature = "author")]
const _: () = assert!(HANDLE_BASE == ifc::HANDLE_BASE);

/// The handle of operation `index`'s entity.
///
/// # Errors
///
/// `out-of-range` for an index past the handle range.
pub fn handle(index: u64) -> Result<u64, BindingError> {
    HANDLE_BASE
        .checked_add(index)
        .filter(|_| index < HANDLE_BASE)
        .ok_or_else(|| BindingError::OutOfRange(format!("handle index {index}")))
}

/// One operation of a batch, read from its tape form.
#[derive(Debug, Clone)]
pub struct AuthorOp(Inner);

#[cfg(feature = "author")]
type Inner = ifc::AuthorOp;
#[cfg(not(feature = "author"))]
type Inner = std::convert::Infallible;

impl AuthorOp {
    /// Read the tape form: a `LIST` of the operation's `ENUM` name, then
    /// field names and values (see the `ops` module).
    ///
    /// # Errors
    ///
    /// `invalid-value` for a malformed operation; `feature-disabled`
    /// without the `author` feature.
    pub fn from_tagged(value: &Tagged) -> Result<Self, BindingError> {
        let fields = ops::read(value)?;
        #[cfg(feature = "author")]
        {
            convert::op(fields).map(Self)
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = fields;
            Err(BindingError::FeatureDisabled("author"))
        }
    }
}

/// What a committed batch did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringResult {
    /// Per operation, the id of the entity it produced; `None` for a
    /// removal.
    pub ids: Vec<Option<u64>>,
    /// Entities created, in commit order.
    pub created: Vec<u64>,
    /// Entities removed.
    pub removed: Vec<u64>,
}

impl ToRecord for AuthoringResult {
    fn to_record(&self) -> Record {
        Record::new(
            "AuthoringResult",
            vec![
                (
                    "ids",
                    Field::List(self.ids.iter().map(|id| Field::id(*id)).collect()),
                ),
                ("created", Field::ids(self.created.iter().copied())),
                ("removed", Field::ids(self.removed.iter().copied())),
            ],
        )
    }
}

impl IfcModel {
    /// Apply `ops` as one checked transaction, with `GlobalId`s drawn from
    /// a fresh seed. See the module docs for the refusals.
    pub fn author(&mut self, ops: Vec<AuthorOp>) -> Result<AuthoringResult, BindingError> {
        #[cfg(feature = "author")]
        {
            let seed = crate::relations::fresh_seed()?;
            self.tracked(|this| this.author_seeded(ops, seed))
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = ops;
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    /// [`Self::author`] with the seed new `GlobalId`s derive from: a host
    /// with entropy of its own (the browser) passes it. The seed names the
    /// batch: a fixed seed reproduces its `GlobalId`s, also after a
    /// write/read cycle; reusing it for another batch on the same model may
    /// be refused (`invalid-value`).
    pub fn author_seeded(
        &mut self,
        ops: Vec<AuthorOp>,
        seed: u128,
    ) -> Result<AuthoringResult, BindingError> {
        #[cfg(feature = "author")]
        {
            let schema = self.declared_schema()?;
            let ops: Vec<ifc::AuthorOp> = ops.into_iter().map(|AuthorOp(op)| op).collect();
            let outcome = self
                .tracked(|this| ifc::apply_authoring(&mut this.inner, schema, &ops, seed))
                .map_err(convert::error)?;
            let ids = |ids: Vec<ifc::EntityId>| ids.into_iter().map(|id| id.0).collect();
            Ok(AuthoringResult {
                ids: outcome
                    .ids
                    .into_iter()
                    .map(|id| id.map(|id| id.0))
                    .collect(),
                created: ids(outcome.created),
                removed: ids(outcome.removed),
            })
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = (ops, seed);
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    /// Create one entity of `type_name` from named attributes: a batch of
    /// one `create`. Returns its id.
    pub fn create_entity(
        &mut self,
        type_name: &str,
        attributes: Vec<(String, Tagged)>,
    ) -> Result<u64, BindingError> {
        let pairs = attributes
            .into_iter()
            .map(|(name, value)| Tagged::List(vec![Tagged::Text(name), value]))
            .collect();
        let op = AuthorOp::from_tagged(&Tagged::List(vec![
            Tagged::Enum("CREATE".into()),
            Tagged::Text("type".into()),
            Tagged::Text(type_name.to_owned()),
            Tagged::Text("attributes".into()),
            Tagged::List(pairs),
        ]))?;
        let result = self.tracked(|this| this.author(vec![op]))?;
        result
            .ids
            .first()
            .copied()
            .flatten()
            .ok_or_else(|| BindingError::InvalidModel("the batch created nothing".into()))
    }

    /// Remove entity `id` with its relationships: a batch of one `remove`.
    /// Unlike [`Self::remove`], it leaves nothing dangling, and is refused
    /// with `still-referenced` while an entity that is not a relationship
    /// needs `id`.
    pub fn remove_with_relationships(&mut self, id: u64) -> Result<(), BindingError> {
        let op = AuthorOp::from_tagged(&Tagged::List(vec![
            Tagged::Enum("REMOVE".into()),
            Tagged::Text("entity".into()),
            Tagged::Ref(id),
        ]))?;
        self.tracked(|this| this.author(vec![op])).map(drop)
    }
}

#[cfg(feature = "author")]
mod convert;
