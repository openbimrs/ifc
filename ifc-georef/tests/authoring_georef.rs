//! Authored georeferencing entities, resolved by this crate's readers.
//!
//! The subcontext case matters most: four of its attributes are DERIVE,
//! so the authored record must carry `*` rather than `$`.

use ifc_georef::{
    create_direction, create_projected_crs, create_representation_context,
    create_representation_subcontext, resolve_project_to_map, ProjectedCrsDraft,
};
use ifc_model::codec::Codec;
use ifc_model::{Entity, Model, Transaction, Value};
use ifc_step::StepCodec;

/// An authored CRS resolves through the map-conversion reader.
#[test]
fn an_authored_crs_resolves_through_map_conversion() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let metre = tx.create(Entity::new(
        "IFCSIUNIT",
        vec![
            Value::Derived,
            Value::Enum("LENGTHUNIT".into()),
            Value::Null,
            Value::Enum("METRE".into()),
        ],
    ));
    let north = create_direction(&mut tx, &[0.0, 1.0]).expect("north");
    let origin = tx.create(Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Null; 3]));
    let context =
        create_representation_context(&mut tx, Some("Model"), 3, Some(1e-5), origin, Some(north))
            .expect("context");
    let crs = create_projected_crs(
        &mut tx,
        ProjectedCrsDraft {
            name: "EPSG:25832",
            map_projection: Some("UTM"),
            map_zone: Some("32N"),
            map_unit: Some(metre),
            ..ProjectedCrsDraft::default()
        },
    )
    .expect("crs");
    let conversion = tx.create(Entity::new(
        "IFCMAPCONVERSION",
        vec![
            Value::Ref(context),
            Value::Ref(crs),
            Value::Real(1000.0),
            Value::Real(2000.0),
            Value::Real(50.0),
            Value::Real(1.0),
            Value::Real(0.0),
            Value::Real(1.0),
        ],
    ));
    tx.commit(&mut model).expect("commit");

    let resolved = resolve_project_to_map(&model, conversion, 1.0).expect("resolves");
    assert_eq!(resolved.target_crs.name, "EPSG:25832");
    assert_eq!(resolved.target_crs.map_projection.as_deref(), Some("UTM"));
}

/// A subcontext writes its inherited geometry attributes as `*`.
///
/// The schema derives CoordinateSpaceDimension, Precision,
/// WorldCoordinateSystem and TrueNorth from the parent. `$` would
/// claim they are absent, which discards the parent's precision.
/// Both decode to a Rust value, so this asserts the emitted text.
#[test]
fn a_subcontext_writes_derived_attributes_as_asterisks() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = tx.create(Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Null; 3]));
    let parent = create_representation_context(&mut tx, Some("Model"), 3, Some(1e-5), origin, None)
        .expect("parent");
    create_representation_subcontext(&mut tx, parent, false, Some("Body"), "MODEL_VIEW", None)
        .expect("subcontext");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).expect("written");
    let text = String::from_utf8(bytes).expect("utf8");
    let line = text
        .lines()
        .find(|l| l.contains("IFCGEOMETRICREPRESENTATIONSUBCONTEXT"))
        .expect("the subcontext is written");
    assert_eq!(
        line.matches('*').count(),
        4,
        "four derived attributes, four asterisks: {line}",
    );
}

/// Georeferencing records that parse but locate nothing are refused.
#[test]
fn meaningless_georeferencing_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = tx.create(Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Null; 3]));
    let parent =
        create_representation_context(&mut tx, None, 3, None, origin, None).expect("parent");

    // LIST [2:3]: one ratio is not a direction, four is not either.
    assert!(create_direction(&mut tx, &[1.0]).is_err());
    assert!(create_direction(&mut tx, &[1.0, 0.0, 0.0, 0.0]).is_err());
    // A zero-length direction points nowhere.
    assert!(create_direction(&mut tx, &[0.0, 0.0]).is_err());
    assert!(create_direction(&mut tx, &[f64::NAN, 1.0]).is_err());

    // A CRS the reader cannot match by name.
    assert!(create_projected_crs(
        &mut tx,
        ProjectedCrsDraft {
            name: "   ",
            ..ProjectedCrsDraft::default()
        },
    )
    .is_err(),);

    // Dimension outside 1..=3, and a precision that is not a number.
    assert!(create_representation_context(&mut tx, None, 4, None, origin, None).is_err());
    assert!(create_representation_context(&mut tx, None, 3, Some(f64::NAN), origin, None).is_err(),);

    // ParentNoSub: the derived attributes resolve one level only.
    assert!(
        create_representation_subcontext(&mut tx, parent, true, None, "MODEL_VIEW", None).is_err(),
    );
    // UserTargetProvided: USERDEFINED without a name states nothing.
    assert!(
        create_representation_subcontext(&mut tx, parent, false, None, "USERDEFINED", None)
            .is_err(),
    );
    create_representation_subcontext(
        &mut tx,
        parent,
        false,
        None,
        "USERDEFINED",
        Some("Coordination"),
    )
    .expect("a named custom view is accepted");
}
