//! Mapping a type's `RepresentationMaps` onto its objects.

use super::*;

// --- representations ---

/// The one mapped representation of `object`, checked against `map`.
pub(super) fn assert_mapped(f: &Fixture, object: EntityId, map: EntityId) -> EntityId {
    let shape = f.reference(object, "Representation");
    assert_eq!(f.get(shape).type_name.as_ref(), "IFCPRODUCTDEFINITIONSHAPE");
    let representations = f.list(shape, "Representations");
    assert_eq!(representations.len(), 1, "{}", f.name());
    let representation = representations[0];
    assert_eq!(
        f.get(representation).type_name.as_ref(),
        "IFCSHAPEREPRESENTATION"
    );
    assert_eq!(
        f.reference(representation, "ContextOfItems"),
        f.base.context
    );
    assert_eq!(
        f.attr(representation, "RepresentationIdentifier"),
        text("Body")
    );
    assert_eq!(
        f.attr(representation, "RepresentationType"),
        text("MappedRepresentation")
    );
    let items = f.list(representation, "Items");
    assert_eq!(items.len(), 1);
    let item = items[0];
    assert_eq!(f.get(item).type_name.as_ref(), "IFCMAPPEDITEM");
    assert_eq!(f.reference(item, "MappingSource"), map);
    let target = f.reference(item, "MappingTarget");
    assert_eq!(
        f.get(target).type_name.as_ref(),
        "IFCCARTESIANTRANSFORMATIONOPERATOR3D"
    );
    for name in ["Axis1", "Axis2", "Scale", "Axis3"] {
        assert_eq!(f.attr(target, name), Value::Null, "identity: no {name}");
    }
    let origin = f.reference(target, "LocalOrigin");
    assert_eq!(f.attr(origin, "Coordinates"), reals(&[0.0, 0.0, 0.0]));
    representation
}

#[test]
fn wall_and_door_types_map_their_representations() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let door_op = f.product("IfcDoor", vec![("Name", text("Door"))]);
        let door = f.apply(vec![door_op])[0].unwrap();
        let options = AssignTypeOptions::new().map_representations(true);
        let ops = vec![
            f.assign(f.base.wall_type, &[wall], options),
            f.assign(f.base.door_type, &[door], options),
        ];
        f.apply(ops);
        assert_mapped(&f, wall, f.base.wall_map);
        assert_mapped(&f, door, f.base.door_map);
        // The type's own representation is shared, not copied.
        let maps = f.list(f.base.door_type, "RepresentationMaps");
        assert_eq!(maps, vec![f.base.door_map]);
    });
}

#[test]
fn one_assignment_maps_every_object_separately() {
    each_release(|mut f| {
        let first = f.wall(vec![]);
        let second = f.wall(vec![]);
        let options = AssignTypeOptions::new().map_representations(true);
        let op = f.assign(f.base.wall_type, &[first, second], options);
        f.apply(vec![op]);
        let a = assert_mapped(&f, first, f.base.wall_map);
        let b = assert_mapped(&f, second, f.base.wall_map);
        assert_ne!(a, b, "{}: each object owns its representation", f.name());
    });
}

#[test]
fn a_given_mapping_target_is_used_by_handle() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let mut b = Batch::default();
        let point = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.5, 0.0, 0.0]))],
        );
        let target = b.create(
            "IfcCartesianTransformationOperator3D",
            vec![("LocalOrigin", Value::Ref(point))],
        );
        let options = AssignTypeOptions::new()
            .map_representations(true)
            .mapping_target(Some(target));
        b.push(f.assign(f.base.wall_type, &[wall], options));
        let ids = f.apply(b.ops);
        let shape = f.reference(wall, "Representation");
        let representation = f.list(shape, "Representations")[0];
        let item = f.list(representation, "Items")[0];
        assert_eq!(Some(f.reference(item, "MappingTarget")), ids[1]);
    });
}

