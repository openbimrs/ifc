//! Schema-checked entity creation (#330): a model built from nothing that
//! validates clean and round-trips through STEP and ifcXML, a refused batch
//! that leaves the model byte-identical, and every refusal's stable code.
//!
//! Attribute layouts, required attributes and rules come from the
//! normative schemas (`references/ifc-spec/`): IFC4 ADD2 TC1 requires
//! `IfcWallType.PredefinedType`, derives `IfcSIUnit.Dimensions`, declares
//! `IfcUnitAssignment.Units` a `SET [1:?]` and `IfcObject.IsTypedBy`,
//! `IfcElement.ContainedInStructure` and `IfcObjectDefinition.Decomposes`
//! `SET [0:1]`; IFC2X3 TC1 requires `IfcRoot.OwnerHistory` and
//! `IfcOwnerHistory.ChangeAction`.
#![cfg(feature = "ifc4")]

use openbim_ifc_binding_core::header::Header;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{AuthorOp, BindingError, IfcModel};

/// Without the feature, every operation exists and refuses.
#[cfg(not(feature = "author"))]
#[test]
fn authoring_without_the_feature_refuses() {
    let mut model = empty("IFC4");
    let op = Tagged::List(vec![token("PROJECT")]);
    assert_eq!(code(AuthorOp::from_tagged(&op)), "feature-disabled");
    assert_eq!(code(model.author(vec![])), "feature-disabled");
    assert_eq!(
        code(model.create_entity("IfcWall", vec![])),
        "feature-disabled"
    );
    assert_eq!(code(model.remove_with_relationships(1)), "feature-disabled");
}

fn token(value: &str) -> Tagged {
    Tagged::Enum(value.into())
}

fn empty(schema: &str) -> IfcModel {
    let mut model = IfcModel::empty();
    model.set_header(Header {
        schema: vec![schema.to_owned()],
        ..Header::default()
    });
    model
}

fn code<T: std::fmt::Debug>(result: Result<T, BindingError>) -> &'static str {
    result.expect_err("refused").code()
}

#[cfg(feature = "author")]
mod enabled {
    use super::*;
    use openbim_ifc_binding_core::authoring::{handle, HANDLE_BASE};
    #[cfg(feature = "properties-write")]
    use openbim_ifc_binding_core::property_edit::PropertyEdit;

    fn text(value: &str) -> Tagged {
        Tagged::Text(value.into())
    }

    fn h(index: u64) -> Tagged {
        Tagged::Ref(handle(index).unwrap())
    }

    fn reals(values: &[f64]) -> Tagged {
        Tagged::List(values.iter().copied().map(Tagged::Real).collect())
    }

    fn attributes(pairs: &[(&str, Tagged)]) -> Tagged {
        Tagged::List(
            pairs
                .iter()
                .map(|(name, value)| Tagged::List(vec![text(name), value.clone()]))
                .collect(),
        )
    }

    /// One operation in the tape form every host converts to.
    fn op(name: &str, fields: &[(&str, Tagged)]) -> AuthorOp {
        let mut tape = vec![token(name)];
        for (key, value) in fields {
            tape.push(text(key));
            tape.push(value.clone());
        }
        AuthorOp::from_tagged(&Tagged::List(tape)).expect("well-formed op")
    }

