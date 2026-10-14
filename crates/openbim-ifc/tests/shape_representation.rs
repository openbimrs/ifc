//! Assembling, assigning, unassigning, copying and removing shape
//! representations (#481), over the cascade model in IFC2X3, IFC4 and
//! IFC4X3 (`test/fixtures/synthetic-relations/cascade_*.ifc`, the same ids
//! in each release; see `cascade_delete.rs` for the map):
//!
//! ```text
//! #10, #11        the 'Model' context and its 'Body' sub-context
//! #20 Wall A      body #24 (shape) > #25 (rep) > #26 (extrusion), profile
//!                 #27 > #28 > #32, Position #9, direction #31 (shared
//!                 with Wall B's #46); styled by #33 with #34 (IFC2X3
//!                 through the assignment #300), on layer #80 with #45
//! #100 Door 1     body #104 > #105, a mapped item #106 of the door type
//!                 #111's map #112 > #114 > #115
//! #130 Window 1   no representation; typed by #135, which has no map
//! ```
//!
//! Every model is validated after every batch, and every shape model is
//! checked to have exactly one use (`IfcShapeModel.WR11`, which the
//! validator does not evaluate).

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "validate",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

mod representation_support;

use ifc::{authoring_handle, AuthorOp, AuthoringFailure, KeepReason, SchemaVersion, Value};
use representation_support::*;

#[test]
fn assembles_a_body_and_gives_a_product_its_first_shape() {
    each_release(|name, mut model, schema| {
        let mut ops = extrusion();
        ops.push(shape(11, vec![authoring_handle(3)], None, None));
        ops.push(assign(authoring_handle(4), 130));
        let outcome = apply(name, &mut model, schema, &ops);
        let representation = outcome.ids[4].unwrap().0;
        let product_shape = outcome.ids[5].unwrap().0;
        assert_eq!(
            text(&attr(
                &model,
                schema,
                representation,
                "RepresentationIdentifier"
            )),
            Some("Body"),
            "{name}: the sub-context's identifier"
        );
        assert_eq!(
            text(&attr(&model, schema, representation, "RepresentationType")),
            Some("SweptSolid"),
            "{name}: inferred"
        );
        assert_eq!(refs(&model, schema, 130, "Representation"), [product_shape]);
        assert_eq!(
            refs(&model, schema, product_shape, "Representations"),
            [representation]
        );
    });
}

#[test]
fn appends_an_axis_to_an_existing_product_shape() {
    each_release(|name, mut model, schema| {
        let ops = [
            create(
                "IfcCartesianPoint",
                vec![("Coordinates", reals(&[0.0, 0.0]))],
            ),
            create(
                "IfcCartesianPoint",
                vec![("Coordinates", reals(&[5.0, 0.0]))],
            ),
            create(
                "IfcPolyline",
                vec![(
                    "Points",
                    Value::List(vec![
                        Value::Ref(authoring_handle(0)),
                        Value::Ref(authoring_handle(1)),
                    ]),
                )],
            ),
            shape(10, vec![authoring_handle(2)], Some("Axis"), None),
            assign(authoring_handle(3), 20),
        ];
        let outcome = apply(name, &mut model, schema, &ops);
        let representation = outcome.ids[3].unwrap().0;
        assert_eq!(outcome.ids[4], Some(id(24)), "{name}: the existing shape");
        assert_eq!(
            text(&attr(&model, schema, representation, "RepresentationType")),
            Some("Curve2D"),
            "{name}"
        );
        assert_eq!(
            refs(&model, schema, 24, "Representations"),
            [25, representation]
        );
    });
}