#[test]
fn a_representation_in_the_way_is_refused_and_another_kept() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let start = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.0, 0.0]))],
        );
        let end = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[5.0, 0.0]))],
        );
        let line = b.create("IfcPolyline", vec![("Points", refs(&[start, end]))]);
        let make = |b: &mut Batch, identifier: &str, kind: &str| {
            b.create(
                "IfcShapeRepresentation",
                vec![
                    ("ContextOfItems", Value::Ref(f.base.context)),
                    ("RepresentationIdentifier", text(identifier)),
                    ("RepresentationType", text(kind)),
                    ("Items", refs(&[line])),
                ],
            )
        };
        let axis = make(&mut b, "Axis", "Curve2D");
        let body = make(&mut b, "Body", "Curve2D");
        let axis_shape = b.create(
            "IfcProductDefinitionShape",
            vec![("Representations", refs(&[axis]))],
        );
        let body_shape = b.create(
            "IfcProductDefinitionShape",
            vec![("Representations", refs(&[body]))],
        );
        let mut with_shape = |b: &mut Batch, shape: EntityId| {
            b.push(f.product(
                "IfcWall",
                vec![
                    ("Name", text("Wall")),
                    ("Representation", Value::Ref(shape)),
                ],
            ))
        };
        let with_axis = with_shape(&mut b, axis_shape);
        let with_body = with_shape(&mut b, body_shape);
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (with_axis, with_body, axis) = (real(with_axis), real(with_body), real(axis));

        let options = AssignTypeOptions::new().map_representations(true);
        let op = f.assign(f.base.wall_type, &[with_body], options);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::RepresentationConflict { object, .. } if object == with_body),
            "{}: {failure:?}",
            f.name()
        );

        let op = f.assign(f.base.wall_type, &[with_axis], options);
        f.apply(vec![op]);
        let shape = f.reference(with_axis, "Representation");
        let representations = f.list(shape, "Representations");
        assert_eq!(representations.len(), 2, "{}", f.name());
        assert_eq!(representations[0], axis, "the object's own stays first");
    });
}

#[test]
fn a_map_of_another_representation_kind_is_refused() {
    // IFC2X3 is the release whose `IfcRepresentationMap` has no rule
    // asking for a shape model, so a topology representation is legal
    // there and must be refused rather than mapped.
    let mut f = fixture(SchemaVersion::Ifc4, "IFC4");
    let mut b = Batch::default();
    let point = b.create(
        "IfcCartesianPoint",
        vec![("Coordinates", reals(&[0.0, 0.0, 0.0]))],
    );
    let vertex = b.create(
        "IfcVertexPoint",
        vec![("VertexGeometry", Value::Ref(point))],
    );
    let topology = b.create(
        "IfcTopologyRepresentation",
        vec![
            ("ContextOfItems", Value::Ref(f.base.context)),
            ("RepresentationIdentifier", text("Reference")),
            ("RepresentationType", text("Vertex")),
            ("Items", refs(&[vertex])),
        ],
    );
    let origin = b.create("IfcAxis2Placement3D", vec![("Location", Value::Ref(point))]);
    let map = b.create(
        "IfcRepresentationMap",
        vec![
            ("MappingOrigin", Value::Ref(origin)),
            ("MappedRepresentation", Value::Ref(topology)),
        ],
    );
    let kind = b.push(AuthorOp::TypeObject {
        type_name: "IfcWallType".into(),
        attributes: named(vec![
            ("Name", text("Topology")),
            ("RepresentationMaps", refs(&[map])),
            ("PredefinedType", enumeration("SOLIDWALL")),
        ]),
        owner_history: Some(f.base.owner),
    });
    let ids = f.apply(b.ops);
    let real = |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
    let (kind, map) = (real(kind), real(map));
    let wall = f.wall(vec![]);
    let op = f.assign(
        kind,
        &[wall],
        AssignTypeOptions::new().map_representations(true),
    );
    let failure = f.refuse(vec![op]);
    assert!(
        matches!(failure, AuthoringFailure::UnsupportedRepresentationMap { map: at, .. } if at == map),
        "{failure:?}"
    );
}