    /// Units, a context, the project, site, building and storey, a wall type
    /// and a placed wall contained in the storey and typed: the host tests'
    /// model. Returns the batch result's ids.
    fn building(model: &mut IfcModel) -> Vec<Option<u64>> {
        let ops = vec![
            // 0..=4: a length unit and the 3D model context.
            op(
                "create",
                &[
                    ("type", text("IfcSIUnit")),
                    (
                        "attributes",
                        attributes(&[("UnitType", token("LENGTHUNIT")), ("Name", token("METRE"))]),
                    ),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcUnitAssignment")),
                    (
                        "attributes",
                        attributes(&[("Units", Tagged::List(vec![h(0)]))]),
                    ),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcCartesianPoint")),
                    (
                        "attributes",
                        attributes(&[("Coordinates", reals(&[0.0, 0.0, 0.0]))]),
                    ),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcAxis2Placement3D")),
                    ("attributes", attributes(&[("Location", h(2))])),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcGeometricRepresentationContext")),
                    (
                        "attributes",
                        attributes(&[
                            ("ContextType", text("Model")),
                            ("CoordinateSpaceDimension", Tagged::Integer(3)),
                            ("Precision", Tagged::Real(1e-5)),
                            ("WorldCoordinateSystem", h(3)),
                        ]),
                    ),
                ],
            ),
            // 5: the project.
            op(
                "project",
                &[(
                    "attributes",
                    attributes(&[
                        ("Name", text("Demo")),
                        ("UnitsInContext", h(1)),
                        ("RepresentationContexts", Tagged::List(vec![h(4)])),
                    ]),
                )],
            ),
            // 6..=11: site, building and storey, each placed in its parent.
            op("placement", &[]),
            op(
                "spatial",
                &[
                    ("type", text("IfcSite")),
                    ("parent", h(5)),
                    ("placement", h(6)),
                    ("attributes", attributes(&[("Name", text("Site"))])),
                ],
            ),
            op("placement", &[("relative_to", h(6))]),
            op(
                "spatial",
                &[
                    ("type", text("IfcBuilding")),
                    ("parent", h(7)),
                    ("placement", h(8)),
                ],
            ),
            op("placement", &[("relative_to", h(8))]),
            op(
                "spatial",
                &[
                    ("type", text("IfcBuildingStorey")),
                    ("parent", h(9)),
                    ("placement", h(10)),
                    (
                        "attributes",
                        attributes(&[("Name", text("Level 0")), ("Elevation", Tagged::Real(0.0))]),
                    ),
                ],
            ),
            // 12: the wall type.
            op(
                "type_object",
                &[
                    ("type", text("IfcWallType")),
                    (
                        "attributes",
                        attributes(&[
                            ("Name", text("Basic 200")),
                            ("PredefinedType", token("STANDARD")),
                        ]),
                    ),
                ],
            ),
            // 13, 14: the wall, placed in the storey, contained and typed.
            op(
                "placement",
                &[
                    ("relative_to", h(10)),
                    ("location", reals(&[1.0, 2.0, 0.0])),
                    ("axis", reals(&[0.0, 0.0, 1.0])),
                    ("ref_direction", reals(&[1.0, 0.0, 0.0])),
                ],
            ),
            op(
                "product",
                &[
                    ("type", text("IfcWall")),
                    ("container", h(11)),
                    ("placement", h(13)),
                    ("type_object", h(12)),
                    (
                        "attributes",
                        attributes(&[
                            ("Name", text("Wall")),
                            ("PredefinedType", token("STANDARD")),
                        ]),
                    ),
                ],
            ),
        ];
        model.author(ops).expect("the building authors").ids
    }

    #[cfg(all(feature = "validate", feature = "ifcxml", feature = "properties-write"))]
    #[test]
    fn a_model_built_from_nothing_validates_and_round_trips() {
        let mut model = empty("IFC4");
        let ids = building(&mut model);
        let wall = ids[14].unwrap();
        assert_eq!(model.type_of(wall).unwrap(), "IFCWALL");
        assert_eq!(model.attribute_by_name(wall, "Name").unwrap(), text("Wall"));
        // A property set through the #316 API, on the new wall.
        model
            .set_properties(vec![PropertyEdit::set(
                wall,
                "ACME_WallData",
                "Mark",
                Tagged::Typed {
                    type_name: "IFCLABEL".into(),
                    value: Box::new(text("W-01")),
                },
            )])
            .expect("the property writes");

        let report = model.validate(None).expect("validates");
        assert_eq!(
            report.summary.errors + report.summary.evaluation_errors,
            0,
            "{:#?}",
            report.findings
        );

        // STEP: written, read and written again, unchanged.
        let step = model.write().unwrap();
        let again = IfcModel::parse(&step).unwrap();
        assert_eq!(again.write().unwrap(), step);
        // ifcXML, native and XSD layouts: read back to the same STEP file.
        for profile in [None, Some("IFC4")] {
            let xml = model.write_ifcxml(profile).unwrap();
            let back = IfcModel::parse_ifcxml(&xml, profile).unwrap();
            assert_eq!(back.write().unwrap(), step, "{profile:?}");
        }
    }