#[test]
fn checks_a_given_type_against_the_items() {
    each_release(|name, mut model, schema| {
        let ifc2x3 = schema.version() == Some(SchemaVersion::Ifc2x3);
        let with = |kind: &str| {
            let mut ops = extrusion();
            ops.push(shape(11, vec![authoring_handle(3)], None, Some(kind)));
            ops
        };
        for wrong in ["Brep", "Curve2D", "MappedRepresentation", "sweptsolid"] {
            match refusal(&mut model, schema, &with(wrong)) {
                AuthoringFailure::RepresentationTypeMismatch {
                    representation_type,
                    items,
                    ..
                } => {
                    assert_eq!(representation_type, wrong, "{name}");
                    assert_eq!(items.len(), 1, "{name}: the solid");
                }
                other => panic!("{name}: {wrong}: {other:?}"),
            }
        }
        // Every type the release lists that admits an extrusion is
        // accepted as given, and so is a label the release does not list.
        let admitted: &[&str] = if ifc2x3 {
            &["SweptSolid", "SolidModel", "UserDefined"]
        } else {
            &[
                "SweptSolid",
                "AdvancedSweptSolid",
                "SolidModel",
                "SurfaceOrSolidModel",
                "UserDefined",
            ]
        };
        for kind in admitted {
            let mut ops = with(kind);
            ops.push(assign(authoring_handle(4), 130));
            let mut copy = model.clone();
            let outcome = apply(name, &mut copy, schema, &ops);
            let representation = outcome.ids[4].unwrap().0;
            assert_eq!(
                text(&attr(&copy, schema, representation, "RepresentationType")),
                Some(*kind),
                "{name}"
            );
        }
    });
}

#[test]
fn refuses_what_no_type_admits_and_what_breaks_the_representation_rules() {
    each_release(|name, mut model, schema| {
        let ifc2x3 = schema.version() == Some(SchemaVersion::Ifc2x3);
        // A solid and a curve: no type of any release admits both.
        let mut ops = extrusion();
        ops.push(create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[1.0, 0.0]))],
        ));
        ops.push(create(
            "IfcPolyline",
            vec![(
                "Points",
                Value::List(vec![
                    Value::Ref(authoring_handle(0)),
                    Value::Ref(authoring_handle(4)),
                ]),
            )],
        ));
        ops.push(shape(
            11,
            vec![authoring_handle(3), authoring_handle(5)],
            None,
            None,
        ));
        assert!(
            matches!(
                refusal(&mut model, schema, &ops),
                AuthoringFailure::NoRepresentationType { ref items } if items.len() == 2
            ),
            "{name}"
        );
        // The 'Model' context carries no identifier: IFC4 on require one.
        let mut ops = extrusion();
        ops.push(shape(10, vec![authoring_handle(3)], None, None));
        ops.push(assign(authoring_handle(4), 130));
        if ifc2x3 {
            let outcome = apply(name, &mut model.clone(), schema, &ops);
            assert!(outcome.ids[4].is_some());
        } else {
            assert_eq!(
                refusal(&mut model, schema, &ops),
                AuthoringFailure::MissingRepresentationIdentifier { context: id(10) },
                "{name}"
            );
        }
        // A placement is no context (CorrectContext).
        let mut ops = extrusion();
        ops.push(shape(9, vec![authoring_handle(3)], Some("Body"), None));
        assert!(
            matches!(
                refusal(&mut model, schema, &ops),
                AuthoringFailure::WrongReferenceType { .. }
            ),
            "{name}"
        );
        // A product definition shape is no item.
        assert!(
            matches!(
                refusal(&mut model, schema, &[shape(11, vec![id(24)], None, None)]),
                AuthoringFailure::WrongReferenceType { .. }
            ),
            "{name}"
        );
    });
}

#[test]
fn a_representation_has_one_use() {
    each_release(|name, mut model, schema| {
        assert_eq!(
            refusal(&mut model, schema, &[assign(id(25), 130)]),
            AuthoringFailure::AlreadyRelated {
                id: id(25),
                relationship: "IFCPRODUCTDEFINITIONSHAPE",
                existing: id(24),
            },
            "{name}"
        );
        assert_eq!(
            refusal(&mut model, schema, &[assign(id(114), 135)]),
            AuthoringFailure::AlreadyRelated {
                id: id(114),
                relationship: "IFCREPRESENTATIONMAP",
                existing: id(112),
            },
            "{name}"
        );
        // Not a shape model; not a product or type.
        assert!(matches!(
            refusal(&mut model, schema, &[assign(id(26), 130)]),
            AuthoringFailure::NotA { .. }
        ));
        let mut ops = extrusion();
        ops.push(shape(11, vec![authoring_handle(3)], None, None));
        ops.push(assign(authoring_handle(4), 61));
        assert!(matches!(
            refusal(&mut model, schema, &ops),
            AuthoringFailure::NotA { .. }
        ));
    });
}

