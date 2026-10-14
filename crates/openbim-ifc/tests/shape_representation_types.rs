//! The `RepresentationType` a shape representation of given items gets (#481):
//! inferred as the most specific type the release lists, or refused, over the
//! cascade model in IFC2X3, IFC4 and IFC4X3 (see `shape_representation.rs`).

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "validate",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

mod representation_support;

use ifc::{authoring_handle, AuthorOp, AuthoringFailure, SchemaVersion, Value};
use representation_support::*;

#[test]
fn infers_the_most_specific_type_the_release_lists() {
    each_release(|name, model, schema| {
        let ifc2x3 = schema.version() == Some(SchemaVersion::Ifc2x3);
        let infer = |items: Vec<AuthorOp>, of: &[usize]| {
            represent(&model, schema, items, of, None)
                .unwrap_or_else(|failure| panic!("{name}: {failure}"))
        };
        let curve3d = vec![
            point(&[0.0, 0.0, 0.0]),
            point(&[0.0, 0.0, 3.0]),
            polyline(0, 1),
        ];
        assert_eq!(
            infer(curve3d.clone(), &[2]),
            if ifc2x3 {
                "GeometricCurveSet"
            } else {
                "Curve3D"
            },
            "{name}"
        );
        assert_eq!(
            infer(vec![point(&[0.0, 0.0, 0.0]), bounding_box(0)], &[1]),
            "BoundingBox",
            "{name}"
        );
        assert_eq!(infer(clipping(), &[6]), "Clipping", "{name}");
        assert_eq!(infer(clipping(), &[7]), "CSG", "{name}");
        // A Boolean result is no solid model: nothing admits it with a
        // plain extrusion.
        assert!(matches!(
            represent(&model, schema, clipping(), &[3, 6], None),
            Err(AuthoringFailure::NoRepresentationType { .. })
        ));
        assert_eq!(
            infer(vec![point(&[0.0, 0.0, 0.0])], &[0]),
            if ifc2x3 { "GeometricCurveSet" } else { "Point" },
            "{name}"
        );
        if !ifc2x3 {
            let faces = vec![
                create(
                    "IfcCartesianPointList3D",
                    vec![(
                        "CoordList",
                        Value::List(vec![
                            reals(&[0.0, 0.0, 0.0]),
                            reals(&[1.0, 0.0, 0.0]),
                            reals(&[0.0, 1.0, 0.0]),
                        ]),
                    )],
                ),
                create(
                    "IfcTriangulatedFaceSet",
                    vec![
                        ("Coordinates", Value::Ref(authoring_handle(0))),
                        (
                            "CoordIndex",
                            Value::List(vec![Value::List(vec![
                                Value::Integer(1),
                                Value::Integer(2),
                                Value::Integer(3),
                            ])]),
                        ),
                    ],
                ),
            ];
            assert_eq!(infer(faces.clone(), &[1]), "Tessellation", "{name}");
            assert_eq!(infer(faces.clone(), &[0]), "PointCloud", "{name}");
            let mut mixed = faces;
            mixed.extend(extrusion().into_iter().map(|op| shift(op, 2)));
            assert_eq!(infer(mixed, &[1, 5]), "SurfaceOrSolidModel", "{name}");
        }
        // Two boxes are no BoundingBox, and a 3D curve no Curve2D.
        let boxes = vec![point(&[0.0, 0.0, 0.0]), bounding_box(0), bounding_box(0)];
        assert!(
            matches!(
                represent(&model, schema, boxes, &[1, 2], Some("BoundingBox")),
                Err(AuthoringFailure::RepresentationTypeMismatch { ref items, .. }) if items.len() == 2
            ),
            "{name}"
        );
        assert!(
            matches!(
                represent(&model, schema, curve3d, &[2], Some("Curve2D")),
                Err(AuthoringFailure::RepresentationTypeMismatch { .. })
            ),
            "{name}"
        );
    });
}
