//! Unit tests for IFC boolean views and recursive operand semantics.

use super::*;
use crate::solid::testkit::{e, entity, model, r};
use ifc_model::Value;

fn result(op: &str, first: u64, second: u64) -> Entity {
    entity("IFCBOOLEANRESULT", vec![e(op), r(first), r(second)])
}

#[test]
fn operator_parses_every_express_token_case_insensitively() {
    for (token, expected) in [
        ("UNION", IfcBooleanOperator::Union),
        ("intersection", IfcBooleanOperator::Intersection),
        (".DIFFERENCE.", IfcBooleanOperator::Difference),
    ] {
        assert_eq!(IfcBooleanOperator::parse(token), Some(expected));
    }
    assert_eq!(IfcBooleanOperator::parse("SUBTRACT"), None);
    assert_eq!(IfcBooleanOperator::Difference.to_string(), ".DIFFERENCE.");
    let neutral: axiolid_core::BooleanOperator = IfcBooleanOperator::Difference.into();
    assert_eq!(neutral, axiolid_core::BooleanOperator::Difference);
}

/// An unknown operator must not degrade into a default; a wrong operator
/// produces a solid that looks built and is not.
#[test]
fn an_unknown_operator_token_is_an_error_not_a_default() {
    let e_ = result("SUBTRACT", 2, 3);
    let err = BooleanResult::new(EntityId(1), &e_).operator().unwrap_err();
    assert_eq!(err.entity(), Some(EntityId(1)));
}

/// DIFFERENCE is not commutative, so operand order is load-bearing.
#[test]
fn difference_is_the_only_order_sensitive_operator() {
    assert!(IfcBooleanOperator::Difference.is_order_sensitive());
    assert!(!IfcBooleanOperator::Union.is_order_sensitive());
    assert!(!IfcBooleanOperator::Intersection.is_order_sensitive());
}

#[test]
fn operands_are_returned_in_schema_order() {
    let e_ = result("DIFFERENCE", 100, 200);
    let view = BooleanResult::new(EntityId(1), &e_);
    assert_eq!(view.operands().unwrap(), (EntityId(100), EntityId(200)));
    assert_eq!(view.operator().unwrap(), IfcBooleanOperator::Difference);
}

/// The structure is a tree: an operand may itself be a boolean result, and
/// a consumer that stops at depth one drops every clip but the first.
#[test]
fn a_nested_boolean_operand_is_classified_for_recursion() {
    let m = model(vec![
        (1, result("DIFFERENCE", 2, 5)),
        (2, result("DIFFERENCE", 3, 4)),
        (
            3,
            entity(
                "IFCEXTRUDEDAREASOLID",
                vec![r(90), r(91), r(92), Value::Real(1.0)],
            ),
        ),
        (
            4,
            entity("IFCHALFSPACESOLID", vec![r(93), Value::Bool(true)]),
        ),
        (
            5,
            entity("IFCHALFSPACESOLID", vec![r(94), Value::Bool(true)]),
        ),
    ]);

    let root = BooleanResult::new(EntityId(1), m.get(EntityId(1)).unwrap());
    let (first, second) = root.operands().unwrap();

    assert_eq!(
        root.operand_kind(&m, first).unwrap(),
        Some(OperandKind::BooleanResult)
    );
    assert!(root
        .operand_kind(&m, first)
        .unwrap()
        .unwrap()
        .is_nested_boolean());
    assert_eq!(
        root.operand_kind(&m, second).unwrap(),
        Some(OperandKind::HalfSpace)
    );

    // Descending one level reaches the leaf solid, proving the tree has
    // real depth rather than two leaves.
    let inner = BooleanResult::new(first, m.get(first).unwrap());
    assert_eq!(
        inner
            .operand_kind(&m, inner.first_operand().unwrap())
            .unwrap(),
        Some(OperandKind::SolidModel)
    );
}

/// Walking a chain of clips must reach the bottom, not stop at the first.
#[test]
fn a_deep_clipping_chain_is_walkable_to_its_leaf_solid() {
    let depth = 8usize;
    let mut entities = Vec::new();
    // #1..#8 are clipping results, each cutting the previous; #9 is the
    // leaf solid and #100.. are the half spaces.
    for i in 1..=depth {
        let first = if i == depth { 9 } else { (i + 1) as u64 };
        entities.push((
            i as u64,
            entity(
                "IFCBOOLEANCLIPPINGRESULT",
                vec![e("DIFFERENCE"), r(first), r(100 + i as u64)],
            ),
        ));
        entities.push((
            100 + i as u64,
            entity("IFCHALFSPACESOLID", vec![r(500), Value::Bool(true)]),
        ));
    }
    entities.push((
        9,
        entity(
            "IFCEXTRUDEDAREASOLID",
            vec![r(90), r(91), r(92), Value::Real(1.0)],
        ),
    ));
    let m = model(entities);

    let mut current = EntityId(1);
    let mut clips = 0;
    loop {
        let entity_ref = m.get(current).unwrap();
        match OperandKind::classify(&entity_ref.type_name) {
            Some(OperandKind::BooleanResult) => {
                let view = BooleanResult::new(current, entity_ref);
                assert!(view.is_clipping());
                clips += 1;
                current = view.first_operand().unwrap();
            }
            other => {
                assert_eq!(other, Some(OperandKind::SolidModel));
                break;
            }
        }
    }
    assert_eq!(clips, depth, "every clip in the chain must be visited");
}

#[test]
fn every_operand_select_member_family_is_classified() {
    for (name, expected) in [
        ("IFCBOOLEANCLIPPINGRESULT", OperandKind::BooleanResult),
        ("IFCBLOCK", OperandKind::CsgPrimitive),
        ("IFCPOLYGONALBOUNDEDHALFSPACE", OperandKind::HalfSpace),
        ("IFCFACETEDBREP", OperandKind::SolidModel),
        ("IFCTRIANGULATEDFACESET", OperandKind::TessellatedFaceSet),
    ] {
        assert_eq!(OperandKind::classify(name), Some(expected), "{name}");
    }
    assert_eq!(OperandKind::classify("IFCWALL"), None);
}

#[test]
fn clipping_result_rejects_a_non_difference_operator() {
    let ok = entity(
        "IFCBOOLEANCLIPPINGRESULT",
        vec![e("DIFFERENCE"), r(2), r(3)],
    );
    assert_eq!(
        BooleanClippingResult::new(EntityId(1), &ok)
            .checked_operator()
            .unwrap(),
        IfcBooleanOperator::Difference
    );

    let bad = entity("IFCBOOLEANCLIPPINGRESULT", vec![e("UNION"), r(2), r(3)]);
    let err = BooleanClippingResult::new(EntityId(1), &bad)
        .checked_operator()
        .unwrap_err();
    assert!(err.to_string().contains("DIFFERENCE"));
}

#[test]
fn a_dangling_operand_reference_names_both_entities() {
    let m = model(vec![(1, result("DIFFERENCE", 2, 3))]);
    let view = BooleanResult::new(EntityId(1), m.get(EntityId(1)).unwrap());
    let err = view.operand_kind(&m, EntityId(2)).unwrap_err();
    assert!(err.to_string().contains("#2"));
}
