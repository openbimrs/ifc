//! Borrowed, codec-independent IFC MaterialResource views.
//!
//! The crate projects typed material identity, constituents, layers, profiles,
//! usage assignments, relationships, and material property sets from
//! [`ifc_model::Model`]. Accessors distinguish absent optional values from
//! malformed values and enforce immediate aggregate shape, required slots, and
//! MaterialResource WHERE constraints. Authored placement and offset values are
//! exposed here; geometric interpretation remains in `ifc-geometry`.

// Documentation debt: this crate does not yet document every public item,
// so it cannot join [workspace.lints] (missing_docs = deny) yet. The
// remaining count is budgeted in scripts/check-missing-docs.py, which fails
// if it grows. Delete this attribute once the count reaches zero and add
// `[lints] workspace = true` to Cargo.toml instead.
#![allow(missing_docs)]

pub mod authoring;
pub mod constituent;
pub mod error;
pub mod layer;
pub mod material;
pub mod profile;
pub mod types;
pub mod usage;
pub mod view;

pub use authoring::{
    associate_material, create_layer, create_layer_set, create_material, LayerDraft, LayerSetDraft,
    MaterialAssignmentDraft, MaterialDraft,
};
pub use constituent::{MaterialConstituent, MaterialConstituentSet};
pub use error::{MaterialError, MaterialResult};
pub use layer::{MaterialLayer, MaterialLayerSet, MaterialLayerSetUsage, MaterialLayerWithOffsets};
pub use material::{
    Material, MaterialClassificationRelationship, MaterialList, MaterialProperties,
    MaterialRelationship,
};
pub use profile::{
    MaterialProfile, MaterialProfileSet, MaterialProfileSetUsage, MaterialProfileSetUsageTapering,
    MaterialProfileWithOffsets,
};
pub use types::{
    CardinalPointReference, DirectionSense, LayerSetDirection, LogicalValue, MaterialSelect,
    StandardCardinalPoint, IFC4_MATERIAL_RESOURCE_ENTITIES, IFC4_MATERIAL_RESOURCE_TYPES,
};
pub use usage::{
    AssignmentSource, MaterialAssignment, MaterialDefinition, MaterialUsageDefinition,
    ResolvedAssignment, ResolvedMaterialSelect,
};
pub use view::MaterialView;
