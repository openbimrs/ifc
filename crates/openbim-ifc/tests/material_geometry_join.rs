//! Facade wiring: material plus geometry-select features together.

#![cfg(all(feature = "material", feature = "geometry-select"))]

use ifc::geometry::{
    DirectionSense as GeometrySense, LayerSetDirection as GeometryDirection,
    MaterialLayerSetUsageGeometry, MaterialProfileGeometry,
    MaterialProfileSetUsageTaperingGeometry,
};
use ifc::material::{DirectionSense, LayerSetDirection, MaterialView};
use ifc::{Entity, EntityId, Model, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

#[test]
fn material_semantics_and_geometry_join_on_the_same_entity_id() {
    let profile_id = EntityId(7);
    let material_profile_id = EntityId(11);
    let mut model = Model::new();
    model.insert(profile_id, Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    model.insert(
        material_profile_id,
        Entity::new(
            "IFCMATERIALPROFILEWITHOFFSETS",
            vec![
                Value::Text("Taper start".into()),
                Value::Text("Semantic and geometric projection".into()),
                Value::Null,
                Value::Ref(profile_id),
                Value::Integer(80),
                Value::Text("STEEL".into()),
                Value::List(vec![Value::Real(-0.01), Value::Real(0.02)]),
            ],
        ),
    );

    let semantic = MaterialView::new(&model)
        .profiles_with_offsets()
        .next()
        .expect("material semantic projection");
    let geometry = MaterialProfileGeometry::new(
        semantic.id(),
        model
            .get(semantic.id())
            .expect("both projections borrow the same model record"),
    )
    .expect("geometry-input projection");

    assert_eq!(semantic.id(), material_profile_id);
    assert_eq!(semantic.profile_id().unwrap(), profile_id);
    assert_eq!(geometry.profile_id().unwrap(), profile_id);
    assert_eq!(semantic.offset_values().unwrap(), [-0.01, 0.02]);
    assert_eq!(geometry.offset_values().unwrap(), Some([-0.01, 0.02]));
    assert_eq!(model.len(), 2, "the join must not duplicate IFC storage");
}

/// One `IfcMaterialLayerSetUsage` read by `ifc-material` (semantics) and by
/// `ifc-geometry` (shape input): both views borrow the same record and must
/// agree on every slot they share.
#[test]
fn layer_set_usage_semantics_and_geometry_join_on_the_same_entity_id() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIAL", vec![text("Brick"), Value::Null, Value::Null]),
    );
    // IfcMaterialLayer: Material, LayerThickness, IsVentilated, Name,
    // Description, Category, Priority.
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALLAYER",
            vec![
                r(1),
                Value::Real(0.24),
                Value::Null,
                text("Core"),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    // IfcMaterialLayerSet: MaterialLayers, LayerSetName, Description.
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCMATERIALLAYERSET",
            vec![Value::List(vec![r(2)]), text("Wall"), Value::Null],
        ),
    );
    // IfcMaterialLayerSetUsage: ForLayerSet, LayerSetDirection,
    // DirectionSense, OffsetFromReferenceLine, ReferenceExtent.
    let usage_id = EntityId(4);
    model.insert(
        usage_id,
        Entity::new(
            "IFCMATERIALLAYERSETUSAGE",
            vec![
                r(3),
                Value::Enum("AXIS2".into()),
                Value::Enum("NEGATIVE".into()),
                Value::Real(-0.12),
                Value::Real(3.0),
            ],
        ),
    );
    let before = model.len();

    let semantic = MaterialView::new(&model)
        .layer_set_usages()
        .next()
        .expect("material semantic projection");
    let geometry = MaterialLayerSetUsageGeometry::new(
        semantic.id(),
        model
            .get(semantic.id())
            .expect("both projections borrow the same model record"),
    )
    .expect("geometry-input projection");

    assert_eq!(semantic.id(), usage_id);
    assert_eq!(semantic.layer_set_id().unwrap(), EntityId(3));
    assert_eq!(geometry.layer_set_id().unwrap(), EntityId(3));

    // The two crates declare their own enums (sibling domains share no
    // types), so agreement is checked value by value.
    assert_eq!(
        semantic.layer_set_direction().unwrap(),
        LayerSetDirection::Axis2
    );
    assert_eq!(
        geometry.layer_set_direction().unwrap(),
        GeometryDirection::Axis2
    );
    assert_eq!(
        semantic.direction_sense().unwrap(),
        DirectionSense::Negative
    );
    assert_eq!(geometry.direction_sense().unwrap(), GeometrySense::Negative);

    // The signed offset survives both projections unchanged.
    assert_eq!(semantic.offset_from_reference_line().unwrap(), -0.12);
    assert_eq!(geometry.offset_from_reference_line().unwrap(), -0.12);
    assert_eq!(semantic.reference_extent().unwrap(), Some(3.0));
    assert_eq!(geometry.reference_extent().unwrap(), Some(3.0));

    assert_eq!(
        model.len(),
        before,
        "the join must not duplicate IFC storage"
    );
}

