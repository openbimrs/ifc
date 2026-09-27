//! Load groups, load cases, and the varying member forms.
//!
//! `IfcStructuralLoadCase` is a load group whose `PredefinedType`
//! the schema pins to LOAD_CASE, so the subtype is not free to
//! disagree with its own supertype slot.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};
use ifc_structural::{
    stage_action, stage_load_group, stage_member, ActionDraft, ActionDraftKind, CoordinateSystem,
    LoadGroupDraft, LoadGroupKind, MemberDraft, MemberDraftKind, MemberPredefinedType,
    StructuralRootDraft,
};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";
const GUID2: &str = "2kLmN$PqR9$tUvWxYzAbCd";

fn root(global_id: &str) -> StructuralRootDraft {
    StructuralRootDraft {
        global_id: global_id.to_owned(),
        owner_history: None,
        name: None,
        description: None,
        object_type: None,
        object_placement: None,
        representation: None,
    }
}

fn group(kind: LoadGroupKind) -> LoadGroupDraft {
    LoadGroupDraft {
        global_id: GUID.to_owned(),
        owner_history: None,
        name: Some("Dead load".to_owned()),
        description: None,
        object_type: None,
        action_type: "PERMANENT_G",
        action_source: "DEAD_LOAD_G",
        coefficient: None,
        purpose: None,
        kind,
    }
}

/// IsLoadCasePredefinedType is enforced by construction.
///
/// The case form takes no PredefinedType from the caller: the writer
/// pins LOAD_CASE. The rule therefore cannot be violated through this
/// API, and the staged record proves the pinned value.
#[test]
fn a_load_case_pins_its_predefined_type() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let case = stage_load_group(
        &mut tx,
        &model,
        ifc4x3(),
        group(LoadGroupKind::Case {
            self_weight_coefficients: None,
        }),
    )
    .expect("load case");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(case).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSTRUCTURALLOADCASE");
    assert_eq!(
        staged.attributes[5],
        Value::Enum("LOAD_CASE".into()),
        "the inherited slot is pinned, not caller-chosen",
    );
}

/// HasObjectType spans three attributes, not one.
///
/// USERDEFINED in *any* of PredefinedType, ActionType or
/// ActionSource obliges an ObjectType.
#[test]
fn userdefined_in_any_of_three_slots_needs_an_object_type() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    for (predefined, action, source) in [
        ("USERDEFINED", "PERMANENT_G", "DEAD_LOAD_G"),
        ("LOAD_GROUP", "USERDEFINED", "DEAD_LOAD_G"),
        ("LOAD_GROUP", "PERMANENT_G", "USERDEFINED"),
    ] {
        let mut draft = group(LoadGroupKind::Group {
            predefined_type: predefined,
        });
        draft.action_type = action;
        draft.action_source = source;
        assert!(
            stage_load_group(&mut tx, &model, ifc4x3(), draft.clone()).is_err(),
            "accepted USERDEFINED without ObjectType: {predefined}/{action}/{source}",
        );

        draft.object_type = Some("Site-specific load".to_owned());
        assert!(
            stage_load_group(&mut tx, &model, ifc4x3(), draft).is_ok(),
            "refused USERDEFINED with ObjectType",
        );
    }
}

/// A blank ObjectType names nothing, so it does not satisfy the rule.
#[test]
fn a_blank_object_type_does_not_satisfy_the_rule() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let mut draft = group(LoadGroupKind::Group {
        predefined_type: "USERDEFINED",
    });
    draft.object_type = Some("   ".to_owned());
    assert!(stage_load_group(&mut tx, &model, ifc4x3(), draft).is_err());
}

/// SelfWeightCoefficients is LIST [3:3]: exactly three, and only
/// on the load case, which is the form that declares it.
#[test]
fn self_weight_coefficients_belong_to_the_case_alone() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let draft = group(LoadGroupKind::Case {
        self_weight_coefficients: Some([0.0, 0.0, -1.0]),
    });
    let case = stage_load_group(&mut tx, &model, ifc4x3(), draft).expect("case");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(case).expect("staged");
    let Value::List(items) = &staged.attributes[10] else {
        panic!("SelfWeightCoefficients is a list");
    };
    assert_eq!(items.len(), 3, "LIST [3:3] is exactly three");
    assert_eq!(items[2], Value::Real(-1.0), "gravity points down");

    // A plain group has no such attribute: offering one is refused,
    // not silently dropped.
    let mut tx = Transaction::new(&model);
    // A plain group does not declare SelfWeightCoefficients at all,
    // so the kind enum gives no way to offer one. The case form is the
    // only carrier, which is the refusal expressed in the type system.
    let draft = group(LoadGroupKind::Group {
        predefined_type: "LOAD_CASE",
    });
    assert!(stage_load_group(&mut tx, &model, ifc4x3(), draft).is_err());
}

/// A non-finite coefficient is refused.
#[test]
fn a_non_finite_coefficient_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let mut draft = group(LoadGroupKind::Group {
        predefined_type: "LOAD_GROUP",
    });
    draft.coefficient = Some(f64::NAN);
    assert!(stage_load_group(&mut tx, &model, ifc4x3(), draft).is_err());

    let draft = group(LoadGroupKind::Case {
        self_weight_coefficients: Some([0.0, f64::INFINITY, -1.0]),
    });
    assert!(stage_load_group(&mut tx, &model, ifc4x3(), draft).is_err());
}

