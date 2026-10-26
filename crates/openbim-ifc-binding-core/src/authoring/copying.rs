//! Copying a product with what it solely owns, and appending entities from
//! another model (#479, feature `author`).
//!
//! [`IfcModel::copy_product`] runs the facade's `AuthorOp::CopyProduct` as
//! a batch of one; [`IfcModel::append`] runs `ifc::apply_append`. With
//! `dry_run` each only plans: the report is the same and the model is
//! unchanged. Every host returns the [`CopyReport`] as its record.
//!
//! Refusals: `missing-entity` for no such product or source root;
//! `wrong-entity-type` for a product that is neither an `IfcProduct` nor
//! an `IfcTypeObject`; `unsupported` for an offset onto a placement that is
//! not a local placement with a Cartesian location, a moved copy whose
//! openings, fillings or ports are not placed relative to it, and models
//! whose length or plane angle units differ (convert one with
//! `convert_length_unit` first, #469);
//! `unsupported-schema` for an entity the target release cannot hold (a
//! type or attribute it lacks, a constant it does not know); `invalid-value`
//! for a `GlobalId` the target holds already, kept, and for an unknown
//! `GlobalId` mode; `feature-disabled` without the `author` feature.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// What [`IfcModel::copy_product`] copies and joins (the facade's
/// `CopyProductOptions`).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct CopyProductOptions {
    /// Move the copy by this vector, in its parent placement's frame.
    pub offset: Option<[f64; 3]>,
    /// Place the copy by this `IfcObjectPlacement` instead.
    pub placement: Option<u64>,
    /// Copy the openings voiding the product.
    pub openings: bool,
    /// Copy the doors and windows filling a copied opening.
    pub fillings: bool,
    /// Contain the copy in the original's spatial structure element.
    pub container: bool,
    /// Make the copy a part of the original's whole.
    pub aggregate: bool,
    /// Report what would be copied without changing the model.
    pub dry_run: bool,
}

impl Default for CopyProductOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CopyProductOptions {
    /// In place, with openings and fillings, in the original's container
    /// and whole, applied: the default.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            offset: None,
            placement: None,
            openings: true,
            fillings: true,
            container: true,
            aggregate: true,
            dry_run: false,
        }
    }
}

/// How [`IfcModel::append`] copies from another model (the facade's
/// `AppendOptions`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AppendOptions {
    /// `keep` (the default), `regenerate`, `regenerate-colliding` or
    /// `merge`.
    pub global_ids: String,
    /// Reuse the target's equal units.
    pub reuse_units: bool,
    /// Reuse the target's equal representation contexts.
    pub reuse_contexts: bool,
    /// Reuse the target's equal owner history and actors.
    pub reuse_owner_history: bool,
    /// Reuse the target's equal material definitions.
    pub reuse_materials: bool,
    /// Reuse the target's equal presentation styles.
    pub reuse_styles: bool,
    /// Report what would be copied without changing the model.
    pub dry_run: bool,
}

impl Default for AppendOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl AppendOptions {
    /// Keep `GlobalId`s, reuse every equal resource, applied: the default.
    #[must_use]
    pub fn new() -> Self {
        Self {
            global_ids: "keep".to_owned(),
            reuse_units: true,
            reuse_contexts: true,
            reuse_owner_history: true,
            reuse_materials: true,
            reuse_styles: true,
            dry_run: false,
        }
    }
}

/// One original and its copy (or the target entity reused for it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedEntity {
    /// The original's id: in this model for a copy, in the source for an
    /// append.
    pub original: u64,
    /// The copy's id.
    pub copy: u64,
    /// Its type, upper case.
    pub type_name: String,
}

/// What a copy or an append did, or, for a dry run, would do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CopyReport {
    /// The product's copy; `None` for an append.
    pub copy: Option<u64>,
    /// Every entity copied, in the order the copies were made.
    pub copied: Vec<CopiedEntity>,
    /// Append: source entities the target holds an equal one of, with it.
    pub reused: Vec<CopiedEntity>,
    /// Existing relationships and layers the copies joined.
    pub joined: Vec<u64>,
    /// Relationships of an original the copies do not get.
    pub skipped: Vec<u64>,
    /// Append: copies whose `GlobalId` was regenerated.
    pub regenerated: Vec<u64>,
    /// Whether the model was changed: `false` for a dry run.
    pub applied: bool,
}

impl ToRecord for CopiedEntity {
    fn to_record(&self) -> Record {
        Record::new(
            "CopiedEntity",
            vec![
                ("original", Field::Id(self.original)),
                ("copy", Field::Id(self.copy)),
                ("type_name", Field::Text(self.type_name.clone())),
            ],
        )
    }
}

impl ToRecord for CopyReport {
    fn to_record(&self) -> Record {
        Record::new(
            "CopyReport",
            vec![
                ("copy", Field::id(self.copy)),
                ("copied", Field::records(&self.copied)),
                ("reused", Field::records(&self.reused)),
                ("joined", Field::ids(self.joined.iter().copied())),
                ("skipped", Field::ids(self.skipped.iter().copied())),
                ("regenerated", Field::ids(self.regenerated.iter().copied())),
                ("applied", Field::Bool(self.applied)),
            ],
        )
    }
}

