//! Sweep every `ResourceKind` through both writers.
//!
//! `create_resource` and `create_resource_type` map the kind enum to a
//! type name through a match. Tests exercised a subset, so the remaining
//! arms were never proven to stage -- a wrong name in one of them
//! compiles and passes.

use ifc_model::Model;
use ifc_resource::{ResourceDraft, ResourceEditor, ResourceKind};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

/// The six kinds and the two type names each maps to.
const KINDS: &[(ResourceKind, &str, &str)] = &[
    (
        ResourceKind::Labor,
        "IfcLaborResource",
        "IfcLaborResourceType",
    ),
    (
        ResourceKind::Equipment,
        "IfcConstructionEquipmentResource",
        "IfcConstructionEquipmentResourceType",
    ),
    (ResourceKind::Crew, "IfcCrewResource", "IfcCrewResourceType"),
    (
        ResourceKind::Material,
        "IfcConstructionMaterialResource",
        "IfcConstructionMaterialResourceType",
    ),
    (
        ResourceKind::Product,
        "IfcConstructionProductResource",
        "IfcConstructionProductResourceType",
    ),
    (
        ResourceKind::Subcontract,
        "IfcSubContractResource",
        "IfcSubContractResourceType",
    ),
];

fn schema_model() -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    model
}

/// Every occurrence kind stages under its own type name.
#[test]
fn every_resource_kind_stages() {
    for (kind, occurrence, _) in KINDS {
        let mut model = schema_model();
        let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
        let id = editor
            .create_resource(ResourceDraft::new(*kind, GUID).name("Sweep"))
            .unwrap_or_else(|error| panic!("{occurrence} refused: {error:?}"));
        assert_eq!(
            model.get(id).expect("staged").type_name.as_ref(),
            *occurrence,
        );
    }
}

/// Every type kind stages under its own type name.
///
/// The two matches are written separately, so a kind can be right on one
/// side and wrong on the other.
#[test]
fn every_resource_type_kind_stages() {
    for (kind, _, type_object) in KINDS {
        let mut model = schema_model();
        let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
        let id = editor
            .create_resource_type(
                // A resource type must state its PredefinedType; the
                // occurrence side may leave it unset.
                ResourceDraft::new(*kind, GUID)
                    .name("Sweep")
                    .predefined_type("NOTDEFINED"),
            )
            .unwrap_or_else(|error| panic!("{type_object} refused: {error:?}"));
        assert_eq!(
            model.get(id).expect("staged").type_name.as_ref(),
            *type_object,
        );
    }
}
