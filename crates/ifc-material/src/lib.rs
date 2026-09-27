//! Borrowed, codec-independent IFC MaterialResource views.
//!
//! The crate projects typed material identity, constituents, layers, profiles,
//! usage assignments, relationships, and material property sets from
//! [`ifc_model::Model`]. Accessors distinguish absent optional values from
//! malformed values and enforce immediate aggregate shape, required slots, and
//! MaterialResource WHERE constraints. Authored placement and offset values are
//! exposed here; geometric interpretation remains in `ifc-geometry`.
//!
//! Views and authoring bind to the release the model's header declares
//! (IFC2X3 TC1, IFC4 ADD2 TC1 or IFC4X3 ADD2; see [`material_schema`]):
//! every slot position comes from that release's bundled schema table. An
//! attribute the release lacks is [`MaterialError::NotInSchema`], and a record
//! type it lacks is [`MaterialError::EntityNotInSchema`]; neither is read from
//! another release's slot.

pub mod authoring;
pub mod constituent;
pub mod error;
pub mod layer;
pub mod material;
pub mod profile;
mod release;
pub mod types;
pub mod usage;
pub mod view;

pub use authoring::{
    associate_material, associate_material_with_owner_history, create_constituent,
    create_constituent_set, create_layer, create_layer_set, create_layer_set_usage,
    create_layer_with_offsets, create_material, create_material_classification_relationship,
    create_material_definition_representation, create_material_list, create_material_properties,
    create_material_relationship, create_profile, create_profile_set, create_profile_set_usage,
    create_profile_set_usage_tapering, create_profile_with_offsets, ConstituentDraft, LayerDraft,
    LayerSetDraft, MaterialAssignmentDraft, MaterialDraft, ProfileDraft,
};
pub use constituent::{ConstituentFractionDiagnostic, MaterialConstituent, MaterialConstituentSet};
pub use error::{MaterialError, MaterialResult};
/// The IFC release a material read or write binds to (re-exported from
/// `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use layer::{MaterialLayer, MaterialLayerSet, MaterialLayerSetUsage, MaterialLayerWithOffsets};
pub use material::{
    Material, MaterialClassificationRelationship, MaterialList, MaterialProperties,
    MaterialRelationship,
};
pub use profile::{
    MaterialProfile, MaterialProfileSet, MaterialProfileSetUsage, MaterialProfileSetUsageTapering,
    MaterialProfileWithOffsets,
};
pub use release::material_schema;
pub use types::{
    CardinalPointReference, DirectionSense, LayerSetDirection, LogicalValue, MaterialSelect,
    StandardCardinalPoint, IFC4_MATERIAL_RESOURCE_ENTITIES, IFC4_MATERIAL_RESOURCE_TYPES,
};
pub use usage::{
    AssignmentSource, MaterialAssignment, MaterialDefinition, MaterialUsageDefinition,
    ResolvedAssignment, ResolvedMaterialSelect,
};
pub use view::MaterialView;