impl IfcModel {
    /// Copy product (or type object) `id` with everything it solely owns,
    /// as one checked transaction: its placement (moved or replaced by
    /// option), representation, own property sets, material usage, ports
    /// and, by option, openings and fillings; joining its type, shared
    /// property sets, classification and, by option, container and whole.
    /// With `options.dry_run`, only report. See the module docs for the
    /// refusals.
    pub fn copy_product(
        &mut self,
        id: u64,
        options: CopyProductOptions,
    ) -> Result<CopyReport, BindingError> {
        #[cfg(feature = "author")]
        {
            self.copy_product_seeded(id, options, ifc::fresh_seed())
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = (id, options);
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    /// [`Self::copy_product`] with the seed the copy's new `GlobalId`s
    /// derive from: a host with entropy of its own (the browser) passes it.
    /// The seed names the batch; reusing it on a model that holds its
    /// `GlobalId`s is refused (`invalid-value`).
    pub fn copy_product_seeded(
        &mut self,
        id: u64,
        options: CopyProductOptions,
        seed: u128,
    ) -> Result<CopyReport, BindingError> {
        #[cfg(feature = "author")]
        {
            let placement = match (options.offset, options.placement) {
                (Some(_), Some(_)) => {
                    return Err(BindingError::InvalidValue(
                        "copy_product takes `offset` or `placement`, not both".into(),
                    ))
                }
                (Some(offset), None) => ifc::CopyPlacement::Offset(offset),
                (None, Some(placement)) => ifc::CopyPlacement::At(ifc::EntityId(placement)),
                (None, None) => ifc::CopyPlacement::Same,
            };
            let op = ifc::AuthorOp::CopyProduct {
                product: ifc::EntityId(id),
                options: ifc::CopyProductOptions::new()
                    .placement(placement)
                    .openings(options.openings)
                    .fillings(options.fillings)
                    .container(options.container)
                    .aggregate(options.aggregate),
            };
            let schema = self.declared_schema()?;
            let ops = [op];
            let (ids, copies) = if options.dry_run {
                let staged = ifc::stage_authoring(&self.inner, schema, &ops, seed)
                    .map_err(super::convert::error)?;
                (staged.ids, staged.copies)
            } else {
                let outcome = ifc::apply_authoring(&mut self.inner, schema, &ops, seed)
                    .map_err(super::convert::error)?;
                (outcome.ids, outcome.copies)
            };
            let report = copies
                .into_iter()
                .next()
                .flatten()
                .ok_or_else(|| BindingError::InvalidModel("the copy made no report".into()))?;
            let mut report = convert(report, !options.dry_run);
            report.copy = ids.first().copied().flatten().map(|id| id.0);
            Ok(report)
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = (id, options, seed);
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    /// Copy `roots` of `source` into this model with what belongs to them,
    /// as one checked transaction: what they reference, their property
    /// sets, type, material and classification, openings, fillings and
    /// ports; units, contexts, owner history, materials and styles equal
    /// to this model's are reused. Relationships to objects not copied are
    /// skipped and reported. With `options.dry_run`, only report. See the
    /// module docs for the refusals.
    pub fn append(
        &mut self,
        source: &IfcModel,
        roots: &[u64],
        options: &AppendOptions,
    ) -> Result<CopyReport, BindingError> {
        #[cfg(feature = "author")]
        {
            self.append_seeded(source, roots, options, ifc::fresh_seed())
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = (source, roots, options);
            Err(BindingError::FeatureDisabled("author"))
        }
    }

    /// [`Self::append`] with the seed new `GlobalId`s derive from: a host
    /// with entropy of its own (the browser) passes it. The seed names the
    /// batch; reusing it on a model that holds its `GlobalId`s is refused
    /// (`invalid-value`).
    pub fn append_seeded(
        &mut self,
        source: &IfcModel,
        roots: &[u64],
        options: &AppendOptions,
        seed: u128,
    ) -> Result<CopyReport, BindingError> {
        #[cfg(feature = "author")]
        {
            let global_ids = ifc::GlobalIdMode::parse(&options.global_ids).ok_or_else(|| {
                BindingError::InvalidValue(format!(
                    "GlobalId mode `{}` is not keep, regenerate, regenerate-colliding or merge",
                    options.global_ids
                ))
            })?;
            let mut reuse = ifc::ReuseOptions::none();
            reuse.units = options.reuse_units;
            reuse.contexts = options.reuse_contexts;
            reuse.owner_history = options.reuse_owner_history;
            reuse.materials = options.reuse_materials;
            reuse.styles = options.reuse_styles;
            let facade = ifc::AppendOptions::new()
                .global_ids(global_ids)
                .reuse(reuse);
            let (target_schema, source_schema) =
                (self.declared_schema()?, source.declared_schema()?);
            let roots: Vec<ifc::EntityId> = roots.iter().copied().map(ifc::EntityId).collect();
            let report = if options.dry_run {
                ifc::stage_append(
                    &self.inner,
                    target_schema,
                    &source.inner,
                    source_schema,
                    &roots,
                    &facade,
                    seed,
                )
                .map_err(super::convert::error)?
                .report
            } else {
                ifc::apply_append(
                    &mut self.inner,
                    target_schema,
                    &source.inner,
                    source_schema,
                    &roots,
                    &facade,
                    seed,
                )
                .map_err(super::convert::error)?
                .report
            };
            Ok(convert(report, !options.dry_run))
        }
        #[cfg(not(feature = "author"))]
        {
            let _ = (source, roots, options, seed);
            Err(BindingError::FeatureDisabled("author"))
        }
    }
}

#[cfg(feature = "author")]
fn convert(report: ifc::CopyReport, applied: bool) -> CopyReport {
    let entities = |list: Vec<ifc::CopiedEntity>| {
        list.into_iter()
            .map(|entity| CopiedEntity {
                original: entity.original.0,
                copy: entity.copy.0,
                type_name: entity.type_name,
            })
            .collect()
    };
    let ids = |list: Vec<ifc::EntityId>| list.into_iter().map(|id| id.0).collect();
    CopyReport {
        copy: None,
        copied: entities(report.copied),
        reused: entities(report.reused),
        joined: ids(report.joined),
        skipped: ids(report.skipped),
        regenerated: ids(report.regenerated),
        applied,
    }
}