/// One tapering `IfcMaterialProfileSetUsage` read through both views.
#[test]
fn tapering_profile_set_usage_semantics_and_geometry_join_on_the_same_entity_id() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    model.insert(EntityId(2), Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    // IfcMaterialProfile: Name, Description, Material, Profile, Priority,
    // Category.
    for (id, profile) in [(3, 1), (4, 2)] {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCMATERIALPROFILE",
                vec![
                    text("Section"),
                    Value::Null,
                    Value::Null,
                    r(profile),
                    Value::Null,
                    Value::Null,
                ],
            ),
        );
    }
    // IfcMaterialProfileSet: Name, Description, MaterialProfiles,
    // CompositeProfile.
    for (id, profile) in [(5, 3), (6, 4)] {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCMATERIALPROFILESET",
                vec![
                    text("Beam"),
                    Value::Null,
                    Value::List(vec![r(profile)]),
                    Value::Null,
                ],
            ),
        );
    }
    // IfcMaterialProfileSetUsageTapering: the three inherited slots
    // (ForProfileSet, CardinalPoint, ReferenceExtent), then ForProfileEndSet
    // and CardinalEndPoint.
    let usage_id = EntityId(7);
    model.insert(
        usage_id,
        Entity::new(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            vec![
                r(5),
                Value::Integer(5),
                Value::Real(4.5),
                r(6),
                Value::Integer(8),
            ],
        ),
    );
    let before = model.len();

    let semantic = MaterialView::new(&model)
        .tapering_profile_set_usages()
        .next()
        .expect("material semantic projection");
    let geometry = MaterialProfileSetUsageTaperingGeometry::new(
        semantic.id(),
        model
            .get(semantic.id())
            .expect("both projections borrow the same model record"),
    )
    .expect("geometry-input projection");

    assert_eq!(semantic.id(), usage_id);
    assert_eq!(semantic.profile_set_id().unwrap(), EntityId(5));
    assert_eq!(geometry.profile_set_id().unwrap(), EntityId(5));
    assert_eq!(semantic.end_profile_set_id().unwrap(), EntityId(6));
    assert_eq!(geometry.end_profile_set_id().unwrap(), EntityId(6));

    let start = semantic.cardinal_point().unwrap().expect("authored");
    let start_geometry = geometry.cardinal_point().unwrap().expect("authored");
    assert_eq!(start.get(), 5);
    assert_eq!(start_geometry.get(), start.get());
    assert_eq!(start_geometry.standard(), Some(5));
    let end = semantic.cardinal_end_point().unwrap().expect("authored");
    let end_geometry = geometry.cardinal_end_point().unwrap().expect("authored");
    assert_eq!(end.get(), 8);
    assert_eq!(end_geometry.get(), end.get());
    assert_ne!(
        start.get(),
        end.get(),
        "the taper changes the cardinal point"
    );

    assert_eq!(semantic.reference_extent().unwrap(), Some(4.5));
    assert_eq!(geometry.reference_extent().unwrap(), Some(4.5));

    assert_eq!(
        model.len(),
        before,
        "the join must not duplicate IFC storage"
    );
}
