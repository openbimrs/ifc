//! Containers, contexts and the two tree relationships in the model's
//! declared release, with a caller-supplied `IfcOwnerHistory` (#202).
//!
//! From the EXPRESS sources: IFC2X3 TC1 requires
//! `IfcSpatialStructureElement.CompositionType`, `IfcSpace.
//! InteriorOrExteriorSpace` (which [`SpatialDraft`] cannot carry, so an
//! IFC2X3 space is refused with `AuthoringRequired`), and
//! `IfcProject.RepresentationContexts` and `UnitsInContext`; it declares no
//! `IfcExternalSpatialElement` or `IfcProjectLibrary`.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};

use super::external::{check_external, EXTERNAL};
use super::owned_relationships::{refs, relate_owned};
use super::release::stage;
use super::{
    container, invalid, optional_text, ExternalSpatialDraft, ProjectLibraryDraft,
    SpatialAuthoringResult, SpatialDraft,
};
use crate::relation::slots::{AGGREGATES, CONTAINED_IN};
use crate::tree::SpatialKind;

/// [`create_spatial_element`](super::create_spatial_element) in the model's
/// declared release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
/// requires.
///
/// IFC2X3 also requires `CompositionType`, and an `IfcSpace`'s
/// `InteriorOrExteriorSpace`, which the draft cannot carry: an IFC2X3
/// space is refused.
///
/// # Errors
///
/// Those of [`create_spatial_element`](super::create_spatial_element), a
/// value the release cannot hold, and the release and owner-history
/// refusals of [`aggregate_with_owner_history`]. Nothing is staged on an
/// error.
pub fn create_spatial_element_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: SpatialKind,
    global_id: &str,
    draft: SpatialDraft<'_>,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    let (type_name, _) = container(kind, global_id)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        (
            "ObjectPlacement",
            draft.placement.map_or(Value::Null, Value::Ref),
        ),
        ("LongName", optional_text(draft.long_name)),
        (
            "CompositionType",
            draft
                .composition
                .map_or(Value::Null, |t| Value::Enum(t.into())),
        ),
    ];
    stage(tx, model, type_name, values, Some(owner_history))
}

/// [`create_project`](super::create_project) in the model's declared
/// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
/// requires.
///
/// IFC2X3 also requires `RepresentationContexts`, which this writer does
/// not take: an IFC2X3 project is refused with `AuthoringRequired`.
///
/// # Errors
///
/// Those of [`create_project`](super::create_project), and the release and
/// owner-history refusals of [`aggregate_with_owner_history`]. Nothing is
/// staged on an error.
pub fn create_project_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    units: Option<EntityId>,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCPROJECT";
    if Guid::parse(global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", global_id));
    }
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", optional_text(name)),
        ("UnitsInContext", units.map_or(Value::Null, Value::Ref)),
    ];
    stage(tx, model, ENTITY, values, Some(owner_history))
}

/// [`aggregate`](super::aggregate) in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory`, which IFC2X3 requires.
///
/// The release is bound from `FILE_SCHEMA` (none binds IFC4) and the record
/// laid out by attribute name from its table. In IFC4 and IFC4X3 it is the
/// record of the plain writer with the reference in the optional slot. The
/// owner history is never invented: build it with `ifc-author`.
///
/// # Errors
///
/// Those of [`aggregate`](super::aggregate), and:
/// [`MultipleSchemas`](super::SpatialAuthoringError::MultipleSchemas) or
/// [`UnsupportedSchema`](super::SpatialAuthoringError::UnsupportedSchema)
/// if the model binds no single known release;
/// [`EntityNotInSchema`](super::SpatialAuthoringError::EntityNotInSchema),
/// [`AuthoringNotInSchema`](super::SpatialAuthoringError::AuthoringNotInSchema),
/// [`AuthoringValueType`](super::SpatialAuthoringError::AuthoringValueType)
/// or [`AuthoringRequired`](super::SpatialAuthoringError::AuthoringRequired)
/// for an entity or value the release cannot hold;
/// [`MissingReference`](super::SpatialAuthoringError::MissingReference) if
/// `owner_history` is neither in the model nor staged;
/// [`WrongReferenceType`](super::SpatialAuthoringError::WrongReferenceType)
/// if it is not an `IfcOwnerHistory`. Nothing is staged on an error.
pub fn aggregate_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    let extra = Vec::new();
    relate_owned(
        tx,
        model,
        AGGREGATES,
        global_id,
        parent,
        children,
        extra,
        owner_history,
    )
}

/// [`contain`](super::contain) in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory`, which IFC2X3 requires.
///
/// # Errors
///
/// Those of [`contain`](super::contain), and the release and owner-history
/// refusals of [`aggregate_with_owner_history`]. Nothing is staged on an
/// error.
pub fn contain_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    let extra = Vec::new();
    relate_owned(
        tx,
        model,
        CONTAINED_IN,
        global_id,
        structure,
        elements,
        extra,
        owner_history,
    )
}

/// [`create_external_spatial_element`](super::create_external_spatial_element)
/// in the model's declared release, with a caller-supplied
/// `IfcOwnerHistory`. IFC2X3 declares no `IfcExternalSpatialElement`
/// (`EntityNotInSchema`).
///
/// # Errors
///
/// Those of
/// [`create_external_spatial_element`](super::create_external_spatial_element),
/// and the release and owner-history refusals of
/// [`aggregate_with_owner_history`]. Nothing is staged on an error.
pub fn create_external_spatial_element_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    draft: ExternalSpatialDraft<'_>,
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    check_external(global_id, &draft)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("ObjectType", optional_text(draft.object_type)),
        (
            "ObjectPlacement",
            draft.placement.map_or(Value::Null, Value::Ref),
        ),
        (
            "Representation",
            draft.representation.map_or(Value::Null, Value::Ref),
        ),
        ("LongName", optional_text(draft.long_name)),
        (
            "PredefinedType",
            draft
                .predefined_type
                .map_or(Value::Null, |t| Value::Enum(t.into())),
        ),
    ];
    stage(tx, model, EXTERNAL, values, Some(owner_history))
}

/// [`create_project_library`](super::create_project_library) in the model's
/// declared release, with a caller-supplied `IfcOwnerHistory`. IFC2X3
/// declares no `IfcProjectLibrary` (`EntityNotInSchema`).
///
/// # Errors
///
/// Those of [`create_project_library`](super::create_project_library), and
/// the release and owner-history refusals of
/// [`aggregate_with_owner_history`]. Nothing is staged on an error.
pub fn create_project_library_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    draft: ProjectLibraryDraft<'_>,
    representation_contexts: &[EntityId],
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCPROJECTLIBRARY";
    if Guid::parse(global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", global_id));
    }
    let contexts = if representation_contexts.is_empty() {
        Value::Null
    } else {
        refs(representation_contexts)
    };
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("ObjectType", optional_text(draft.object_type)),
        ("LongName", optional_text(draft.long_name)),
        ("Phase", optional_text(draft.phase)),
        ("RepresentationContexts", contexts),
        (
            "UnitsInContext",
            draft.units.map_or(Value::Null, Value::Ref),
        ),
    ];
    stage(tx, model, ENTITY, values, Some(owner_history))
}