/// The varying member forms share their base's slots and differ
/// only in the type name the analysis side reads.
#[test]
fn the_varying_forms_keep_the_base_slot_layout() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let axis = tx.create(Entity::new("IFCDIRECTION", vec![Value::Null; 1]));

    let varying = stage_member(
        &mut tx,
        &model,
        ifc4x3(),
        MemberDraft {
            root: root(GUID),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::Cable,
                axis: Some(axis),
                varying: true,
            },
        },
    )
    .expect("varying curve member");

    let base = stage_member(
        &mut tx,
        &model,
        ifc4x3(),
        MemberDraft {
            root: root(GUID2),
            kind: MemberDraftKind::Curve {
                predefined_type: MemberPredefinedType::Cable,
                axis: Some(axis),
                varying: false,
            },
        },
    )
    .expect("curve member");
    tx.commit(&mut model).expect("commit");

    let varying = model.get(varying).expect("staged");
    let base = model.get(base).expect("staged");
    assert_eq!(
        varying.type_name.as_ref(),
        "IFCSTRUCTURALCURVEMEMBERVARYING",
    );
    assert_eq!(base.type_name.as_ref(), "IFCSTRUCTURALCURVEMEMBER");
    assert_eq!(
        varying.attributes.len(),
        base.attributes.len(),
        "the varying form adds no attribute",
    );
    assert_eq!(varying.attributes[7], base.attributes[7]);
    assert_eq!(varying.attributes[8], base.attributes[8]);
}

/// SuitablePredefinedType: EQUIDISTANT is a member of the curve
/// activity enum that the curve action alone may not use.
///
/// The token is legal for IfcStructuralCurveActivity, so a writer
/// that only checks enum membership accepts it.
#[test]
fn a_curve_action_refuses_equidistant() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let load = tx.create(Entity::new(
        "IFCSTRUCTURALLOADLINEARFORCE",
        vec![Value::Null; 7],
    ));

    let action = |token: &'static str| ActionDraft {
        root: root(GUID),
        applied_load: load,
        coordinate_system: CoordinateSystem::Global,
        destabilizing_load: Some(false),
        caused_by: None,
        kind: ActionDraftKind::Curve {
            projected_or_true: None,
            predefined_type: token,
        },
    };

    assert!(
        stage_action(&mut tx, &model, ifc4x3(), action("EQUIDISTANT")).is_err(),
        "EQUIDISTANT is barred on the curve action",
    );
    assert!(
        stage_action(&mut tx, &model, ifc4x3(), action("CONST")).is_ok(),
        "CONST is a legal curve activity",
    );
    // A token no schema declares for this attribute.
    assert!(
        stage_action(&mut tx, &model, ifc4x3(), action("ISOCONTOUR")).is_err(),
        "ISOCONTOUR belongs to the surface enum",
    );
    let _ = &mut model;
}

/// The surface varying form writes its own type name.
///
/// The base and varying forms share slot 8 (`Thickness`), so a test
/// that only checks the thickness passes either way. Naming the type
/// is what separates constant thickness from varying.
#[test]
fn the_surface_varying_form_names_itself() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let id = stage_member(
        &mut tx,
        &model,
        ifc4x3(),
        MemberDraft {
            root: root(GUID),
            kind: MemberDraftKind::Surface {
                predefined_type: MemberPredefinedType::Shell,
                thickness: Some(0.2),
                varying: true,
            },
        },
    )
    .expect("varying surface member");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(id).expect("staged").type_name.as_ref(),
        "IFCSTRUCTURALSURFACEMEMBERVARYING",
    );
}

/// An enum token the schema does not declare is refused.
///
/// `validate_enum_token` reads the declared members out of the
/// schema rather than a hand-kept list, so a typo in any of the
/// three attributes is caught instead of written as an enum value
/// no reader can resolve.
#[test]
fn an_undeclared_enum_token_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    for (attribute, draft) in [
        (
            "PredefinedType",
            group(LoadGroupKind::Group {
                predefined_type: "LOAD_GRUOP",
            }),
        ),
        ("ActionType", {
            let mut d = group(LoadGroupKind::Group {
                predefined_type: "LOAD_GROUP",
            });
            d.action_type = "PERMANENT";
            d
        }),
        ("ActionSource", {
            let mut d = group(LoadGroupKind::Group {
                predefined_type: "LOAD_GROUP",
            });
            d.action_source = "DEAD_LOAD";
            d
        }),
    ] {
        assert!(
            stage_load_group(&mut tx, &model, ifc4x3(), draft).is_err(),
            "{attribute} accepted an undeclared token",
        );
    }
}

/// Both schemas declare the load group family identically.
///
/// The three enums differ only in declaration order between IFC4
/// and IFC4X3, not membership, and the slot layouts match. A writer
/// that resolves slots from the schema is therefore correct for both,
/// and this pins that assumption rather than trusting it.
#[test]
fn both_schemas_agree_on_the_load_group_family() {
    for entity in [
        "IfcStructuralLoadGroup",
        "IfcStructuralLoadCase",
        "IfcStructuralCurveMemberVarying",
        "IfcStructuralSurfaceMemberVarying",
    ] {
        let four = ifc4().attribute_names(entity);
        let x3 = ifc4x3().attribute_names(entity);
        assert_eq!(four, x3, "{entity} slot layout drifted between schemas");
    }

    // A load case staged against IFC4 lands in the same slots.
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = stage_load_group(
        &mut tx,
        &model,
        ifc4(),
        group(LoadGroupKind::Case {
            self_weight_coefficients: Some([1.0, 0.0, 0.0]),
        }),
    )
    .expect("load case under IFC4");
    tx.commit(&mut model).expect("commit");
    let staged = model.get(id).expect("staged");
    assert_eq!(staged.attributes[5], Value::Enum("LOAD_CASE".into()));
}