#[test]
fn a_shared_product_shape_is_not_extended() {
    each_release(|name, mut model, schema| {
        let mut ops = vec![AuthorOp::Edit {
            entity: id(40),
            attributes: vec![("Representation".into(), Value::Ref(id(24)))],
        }];
        let offset = ops.len();
        ops.extend(extrusion().into_iter().map(|op| shift(op, offset)));
        ops.push(shape(11, vec![authoring_handle(offset + 3)], None, None));
        ops.push(assign(authoring_handle(offset + 4), 20));
        assert_eq!(
            refusal(&mut model, schema, &ops),
            AuthoringFailure::SharedProductShape {
                shape: id(24),
                products: vec![id(20), id(40)],
            },
            "{name}"
        );
    });
}

#[test]
fn types_carry_representations_as_maps() {
    each_release(|name, mut model, schema| {
        let mut ops = extrusion();
        ops.push(shape(11, vec![authoring_handle(3)], None, None));
        ops.push(assign(authoring_handle(4), 135));
        let outcome = apply(name, &mut model, schema, &ops);
        let representation = outcome.ids[4].unwrap().0;
        let map = outcome.ids[5].unwrap().0;
        assert_eq!(refs(&model, schema, 135, "RepresentationMaps"), [map]);
        assert_eq!(
            refs(&model, schema, map, "MappedRepresentation"),
            [representation]
        );
        let origin = refs(&model, schema, map, "MappingOrigin")[0];
        let point = refs(&model, schema, origin, "Location")[0];
        assert_eq!(
            attr(&model, schema, point, "Coordinates"),
            reals(&[0.0, 0.0, 0.0]),
            "{name}: at the origin"
        );

        // Taking it off removes the map and its origin, and leaves the
        // representation for the batch to place; here it is removed.
        let outcome = apply(
            name,
            &mut model,
            schema,
            &[unassign(representation, 135), remove(id(representation))],
        );
        let unassigned = outcome.reports[0].as_ref().unwrap();
        let gone: Vec<u64> = unassigned.removed.iter().map(|e| e.id.0).collect();
        assert_eq!(gone, [point, origin, map], "{name}: in id order");
        assert_eq!(attr(&model, schema, 135, "RepresentationMaps"), Value::Null);
        assert!(model.get(id(representation)).is_none());

        // A map an occurrence maps cannot go.
        assert_eq!(
            refusal(&mut model, schema, &[unassign(114, 111)]),
            AuthoringFailure::StillReferenced {
                id: id(112),
                by: id(106),
            },
            "{name}"
        );
        assert_eq!(
            refusal(&mut model, schema, &[remove(id(114))]),
            AuthoringFailure::StillReferenced {
                id: id(112),
                by: id(106),
            },
            "{name}"
        );
        assert_eq!(
            refusal(&mut model, schema, &[unassign(25, 135)]),
            AuthoringFailure::NotAssigned {
                representation: id(25),
                to: id(135),
            }
        );
    });
}

#[test]
fn unassigning_the_last_representation_removes_the_product_shape() {
    each_release(|name, mut model, schema| {
        assert_eq!(
            refusal(&mut model, schema, &[unassign(45, 20)]),
            AuthoringFailure::NotAssigned {
                representation: id(45),
                to: id(20),
            },
            "{name}"
        );
        // Unassigned from Wall A and assigned to Window 1 in one batch.
        let outcome = apply(
            name,
            &mut model,
            schema,
            &[unassign(25, 20), assign(id(25), 130)],
        );
        let report = outcome.reports[0].as_ref().unwrap();
        let gone: Vec<u64> = report.removed.iter().map(|e| e.id.0).collect();
        assert_eq!(gone, [24], "{name}: the emptied shape alone");
        assert_eq!(attr(&model, schema, 20, "Representation"), Value::Null);
        let window_shape = outcome.ids[1].unwrap().0;
        assert_eq!(refs(&model, schema, window_shape, "Representations"), [25]);
        assert_eq!(refs(&model, schema, 25, "Items"), [26], "{name}: untouched");
    });
}