    #[test]
    fn every_root_gets_a_valid_unique_global_id() {
        let mut model = empty("IFC4");
        building(&mut model);
        let mut seen = std::collections::HashSet::new();
        for id in model.ids_of_type_including_subtypes("IfcRoot").unwrap() {
            let Tagged::Text(global_id) = model.attribute_by_name(id, "GlobalId").unwrap() else {
                panic!("#{id} has no GlobalId");
            };
            assert_eq!(global_id.len(), 22);
            assert!(seen.insert(global_id), "#{id} repeats a GlobalId");
        }
        // Project, site, building, storey, wall type and wall, three
        // aggregations, the containment and the typing.
        assert_eq!(seen.len(), 11);
    }

    #[test]
    fn a_fixed_seed_reproduces_the_file() {
        let write = |seed| {
            let mut model = empty("IFC4");
            let ops = vec![op(
                "project",
                &[("attributes", attributes(&[("Name", text("P"))]))],
            )];
            model.author_seeded(ops, seed).unwrap();
            model.write().unwrap()
        };
        assert_eq!(write(7), write(7));
        assert_ne!(write(7), write(8));
    }

    #[test]
    fn a_refused_batch_leaves_the_model_byte_identical() {
        let mut model = empty("IFC4");
        let ids = building(&mut model);
        let before = model.write().unwrap();
        let ops = vec![
            // Fine on its own: another wall in the storey.
            op(
                "product",
                &[
                    ("type", text("IfcWall")),
                    ("container", Tagged::Ref(ids[11].unwrap())),
                ],
            ),
            // Refused: the first wall is already contained.
            op(
                "contain",
                &[
                    ("structure", Tagged::Ref(ids[11].unwrap())),
                    (
                        "elements",
                        Tagged::List(vec![Tagged::Ref(ids[14].unwrap())]),
                    ),
                ],
            ),
        ];
        let refused = model.author(ops).expect_err("refused");
        assert_eq!(refused.code(), "invalid-model");
        assert!(refused.to_string().contains("op 1"), "{refused}");
        assert_eq!(model.write().unwrap(), before);
    }

    #[test]
    fn single_calls_create_and_remove_with_relationships() {
        let mut model = empty("IFC4");
        let ids = building(&mut model);
        let wall = ids[14].unwrap();
        let proxy = model
            .create_entity(
                "IfcBuildingElementProxy",
                vec![("Name".into(), text("Proxy"))],
            )
            .unwrap();
        assert_eq!(model.type_of(proxy).unwrap(), "IFCBUILDINGELEMENTPROXY");

        // The wall is the only element of its containment and of its typing,
        // so both relationships go with it; its placement stays.
        let rels = |model: &IfcModel| {
            (
                model.ids_of_type("IfcRelContainedInSpatialStructure").len(),
                model.ids_of_type("IfcRelDefinesByType").len(),
            )
        };
        assert_eq!(rels(&model), (1, 1));
        model.remove_with_relationships(wall).unwrap();
        assert_eq!(rels(&model), (0, 0));
        assert!(model.dangling_references().is_empty());
        assert_eq!(code(model.type_of(wall)), "missing-entity");
    }

