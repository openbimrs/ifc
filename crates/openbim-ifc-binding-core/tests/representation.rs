//! Shape representation operations (#481) in the tape form every host
//! converts to, and `remove_representation`, over the cascade fixtures.
//! The facade's suite (`openbim-ifc/tests/shape_representation.rs`) checks
//! the operations entity by entity; this one checks the conversion and
//! the refusal codes.

mod support;

use openbim_ifc_binding_core::IfcModel;
use support::code;

fn fixture(name: &str) -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

fn fixtures() -> Vec<IfcModel> {
    let mut out = Vec::new();
    if cfg!(feature = "ifc2x3") {
        out.push(fixture("cascade_ifc2x3.ifc"));
    }
    if cfg!(feature = "ifc4") {
        out.push(fixture("cascade_ifc4.ifc"));
    }
    if cfg!(feature = "ifc4x3") {
        out.push(fixture("cascade_ifc4x3.ifc"));
    }
    assert!(!out.is_empty(), "every build bundles a release");
    out
}

#[cfg(not(feature = "author"))]
#[test]
fn representation_removal_without_the_feature_refuses() {
    for mut model in fixtures() {
        assert_eq!(
            code(model.remove_representation(25, false)),
            "feature-disabled"
        );
    }
}

#[cfg(feature = "author")]
mod enabled {
    use super::*;
    use openbim_ifc_binding_core::value::Tagged;
    use support::{create, h, op, reals, text, token};

    fn ids(values: &[u64]) -> Tagged {
        Tagged::List(values.iter().copied().map(Tagged::Ref).collect())
    }

    /// An extrusion (ops 0..=3), its representation (4) assigned to the
    /// window (5).
    fn assemble(kind: Option<&str>) -> Vec<openbim_ifc_binding_core::AuthorOp> {
        let mut shape = vec![
            ("context", Tagged::Ref(11)),
            ("items", Tagged::List(vec![h(3)])),
        ];
        if let Some(kind) = kind {
            shape.push(("representation_type", text(kind)));
        }
        vec![
            create("IfcCartesianPoint", &[("Coordinates", reals(&[0.6, 0.05]))]),
            create("IfcAxis2Placement2D", &[("Location", h(0))]),
            create(
                "IfcRectangleProfileDef",
                &[
                    ("ProfileType", token("AREA")),
                    ("Position", h(1)),
                    ("XDim", Tagged::Real(1.2)),
                    ("YDim", Tagged::Real(0.1)),
                ],
            ),
            create(
                "IfcExtrudedAreaSolid",
                &[
                    ("SweptArea", h(2)),
                    ("Position", Tagged::Ref(9)),
                    ("ExtrudedDirection", Tagged::Ref(31)),
                    ("Depth", Tagged::Real(1.0)),
                ],
            ),
            op("shape_representation", &shape),
            op(
                "assignRepresentation",
                &[("representation", h(4)), ("to", Tagged::Ref(130))],
            ),
        ]
    }

    #[test]
    fn assembles_assigns_copies_and_unassigns_through_the_tape() {
        for mut model in fixtures() {
            let result = model.author(assemble(None)).expect("authors");
            let representation = result.ids[4].unwrap();
            assert_eq!(
                model
                    .attribute_by_name(representation, "RepresentationType")
                    .unwrap(),
                text("SweptSolid")
            );
            let shape = result.ids[5].unwrap();
            assert_eq!(
                model.attribute_by_name(130, "Representation").unwrap(),
                Tagged::Ref(shape)
            );
            let result = model
                .author(vec![
                    op(
                        "copy_representation",
                        &[("representation", Tagged::Ref(25))],
                    ),
                    op(
                        "unassign_representation",
                        &[
                            ("representation", Tagged::Ref(25)),
                            ("from", Tagged::Ref(20)),
                        ],
                    ),
                    op(
                        "remove_representation",
                        &[("representation", Tagged::Ref(25))],
                    ),
                    op(
                        "assign_representation",
                        &[("representation", h(0)), ("to", Tagged::Ref(20))],
                    ),
                ])
                .expect("swaps Wall A's body for a copy");
            let copy = result.ids[0].unwrap();
            assert!(result.removed.contains(&25));
            let shape = model.attribute_by_name(20, "Representation").unwrap();
            let Tagged::Ref(shape) = shape else {
                panic!("{shape:?}")
            };
            assert_eq!(
                model.attribute_by_name(shape, "Representations").unwrap(),
                ids(&[copy])
            );
        }
    }

    #[test]
    fn refusals_have_their_codes() {
        for mut model in fixtures() {
            assert_eq!(code(model.author(assemble(Some("Brep")))), "invalid-value");
            assert_eq!(
                code(model.author(vec![op(
                    "assign_representation",
                    &[
                        ("representation", Tagged::Ref(25)),
                        ("to", Tagged::Ref(130))
                    ],
                )])),
                "invalid-model"
            );
            assert_eq!(
                code(model.author(vec![op(
                    "unassign_representation",
                    &[
                        ("representation", Tagged::Ref(45)),
                        ("from", Tagged::Ref(20))
                    ],
                )])),
                "invalid-model"
            );
            assert_eq!(
                code(model.remove_representation(114, false)),
                "still-referenced"
            );
            assert_eq!(
                code(model.remove_representation(26, false)),
                "wrong-entity-type"
            );
        }
    }

    #[test]
    fn remove_representation_reports_and_dry_runs() {
        for mut model in fixtures() {
            let before = model.len();
            let report = model.remove_representation(105, true).unwrap();
            assert!(!report.applied);
            assert_eq!(model.len(), before);
            let removed: Vec<u64> = report.removed.iter().map(|e| e.id).collect();
            assert_eq!(removed, [104, 105, 106, 107, 108]);
            let map = report.kept.iter().find(|e| e.id == 112).unwrap();
            assert_eq!(
                (map.reason.as_str(), map.users.as_slice()),
                ("shared", &[111][..])
            );
            let report = model.remove_representation(105, false).unwrap();
            assert!(report.applied);
            assert_eq!(model.len(), before - 5);
            assert_eq!(
                model.attribute_by_name(100, "Representation").unwrap(),
                Tagged::Null
            );
        }
    }
}
