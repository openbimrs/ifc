mod support;

use ifc_model::Value;
use ifc_schema::ifc4;
use ifc_structural::{StructuralError, StructuralView};

use support::{enumeration, model, named, text, GUID};

#[test]
fn activity_query_rejects_non_select_target_without_relations() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let invalid_target = model.push(named(
        schema,
        "IfcStructuralAnalysisModel",
        &[
            ("GlobalId", text(GUID)),
            ("PredefinedType", enumeration("LOADING_3D")),
        ],
    ));

    assert!(matches!(
        StructuralView::new(&model, schema).activities_for(invalid_target),
        Err(StructuralError::WrongType { .. })
    ));
}

#[test]
fn structural_activity_has_at_most_one_attachment() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let load = model.push(named(schema, "IfcStructuralLoadSingleForce", &[]));
    let activity = model.push(named(
        schema,
        "IfcStructuralPointAction",
        &[
            ("GlobalId", text(GUID)),
            ("AppliedLoad", Value::Ref(load)),
            ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
        ],
    ));
    let first_target = model.push(named(
        schema,
        "IfcStructuralPointConnection",
        &[("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH"))],
    ));
    let second_target = model.push(named(
        schema,
        "IfcStructuralPointConnection",
        &[("GlobalId", text("2O2Fr$t4X7Zf8NOew3FLOH"))],
    ));
    for (global_id, target) in [
        ("0O2Fr$t4X7Zf8NOew3FLOH", first_target),
        ("3O2Fr$t4X7Zf8NOew3FLOH", second_target),
    ] {
        model.push(named(
            schema,
            "IfcRelConnectsStructuralActivity",
            &[
                ("GlobalId", text(global_id)),
                ("RelatingElement", Value::Ref(target)),
                ("RelatedStructuralActivity", Value::Ref(activity)),
            ],
        ));
    }

    assert!(matches!(
        StructuralView::new(&model, schema).activities_for(first_target),
        Err(StructuralError::SemanticViolation { .. })
    ));
}