    #[test]
    fn removal_keeps_a_shared_relationship_without_the_entity() {
        let mut model = empty("IFC4");
        let ids = building(&mut model);
        let storey = Tagged::Ref(ids[11].unwrap());
        let result = model
            .author(vec![
                op(
                    "product",
                    &[("type", text("IfcSlab")), ("container", storey.clone())],
                ),
                op("product", &[("type", text("IfcBeam"))]),
                op(
                    "contain",
                    &[
                        ("structure", storey),
                        ("elements", Tagged::List(vec![h(1)])),
                    ],
                ),
            ])
            .unwrap();
        let beam = result.ids[1].unwrap();
        let rel = result.ids[2].unwrap();
        // A relationship with two related objects loses only the removed one.
        model
            .author(vec![op(
                "aggregate",
                &[
                    ("parent", Tagged::Ref(beam)),
                    (
                        "parts",
                        Tagged::List(vec![Tagged::Ref(result.ids[0].unwrap())]),
                    ),
                ],
            )])
            .unwrap();
        model.remove_with_relationships(beam).unwrap();
        assert_eq!(code(model.type_of(rel)), "missing-entity");
        assert!(model.dangling_references().is_empty());
    }

    #[cfg(all(feature = "ifc2x3", feature = "validate"))]
    #[test]
    fn ifc2x3_needs_and_takes_an_owner_history() {
        let mut model = empty("IFC2X3");
        // IFC2X3 requires `IfcRoot.OwnerHistory`, and `IfcProject`'s
        // `RepresentationContexts` and `UnitsInContext`.
        let project = |owner: Option<Tagged>| {
            let mut fields = vec![(
                "attributes",
                attributes(&[
                    ("Name", text("P")),
                    ("UnitsInContext", h(2)),
                    ("RepresentationContexts", Tagged::List(vec![h(5)])),
                ]),
            )];
            if let Some(owner) = owner {
                fields.push(("owner_history", owner));
            }
            op("project", &fields)
        };
        // ... and `IfcOwnerHistory.ChangeAction`.
        let history = |action: Option<&str>| {
            let mut fields = vec![
                ("family_name", text("Doe")),
                ("organization", text("ACME")),
                ("application_name", text("Tool")),
                ("application_version", text("1.0")),
                ("application_identifier", text("tool")),
                ("creation_date", Tagged::Integer(1_700_000_000)),
            ];
            if let Some(action) = action {
                fields.push(("change_action", text(action)));
            }
            op("owner_history", &fields)
        };
        let context = || {
            vec![
                create(
                    "IfcSIUnit",
                    &[("UnitType", token("LENGTHUNIT")), ("Name", token("METRE"))],
                ),
                create("IfcUnitAssignment", &[("Units", Tagged::List(vec![h(1)]))]),
                create(
                    "IfcCartesianPoint",
                    &[("Coordinates", reals(&[0.0, 0.0, 0.0]))],
                ),
                create("IfcAxis2Placement3D", &[("Location", h(3))]),
                create(
                    "IfcGeometricRepresentationContext",
                    &[
                        ("ContextType", text("Model")),
                        ("CoordinateSpaceDimension", Tagged::Integer(3)),
                        ("WorldCoordinateSystem", h(4)),
                    ],
                ),
            ]
        };
        let batch = |history: AuthorOp, project: AuthorOp| {
            let mut ops = vec![history];
            ops.extend(context());
            ops.push(project);
            ops
        };
        assert_eq!(
            code(model.author(batch(history(None), project(Some(h(0)))))),
            "missing-attribute"
        );
        assert_eq!(
            code(model.author(batch(history(Some("NOCHANGE")), project(None)))),
            "missing-attribute"
        );
        let mut ops = batch(history(Some("NOCHANGE")), project(Some(h(0))));
        ops.push(op(
            "spatial",
            &[
                ("type", text("IfcSite")),
                ("parent", h(6)),
                ("owner_history", h(0)),
                (
                    "attributes",
                    attributes(&[("CompositionType", token("ELEMENT"))]),
                ),
            ],
        ));
        let result = model.author(ops).unwrap();
        let site = result.ids[7].unwrap();
        assert_eq!(
            model.attribute_by_name(site, "OwnerHistory").unwrap(),
            Tagged::Ref(result.ids[0].unwrap())
        );
        let report = model.validate(None).unwrap();
        assert_eq!(report.summary.errors, 0, "{:#?}", report.findings);
    }

