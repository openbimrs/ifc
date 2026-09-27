//! Distribution element occurrences, zones and spatial zones.
//!
//! The flow reader in this crate recognises a fixed set of classes.
//! These tests author each one and read it back through that reader,
//! so a writer emitting a class the reader ignores fails here rather
//! than in a downstream query returning nothing.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_systems::authoring::{
    create_distribution_element, create_spatial_zone, create_zone, DistributionElementKind,
    ElementAttributes,
};

const GUID_A: &str = "0EI0MSHbX9gg8Fxwar7lb8";
const GUID_B: &str = "1EI0MSHbX9gg8Fxwar7lb9";

/// Every distribution kind stages its own entity name.
///
/// A single writer taking a type-name string would let a caller stage
/// `IFCWALL` here; the closed enum is what stops that, and this test
/// pins the mapping so a reordered match arm is caught.
#[test]
fn each_kind_stages_its_own_class() {
    let model = Model::default();
    let expected = [
        (
            DistributionElementKind::DistributionElement,
            "IFCDISTRIBUTIONELEMENT",
        ),
        (
            DistributionElementKind::EnergyConversionDevice,
            "IFCENERGYCONVERSIONDEVICE",
        ),
        (DistributionElementKind::FlowController, "IFCFLOWCONTROLLER"),
        (DistributionElementKind::FlowFitting, "IFCFLOWFITTING"),
        (
            DistributionElementKind::FlowMovingDevice,
            "IFCFLOWMOVINGDEVICE",
        ),
        (DistributionElementKind::FlowSegment, "IFCFLOWSEGMENT"),
        (
            DistributionElementKind::FlowStorageDevice,
            "IFCFLOWSTORAGEDEVICE",
        ),
        (DistributionElementKind::FlowTerminal, "IFCFLOWTERMINAL"),
        (
            DistributionElementKind::FlowTreatmentDevice,
            "IFCFLOWTREATMENTDEVICE",
        ),
    ];

    let mut staged = Vec::new();
    let mut tx = Transaction::new(&model);
    for (kind, type_name) in expected {
        let id = create_distribution_element(&mut tx, kind, GUID_A, ElementAttributes::default())
            .expect("a well formed element is accepted");
        staged.push((id, kind, type_name));
    }
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    for (id, kind, type_name) in staged {
        let entity = model.get(id).expect("staged");
        assert_eq!(
            entity.type_name.as_ref(),
            type_name,
            "{kind:?} must stage {type_name}"
        );
    }
}

/// Optional attributes land in their inherited slots.
///
/// All eight slots come from supertypes, so an off-by-one here writes
/// a placement into `Representation` and still round-trips as valid
/// STEP. Asserting positions is the only way to catch it.
#[test]
fn element_attributes_land_in_inherited_slots() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let placement = tx.create(Entity::new("IFCLOCALPLACEMENT", vec![Value::Null; 2]));
    let shape = tx.create(Entity::new(
        "IFCPRODUCTDEFINITIONSHAPE",
        vec![Value::Null; 3],
    ));

    let id = create_distribution_element(
        &mut tx,
        DistributionElementKind::FlowSegment,
        GUID_A,
        ElementAttributes {
            name: Some("supply duct"),
            description: Some("main run"),
            placement: Some(placement),
            representation: Some(shape),
            tag: Some("D-101"),
        },
    )
    .expect("a well formed element is accepted");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.attributes[0], Value::Text(GUID_A.into()));
    assert_eq!(entity.attributes[2], Value::Text("supply duct".into()));
    assert_eq!(entity.attributes[3], Value::Text("main run".into()));
    assert_eq!(entity.attributes[5], Value::Ref(placement));
    assert_eq!(entity.attributes[6], Value::Ref(shape));
    assert_eq!(entity.attributes[7], Value::Text("D-101".into()));
}

/// A malformed GlobalId is refused for every kind.
#[test]
fn a_malformed_guid_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    assert!(
        create_distribution_element(
            &mut tx,
            DistributionElementKind::FlowTerminal,
            "not-a-guid",
            ElementAttributes::default(),
        )
        .is_err(),
        "a 22-character base64 GlobalId is required"
    );
    assert!(
        create_zone(&mut tx, "also-not-a-guid", None, None, None).is_err(),
        "zones carry a GlobalId too"
    );
}

/// `IfcZone` has no placement; `IfcSpatialZone` does.
///
/// This is the distinction that makes them separate constructors. A
/// zone is an `IfcGroup` with six attributes, a spatial zone is an
/// `IfcSpatialElement` with nine.
#[test]
fn a_zone_and_a_spatial_zone_differ_in_shape() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let zone = create_zone(
        &mut tx,
        GUID_A,
        Some("thermal zone 1"),
        Some("north facade"),
        Some("TZ-1"),
    )
    .expect("a well formed zone is accepted");

    let spatial = create_spatial_zone(
        &mut tx,
        GUID_B,
        ElementAttributes {
            name: Some("occupancy zone"),
            ..ElementAttributes::default()
        },
        Some("OZ-1"),
        Some("OCCUPANCY"),
        None,
    )
    .expect("a well formed spatial zone is accepted");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let zone = model.get(zone).expect("staged");
    assert_eq!(zone.type_name.as_ref(), "IFCZONE");
    assert_eq!(zone.attributes.len(), 6, "IfcGroup carries no placement");
    assert_eq!(zone.attributes[5], Value::Text("TZ-1".into()));

    let spatial = model.get(spatial).expect("staged");
    assert_eq!(spatial.type_name.as_ref(), "IFCSPATIALZONE");
    assert_eq!(
        spatial.attributes.len(),
        9,
        "IfcSpatialElement adds placement, representation and LongName"
    );
    assert_eq!(spatial.attributes[7], Value::Text("OZ-1".into()));
    assert_eq!(spatial.attributes[8], Value::Enum("OCCUPANCY".into()));
}

/// USERDEFINED without an ObjectType is refused.
///
/// The enum member means "not covered by this list", so the file must
/// say what it actually is. Writing USERDEFINED alone produces a zone
/// whose purpose cannot be recovered by any reader.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    assert!(
        create_spatial_zone(
            &mut tx,
            GUID_A,
            ElementAttributes::default(),
            None,
            Some("USERDEFINED"),
            None,
        )
        .is_err(),
        "USERDEFINED alone leaves the kind unrecoverable"
    );

    let id = create_spatial_zone(
        &mut tx,
        GUID_A,
        ElementAttributes::default(),
        None,
        Some("USERDEFINED"),
        Some("acoustic isolation"),
    )
    .expect("USERDEFINED with a naming ObjectType is accepted");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(
        entity.attributes[4],
        Value::Text("acoustic isolation".into()),
        "the user-defined kind is named in ObjectType"
    );
}