#[test]
fn a_copy_is_independent_geometry_sharing_definitions() {
    each_release(|name, mut model, schema| {
        let ifc2x3 = schema.version() == Some(SchemaVersion::Ifc2x3);
        let outcome = apply(
            name,
            &mut model,
            schema,
            &[
                AuthorOp::CopyRepresentation {
                    representation: id(25),
                },
                assign(authoring_handle(0), 130),
            ],
        );
        let copy = outcome.ids[0].unwrap().0;
        assert_eq!(
            refs(&model, schema, copy, "ContextOfItems"),
            [11],
            "{name}: shared"
        );
        let item = refs(&model, schema, copy, "Items")[0];
        assert_ne!(item, 26, "{name}: the item is copied");
        let profile = refs(&model, schema, item, "SweptArea")[0];
        assert_ne!(profile, 27);
        assert_ne!(
            refs(&model, schema, item, "Position"),
            [9],
            "{name}: placements copied"
        );
        assert_ne!(refs(&model, schema, item, "ExtrudedDirection"), [31]);
        // The styled item is copied onto the copy, with the same styles.
        let styled: Vec<u64> = model
            .iter()
            .filter(|(_, entity)| schema.is_a(&entity.type_name, "IfcStyledItem"))
            .filter(|(_, entity)| refs_of(schema, entity, "Item") == [item])
            .map(|(id, _)| id.0)
            .collect();
        assert_eq!(styled.len(), 1, "{name}");
        assert_eq!(
            refs(&model, schema, styled[0], "Styles"),
            refs(&model, schema, 33, "Styles"),
            "{name}: styles shared"
        );
        assert_eq!(
            refs(&model, schema, styled[0], "Styles"),
            [if ifc2x3 { 300 } else { 34 }]
        );
        // The layer holds the copy too.
        assert_eq!(refs(&model, schema, 80, "AssignedItems"), [25, 45, copy]);

        // Editing the copy leaves the original alone.
        apply(
            name,
            &mut model,
            schema,
            &[AuthorOp::Edit {
                entity: id(profile),
                attributes: vec![("XDim".into(), Value::Real(9.0))],
            }],
        );
        assert_eq!(
            attr(&model, schema, 27, "XDim").unwrap_typed().as_f64(),
            Some(5.0)
        );
        assert_eq!(
            attr(&model, schema, profile, "XDim")
                .unwrap_typed()
                .as_f64(),
            Some(9.0)
        );

        // A mapped representation's copy maps the same type geometry.
        let outcome = apply(
            name,
            &mut model,
            schema,
            &[
                AuthorOp::CopyRepresentation {
                    representation: id(105),
                },
                remove(authoring_handle(0)),
            ],
        );
        let report = outcome.reports[1].as_ref().unwrap();
        assert!(
            report
                .kept
                .iter()
                .any(|kept| kept.id == id(112) && kept.users == [id(106), id(111)]),
            "{name}: {:#?}",
            report.kept
        );
    });
}

#[test]
fn removal_keeps_what_others_use() {
    each_release(|name, mut model, schema| {
        let ifc2x3 = schema.version() == Some(SchemaVersion::Ifc2x3);
        let outcome = apply(name, &mut model, schema, &[remove(id(25))]);
        let report = outcome.reports[0].as_ref().unwrap();
        let mut gone: Vec<u64> = report.removed.iter().map(|e| e.id.0).collect();
        gone.sort_unstable();
        let mut expected = vec![24, 25, 26, 27, 28, 32, 33];
        if ifc2x3 {
            expected.push(300);
        }
        assert_eq!(gone, expected, "{name}");
        assert_eq!(attr(&model, schema, 20, "Representation"), Value::Null);
        let reason = |id: u64| {
            report
                .kept
                .iter()
                .find(|kept| kept.id.0 == id)
                .unwrap_or_else(|| panic!("{name}: #{id} not kept: {:#?}", report.kept))
                .reason
        };
        assert_eq!(reason(9), KeepReason::Shared, "{name}: Position");
        assert_eq!(reason(31), KeepReason::Shared, "{name}: direction");
        assert_eq!(reason(11), KeepReason::Definition, "{name}: context");
        assert_eq!(reason(34), KeepReason::Shared, "{name}: style");
        assert_eq!(report.detached, [id(80)], "{name}: the layer");
        assert_eq!(refs(&model, schema, 80, "AssignedItems"), [45]);
        assert!(model.get(id(46)).is_some() && model.get(id(34)).is_some());

        // A mapped body goes with its mapped item, not the type's map.
        let outcome = apply(name, &mut model, schema, &[remove(id(105))]);
        let report = outcome.reports[0].as_ref().unwrap();
        let mut gone: Vec<u64> = report.removed.iter().map(|e| e.id.0).collect();
        gone.sort_unstable();
        assert_eq!(gone, [104, 105, 106, 107, 108], "{name}");
        assert!(model.get(id(112)).is_some() && model.get(id(114)).is_some());
    });
}