    #[cfg(feature = "ifc4x3")]
    #[test]
    fn ifc4x3_lays_the_placement_out_by_name() {
        let mut model = empty("IFC4X3_ADD2");
        let result = model
            .author(vec![
                op("placement", &[]),
                op("placement", &[("relative_to", h(0))]),
            ])
            .unwrap();
        let inner = result.ids[1].unwrap();
        assert_eq!(
            model.attribute_by_name(inner, "PlacementRelTo").unwrap(),
            Tagged::Ref(result.ids[0].unwrap())
        );
    }

    /// One refused single-op batch on a fresh copy of the building.
    fn refused(ops: impl FnOnce(&[Option<u64>]) -> Vec<AuthorOp>) -> &'static str {
        let mut model = empty("IFC4");
        let ids = building(&mut model);
        let before = model.write().unwrap();
        let ops = ops(&ids);
        let result = match model.author(ops.clone()) {
            Ok(done) => panic!("accepted: {ops:?} -> {done:?}"),
            Err(error) => error.code(),
        };
        assert_eq!(model.write().unwrap(), before, "{result}");
        result
    }

    fn id(ids: &[Option<u64>], index: usize) -> Tagged {
        Tagged::Ref(ids[index].unwrap())
    }

    fn create(type_name: &str, pairs: &[(&str, Tagged)]) -> AuthorOp {
        op(
            "create",
            &[("type", text(type_name)), ("attributes", attributes(pairs))],
        )
    }

    #[test]
    fn every_refusal_has_its_code() {
        // The declared release does not declare the type.
        assert_eq!(
            refused(|_| vec![create("IfcWal", &[])]),
            "unsupported-schema"
        );
        // Abstract.
        assert_eq!(
            refused(|_| vec![create("IfcElement", &[])]),
            "wrong-entity-type"
        );
        // Names resolve.
        assert_eq!(
            refused(|_| vec![create("IfcWall", &[("Nmae", text("x"))])]),
            "unknown-attribute"
        );
        // Derived attributes are not settable.
        assert_eq!(
            refused(|_| vec![create(
                "IfcSIUnit",
                &[
                    ("Dimensions", Tagged::Null),
                    ("UnitType", token("LENGTHUNIT")),
                    ("Name", token("METRE")),
                ]
            )]),
            "derived-attribute",
            "a derived slot is left unset, and written `*`"
        );
        assert_eq!(
            refused(|ids| vec![create(
                "IfcSIUnit",
                &[
                    ("Dimensions", id(ids, 0)),
                    ("UnitType", token("LENGTHUNIT")),
                    ("Name", token("METRE")),
                ]
            )]),
            "derived-attribute"
        );
        // Required attributes present.
        assert_eq!(
            refused(|_| vec![create("IfcWallType", &[])]),
            "missing-attribute"
        );
        // Value types.
        assert_eq!(
            refused(|_| vec![create("IfcWall", &[("Name", Tagged::Integer(3))])]),
            "invalid-value"
        );
        // An attribute set twice.
        assert_eq!(
            refused(|_| vec![create(
                "IfcWall",
                &[("Name", text("a")), ("name", text("b"))]
            )]),
            "invalid-value"
        );
        // Cardinality: `Units` is `SET [1:?]`.
        assert_eq!(
            refused(|_| vec![create(
                "IfcUnitAssignment",
                &[("Units", Tagged::List(vec![]))]
            )]),
            "invalid-value"
        );
        // A malformed or duplicate GlobalId.
        assert_eq!(
            refused(|_| vec![create("IfcWall", &[("GlobalId", text("nope"))])]),
            "invalid-value"
        );
        assert_eq!(
            refused(|ids| {
                let _ = ids;
                vec![
                    create("IfcWall", &[("GlobalId", text("2YvctVUKr0kugbFTf53O9L"))]),
                    create("IfcWall", &[("GlobalId", text("2YvctVUKr0kugbFTf53O9L"))]),
                ]
            }),
            "invalid-value"
        );
        // References resolve, to an accepted type.
        assert_eq!(
            refused(|_| vec![create(
                "IfcWall",
                &[("ObjectPlacement", Tagged::Ref(99_999))]
            )]),
            "missing-reference"
        );
        assert_eq!(
            refused(|ids| vec![create("IfcWall", &[("ObjectPlacement", id(ids, 0))])]),
            "wrong-entity-type"
        );
        // A builder's type of the wrong kind.
        assert_eq!(
            refused(|ids| vec![op(
                "product",
                &[
                    ("type", text("IfcBuildingStorey")),
                    ("container", id(ids, 11))
                ]
            )]),
            "wrong-entity-type"
        );
        assert_eq!(
            refused(|ids| vec![op(
                "spatial",
                &[("type", text("IfcWall")), ("parent", id(ids, 11))]
            )]),
            "wrong-entity-type"
        );
        assert_eq!(
            refused(|_| vec![op("type_object", &[("type", text("IfcWall"))])]),
            "wrong-entity-type"
        );
        assert_eq!(
            refused(|ids| vec![op(
                "spatial",
                &[("type", text("IfcSpace")), ("parent", id(ids, 14))]
            )]),
            "wrong-entity-type"
        );
        // Relationships the schema allows once.
        assert_eq!(refused(|_| vec![op("project", &[])]), "invalid-model");
        assert_eq!(
            refused(|ids| vec![op(
                "assign_type",
                &[
                    ("type_object", id(ids, 12)),
                    ("objects", Tagged::List(vec![id(ids, 14)]))
                ]
            )]),
            "invalid-model"
        );
        assert_eq!(
            refused(|ids| vec![op(
                "aggregate",
                &[
                    ("parent", id(ids, 9)),
                    ("parts", Tagged::List(vec![id(ids, 11)]))
                ]
            )]),
            "invalid-model"
        );
        assert_eq!(
            refused(|ids| vec![op(
                "contain",
                &[
                    ("structure", id(ids, 11)),
                    ("elements", Tagged::List(vec![]))
                ]
            )]),
            "invalid-value"
        );
        // Edits and removals of entities that do not exist.
        assert_eq!(
            refused(|_| vec![op(
                "edit",
                &[
                    ("entity", Tagged::Ref(99_999)),
                    ("attributes", attributes(&[]))
                ]
            )]),
            "missing-entity"
        );
        assert_eq!(
            refused(|_| vec![op("remove", &[("entity", Tagged::Ref(99_999))])]),
            "missing-entity"
        );
        // An edit is checked like a creation.
        assert_eq!(
            refused(|ids| vec![op(
                "edit",
                &[
                    ("entity", id(ids, 14)),
                    ("attributes", attributes(&[("Name", Tagged::Real(1.0))]))
                ]
            )]),
            "invalid-value"
        );
        assert_eq!(
            refused(|ids| vec![op(
                "edit",
                &[
                    ("entity", id(ids, 12)),
                    (
                        "attributes",
                        attributes(&[("PredefinedType", Tagged::Null)])
                    )
                ]
            )]),
            "missing-attribute"
        );
        assert_eq!(
            refused(|ids| vec![op(
                "edit",
                &[
                    ("entity", id(ids, 0)),
                    ("attributes", attributes(&[("Dimensions", Tagged::Null)]))
                ]
            )]),
            "derived-attribute"
        );
        // A removal another entity still needs: the storey's placement is
        // what the wall's placement is relative to.
        assert_eq!(
            refused(|ids| vec![op("remove", &[("entity", id(ids, 10))])]),
            "still-referenced"
        );
        // Handles name earlier operations that produced an entity.
        assert_eq!(
            refused(|_| vec![create("IfcWall", &[("ObjectPlacement", h(0))])]),
            "invalid-value"
        );
        assert_eq!(
            refused(|ids| vec![
                op("remove", &[("entity", id(ids, 14))]),
                create(
                    "IfcWall",
                    &[("Name", text("x")), ("Description", text("y"))]
                ),
                op("edit", &[("entity", h(0)), ("attributes", attributes(&[]))]),
            ]),
            "invalid-value"
        );
        // Placements the schema cannot hold.
        assert_eq!(
            refused(|_| vec![op(
                "placement",
                &[
                    ("axis", reals(&[0.0, 0.0, 1.0])),
                    ("ref_direction", reals(&[0.0, 0.0, 2.0]))
                ]
            )]),
            "invalid-value"
        );
        assert_eq!(
            refused(|_| vec![op(
                "placement",
                &[
                    ("axis", reals(&[0.0, 0.0, 0.0])),
                    ("ref_direction", reals(&[1.0, 0.0, 0.0]))
                ]
            )]),
            "invalid-value"
        );
    }

    #[test]
    fn malformed_operations_are_invalid_values() {
        let read = |tape: Vec<Tagged>| code(AuthorOp::from_tagged(&Tagged::List(tape)));
        assert_eq!(read(vec![token("BUILD")]), "invalid-value");
        assert_eq!(read(vec![text("create")]), "invalid-value");
        assert_eq!(read(vec![token("CREATE")]), "invalid-value", "needs a type");
        assert_eq!(
            read(vec![
                token("CREATE"),
                text("type"),
                text("IfcWall"),
                text("colour"),
                text("red")
            ]),
            "invalid-value"
        );
        assert_eq!(
            read(vec![
                token("CREATE"),
                text("type"),
                text("IfcWall"),
                text("type"),
                text("IfcSlab")
            ]),
            "invalid-value"
        );
        assert_eq!(
            read(vec![
                token("PLACEMENT"),
                text("location"),
                reals(&[1.0, 2.0])
            ]),
            "invalid-value"
        );
        assert_eq!(
            read(vec![
                token("PLACEMENT"),
                text("axis"),
                reals(&[0.0, 0.0, 1.0])
            ]),
            "invalid-value",
            "axis without ref_direction"
        );
        assert_eq!(
            read(vec![
                token("CONTAIN"),
                text("structure"),
                Tagged::Integer(-1)
            ]),
            "invalid-value"
        );
        // Both spellings of a name are accepted.
        assert!(AuthorOp::from_tagged(&Tagged::List(vec![
            token("assignType"),
            text("typeObject"),
            Tagged::Ref(1),
            text("objects"),
            Tagged::List(vec![Tagged::Ref(2)]),
        ]))
        .is_ok());
    }

    #[test]
    fn a_header_without_a_bundled_release_is_refused() {
        let mut model = IfcModel::empty();
        assert_eq!(
            code(model.author(vec![op("project", &[])])),
            "unsupported-schema"
        );
        let mut model = empty("IFC9");
        assert_eq!(
            code(model.create_entity("IfcWall", vec![])),
            "unsupported-schema"
        );
    }

    #[test]
    fn handles_live_in_their_own_range() {
        assert_eq!(handle(0).unwrap(), HANDLE_BASE);
        assert_eq!(HANDLE_BASE, 1 << 62);
        assert_eq!(code(handle(HANDLE_BASE)), "out-of-range");
    }
}
