//! Removing a product with what it solely owns, and purging what nothing
//! uses (#478, feature `author`).
//!
//! [`IfcModel::remove_product`] and [`IfcModel::purge`] run the facade's
//! `AuthorOp::RemoveProduct` and `AuthorOp::Purge` as a batch of one, or,
//! with `dry_run`, only stage it: the report is the same and the model is
//! unchanged. Every host returns the [`RemovalReport`] as its record.
//!
//! Refusals: `missing-entity` for no such product; `wrong-entity-type` for
//! one that is neither an `IfcProduct` nor an `IfcTypeObject`;
//! `still-referenced` for a placement other placements are relative to
//! (unless re-based), a type object that still types objects (unless
//! they are untyped), or an entity that stays and still needs something
//! removed; `unsupported` for a placement that cannot be re-based;
//! `unsupported-schema` for a purge class the release does not declare;
//! `feature-disabled` without the `author` feature.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// What [`IfcModel::remove_product`] does where it would otherwise refuse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RemoveProductOptions {
    /// Re-base placements relative to a removed placement onto its
    /// parent, composing the transforms, instead of refusing.
    pub rebase_placements: bool,
    /// Remove a type object's typing relationship, leaving its occurrences
    /// untyped, instead of refusing.
    pub untype_occurrences: bool,
    /// Report what would be removed and kept without changing the model.
    pub dry_run: bool,
}

impl RemoveProductOptions {
    /// Every option off.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rebase_placements: false,
            untype_occurrences: false,
            dry_run: false,
        }
    }
}

/// What [`IfcModel::purge`] removes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PurgeOptions {
    /// Entity types to purge, subtypes included; empty for every resource
    /// entity no `IfcRoot` reaches.
    pub classes: Vec<String>,
    /// Also purge unreferenced `IfcRoot`s of `classes`.
    pub include_roots: bool,
    /// Also purge an unreferenced owner history with its actors (and,
    /// without classes, unused representation contexts).
    pub include_owner_history: bool,
    /// Report what would be purged without changing the model.
    pub dry_run: bool,
}

impl PurgeOptions {
    /// The default purge, applied.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// One removed entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedEntity {
    /// Its id.
    pub id: u64,
    /// Its type, upper case as the file writes it.
    pub type_name: String,
}

/// One entity the removal reached and kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptEntity {
    /// Its id.
    pub id: u64,
    /// Its type, upper case.
    pub type_name: String,
    /// Why it stays: `shared` (`users` still need it), `related-object`
    /// (a type, container, filling: an object of its own), `definition`
    /// (a material, classification reference, unit, context or owner
    /// history, which only a purge removes) or `protected` (a purge
    /// skipped it: an `IfcRoot` or the owner history, unless asked).
    pub reason: String,
    /// For `shared`, the entities that stay and use it.
    pub users: Vec<u64>,
}

/// What a removal or purge did, or, for a dry run, would do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemovalReport {
    /// Every entity removed, in id order.
    pub removed: Vec<RemovedEntity>,
    /// Entities reached and kept, with the reason.
    pub kept: Vec<KeptEntity>,
    /// Relationships and layer assignments that lost a removed member and
    /// stay.
    pub detached: Vec<u64>,
    /// Placements re-based onto a kept ancestor.
    pub rebased: Vec<u64>,
    /// Whether the model was changed: `false` for a dry run.
    pub applied: bool,
}

impl ToRecord for RemovedEntity {
    fn to_record(&self) -> Record {
        Record::new(
            "RemovedEntity",
            vec![
                ("id", Field::Id(self.id)),
                ("type_name", Field::Text(self.type_name.clone())),
            ],
        )
    }
}

impl ToRecord for KeptEntity {
    fn to_record(&self) -> Record {
        Record::new(
            "KeptEntity",
            vec![
                ("id", Field::Id(self.id)),
                ("type_name", Field::Text(self.type_name.clone())),
                ("reason", Field::Text(self.reason.clone())),
                ("users", Field::ids(self.users.iter().copied())),
            ],
        )
    }
}

impl ToRecord for RemovalReport {
    fn to_record(&self) -> Record {
        Record::new(
            "RemovalReport",
            vec![
                ("removed", Field::records(&self.removed)),
                ("kept", Field::records(&self.kept)),
                ("detached", Field::ids(self.detached.iter().copied())),
                ("rebased", Field::ids(self.rebased.iter().copied())),
                ("applied", Field::Bool(self.applied)),
            ],
        )
    }
}

impl IfcModel {
    /// Remove product (or type object) `id` with everything it solely
    /// owns, keeping what is shared, as one checked transaction; with
    /// `options.dry_run`, only report. See the module docs for the
    /// refusals.
    pub fn remove_product(
        &mut self,
        id: u64,
        options: RemoveProductOptions,
    ) -> Result<RemovalReport, BindingError> {
        #[cfg(feature = "author")]
        {
            let op = ifc::AuthorOp::RemoveProduct {
                product: ifc::EntityId(id),
                options: ifc::RemoveProductOptions::new()
                    .rebase_placements(options.rebase_placements)
                    .untype_occurrences(options.untype_occurrences),
            };
            self.removal(op, options.dry_run)
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = (id, options);
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    /// Purge unreferenced entities as one checked transaction; with
    /// `options.dry_run`, only report. See the module docs.
    pub fn purge(&mut self, options: PurgeOptions) -> Result<RemovalReport, BindingError> {
        #[cfg(feature = "author")]
        {
            let op = ifc::AuthorOp::Purge(
                ifc::PurgeOptions::new()
                    .classes(options.classes)
                    .include_roots(options.include_roots)
                    .include_owner_history(options.include_owner_history),
            );
            self.removal(op, options.dry_run)
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = options;
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    #[cfg(feature = "author")]
    fn removal(&mut self, op: ifc::AuthorOp, dry_run: bool) -> Result<RemovalReport, BindingError> {
        // Neither operation creates an `IfcRoot`, so no `GlobalId` is
        // drawn from the seed.
        const SEED: u128 = 0;
        let schema = self.declared_schema()?;
        let ops = [op];
        let reports = if dry_run {
            ifc::stage_authoring(&self.inner, schema, &ops, SEED)
                .map_err(super::convert::error)?
                .reports
        } else {
            ifc::apply_authoring(&mut self.inner, schema, &ops, SEED)
                .map_err(super::convert::error)?
                .reports
        };
        let report = reports
            .into_iter()
            .next()
            .flatten()
            .ok_or_else(|| BindingError::InvalidModel("the removal made no report".into()))?;
        Ok(RemovalReport {
            removed: report
                .removed
                .into_iter()
                .map(|entity| RemovedEntity {
                    id: entity.id.0,
                    type_name: entity.type_name,
                })
                .collect(),
            kept: report
                .kept
                .into_iter()
                .map(|entity| KeptEntity {
                    id: entity.id.0,
                    type_name: entity.type_name,
                    reason: entity.reason.as_str().to_owned(),
                    users: entity.users.into_iter().map(|user| user.0).collect(),
                })
                .collect(),
            detached: report.detached.into_iter().map(|id| id.0).collect(),
            rebased: report.rebased.into_iter().map(|id| id.0).collect(),
            applied: !dry_run,
        })
    }
}
