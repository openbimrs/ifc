//! The coordinate operation's `SourceCRS` and the `HasCoordinateOperation`
//! inverse (#101): valid, wrong-type, dangling, sub-context and
//! more-than-one fixtures, under IFC4 and IFC4X3.

#[path = "support/step.rs"]
mod support;

use ifc_georef::{
    coordinate_operation_for, resolve_operation_source, resolve_project_to_map, GeorefError,
    GeorefView, OperationSource,
};
use ifc_model::{EntityId, Model};
use support::{step, BASE, GEOGRAPHIC};

const OPERATION: EntityId = EntityId(80);
const CONTEXT: EntityId = EntityId(7);

fn conversion(source: &str) -> String {
    format!("#80=IFCMAPCONVERSION({source},#50,1000.,2000.,50.,$,$,$);\n")
}

fn model(schema: &str, extra: &str) -> Model {
    step(schema, &format!("{BASE}{extra}"))
}

#[test]
fn a_context_source_resolves_both_ways_in_ifc4_and_ifc4x3() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(schema, &conversion("#7"));
        let view = GeorefView::for_model(&model).expect(schema);
        assert_eq!(
            resolve_operation_source(&view, OPERATION),
            Ok(OperationSource::Context(CONTEXT)),
            "{schema}"
        );
        assert_eq!(
            coordinate_operation_for(&view, CONTEXT),
            Ok(Some(OPERATION)),
            "{schema}: HasCoordinateOperation"
        );
        let resolved = resolve_project_to_map(&model, OPERATION, 1.0).expect(schema);
        assert_eq!(resolved.source, OperationSource::Context(CONTEXT));
        assert_eq!(resolved.source_crs, CONTEXT);
    }
}

#[test]
fn a_crs_source_is_a_select_member_in_both_releases() {
    // IfcCoordinateReferenceSystemSelect = SELECT (IfcCoordinateReferenceSystem,
    // IfcGeometricRepresentationContext) in IFC4 and IFC4X3 alike.
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(
            schema,
            &format!(
                "#55=IFCPROJECTEDCRS('EPSG:4647',$,$,$,$,$,#1);\n{}",
                conversion("#55")
            ),
        );
        let view = GeorefView::for_model(&model).expect(schema);
        assert_eq!(
            resolve_operation_source(&view, OPERATION),
            Ok(OperationSource::CoordinateReferenceSystem(EntityId(55))),
            "{schema}"
        );
        assert_eq!(
            coordinate_operation_for(&view, EntityId(55)),
            Ok(Some(OPERATION))
        );
    }
}

#[test]
fn a_context_without_an_operation_has_none() {
    let model = model("IFC4X3_ADD2", "");
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    assert_eq!(coordinate_operation_for(&view, CONTEXT), Ok(None));
    assert_eq!(
        coordinate_operation_for(&view, EntityId(8)),
        Ok(None),
        "a sub-context without an operation is fine"
    );
}

#[test]
fn a_source_of_the_wrong_type_is_refused() {
    let model = model("IFC4X3_ADD2", &conversion("#5"));
    let wrong = |result: Result<_, GeorefError>| {
        matches!(
            result,
            Err(GeorefError::WrongType { entity, actual, .. })
                if entity == EntityId(5) && actual == "IFCAXIS2PLACEMENT3D"
        )
    };
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    assert!(wrong(
        resolve_operation_source(&view, OPERATION).map(|_| ())
    ));
    assert!(wrong(
        resolve_project_to_map(&model, OPERATION, 1.0).map(|_| ())
    ));
    assert!(wrong(
        coordinate_operation_for(&view, EntityId(5)).map(|_| ())
    ));

    // Unpinned (no header), the same refusal.
    let mut unpinned = model.clone();
    unpinned.header_mut().schema.clear();
    assert!(wrong(
        resolve_project_to_map(&unpinned, OPERATION, 1.0).map(|_| ())
    ));
}

#[test]
fn a_dangling_or_unset_source_is_refused() {
    let dangling = model("IFC4X3_ADD2", &conversion("#999"));
    assert_eq!(
        resolve_project_to_map(&dangling, OPERATION, 1.0),
        Err(GeorefError::MissingEntity {
            referrer: OPERATION,
            missing: EntityId(999),
        })
    );
    let view = GeorefView::for_model(&dangling).expect("IFC4X3");
    assert_eq!(
        coordinate_operation_for(&view, EntityId(999)),
        Err(GeorefError::MissingEntity {
            referrer: EntityId(999),
            missing: EntityId(999),
        })
    );

    let unset = model("IFC4X3_ADD2", &conversion("$"));
    assert_eq!(
        resolve_project_to_map(&unset, OPERATION, 1.0),
        Err(GeorefError::MissingAttribute {
            entity: OPERATION,
            index: 0,
            name: "SourceCRS",
        })
    );
}

#[test]
fn a_sub_context_may_not_be_a_source() {
    let model = model("IFC4X3_ADD2", &conversion("#8"));
    let rule = GeorefError::RuleViolation {
        entity: EntityId(8),
        rule: "NoCoordOperation",
    };
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    assert_eq!(
        resolve_operation_source(&view, OPERATION),
        Err(rule.clone())
    );
    assert_eq!(
        coordinate_operation_for(&view, EntityId(8)),
        Err(rule.clone())
    );
    assert_eq!(resolve_project_to_map(&model, OPERATION, 1.0), Err(rule));
}

#[test]
fn more_than_one_operation_for_a_source_is_refused() {
    let model = model(
        "IFC4X3_ADD2",
        &format!(
            "{}#81=IFCRIGIDOPERATION(#7,#50,IFCLENGTHMEASURE(1.),IFCLENGTHMEASURE(2.),$);\n",
            conversion("#7")
        ),
    );
    let rule = |result: Result<(), GeorefError>| {
        matches!(
            result,
            Err(GeorefError::RuleViolation { entity, rule })
                if entity == CONTEXT && rule.starts_with("HasCoordinateOperation")
        )
    };
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    assert!(rule(coordinate_operation_for(&view, CONTEXT).map(|_| ())));
    assert!(rule(resolve_operation_source(&view, OPERATION).map(|_| ())));
    for operation in [OPERATION, EntityId(81)] {
        assert!(rule(
            resolve_project_to_map(&model, operation, 1.0).map(|_| ())
        ));
    }
}

#[test]
fn a_geographic_crs_source_is_undeclared_under_ifc4() {
    let ifc4 = model("IFC4", &format!("{GEOGRAPHIC}{}", conversion("#70")));
    let view = GeorefView::for_model(&ifc4).expect("IFC4");
    assert!(matches!(
        resolve_operation_source(&view, OPERATION),
        Err(GeorefError::UnsupportedOperation { entity, actual })
            if entity == EntityId(70) && actual.contains("IFC4_ADD2_TC1")
    ));

    let ifc4x3 = model("IFC4X3_ADD2", &format!("{GEOGRAPHIC}{}", conversion("#70")));
    let view = GeorefView::for_model(&ifc4x3).expect("IFC4X3");
    assert_eq!(
        resolve_operation_source(&view, OPERATION),
        Ok(OperationSource::CoordinateReferenceSystem(EntityId(70)))
    );
}

#[test]
fn only_a_coordinate_operation_has_a_source() {
    let model = model("IFC4X3_ADD2", &conversion("#7"));
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    assert!(matches!(
        resolve_operation_source(&view, EntityId(50)),
        Err(GeorefError::WrongType { entity, expected: "IFCCOORDINATEOPERATION", .. })
            if entity == EntityId(50)
    ));
}
