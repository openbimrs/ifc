mod support;

use ifc_model::{Transaction, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};
use ifc_structural::{
    stage_analysis_model, AnalysisModelDraft, AnalysisModelType, StructuralError, StructuralView,
};

use support::{model, named, GUID};

fn schemas() -> [(&'static str, &'static Schema); 3] {
    [
        ("IFC2X3", ifc2x3()),
        ("IFC4", ifc4()),
        ("IFC4X3_ADD2", ifc4x3()),
    ]
}

#[test]
fn analysis_model_set_projections_reject_duplicate_members() {
    for (token, schema) in schemas() {
        let mut model = model(token);
        let load_group = model.push(named(schema, "IfcStructuralLoadGroup", &[]));
        let result_group = model.push(named(schema, "IfcStructuralResultGroup", &[]));
        let analysis = model.push(named(
            schema,
            "IfcStructuralAnalysisModel",
            &[
                ("LoadedBy", Value::List(vec![Value::Ref(load_group); 2])),
                ("HasResults", Value::List(vec![Value::Ref(result_group); 2])),
            ],
        ));
        let view = StructuralView::new(&model, schema)
            .analysis_model(analysis)
            .unwrap();
        assert!(matches!(
            view.loaded_by(),
            Err(StructuralError::InvalidValue {
                attribute: "LoadedBy",
                ..
            })
        ));
        assert!(matches!(
            view.result_groups(),
            Err(StructuralError::InvalidValue {
                attribute: "HasResults",
                ..
            })
        ));
    }
}

#[test]
fn related_objects_set_rejects_duplicate_members() {
    for (token, schema) in schemas() {
        let mut model = model(token);
        let group = model.push(named(schema, "IfcStructuralAnalysisModel", &[]));
        let item = model.push(named(schema, "IfcStructuralCurveMember", &[]));
        model.push(named(
            schema,
            "IfcRelAssignsToGroup",
            &[
                ("RelatedObjects", Value::List(vec![Value::Ref(item); 2])),
                ("RelatingGroup", Value::Ref(group)),
            ],
        ));
        assert!(matches!(
            StructuralView::new(&model, schema).analysis_items(group),
            Err(StructuralError::InvalidValue {
                attribute: "RelatedObjects",
                ..
            })
        ));
    }
}

#[test]
fn analysis_model_authoring_rejects_duplicate_set_members_atomically() {
    for (token, schema) in schemas() {
        let mut model = model(token);
        let owner = model.push(named(schema, "IfcOwnerHistory", &[]));
        let load_group = model.push(named(schema, "IfcStructuralLoadGroup", &[]));
        let result_group = model.push(named(schema, "IfcStructuralResultGroup", &[]));
        for (loaded_by, result_groups, attribute) in [
            (vec![load_group; 2], vec![], "LoadedBy"),
            (vec![], vec![result_group; 2], "HasResults"),
        ] {
            let mut tx = Transaction::new(&model);
            let result = stage_analysis_model(
                &mut tx,
                &model,
                schema,
                AnalysisModelDraft {
                    global_id: GUID.into(),
                    owner_history: (token == "IFC2X3").then_some(owner),
                    predefined_type: AnalysisModelType::Loading3d,
                    loaded_by,
                    result_groups,
                    ..AnalysisModelDraft::default()
                },
            );
            assert!(matches!(
                result,
                Err(StructuralError::InvalidDraftValue { attribute: found, .. }) if found == attribute
            ));
            assert!(tx.is_empty());
        }
    }
}
