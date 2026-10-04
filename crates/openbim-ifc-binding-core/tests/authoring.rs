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

mod support;

use openbim_ifc_binding_core::value::Tagged;
#[cfg(not(feature = "author"))]
use openbim_ifc_binding_core::AuthorOp;
use support::*;

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

#[cfg(feature = "author")]
mod enabled {
    use super::*;
    // `AuthorOp` names the IFC2X3 test's operations only.
    #[cfg(feature = "properties-write")]
    use openbim_ifc_binding_core::property_edit::PropertyEdit;
    #[allow(unused_imports)]
    use openbim_ifc_binding_core::{AuthorOp, IfcModel};

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
}
