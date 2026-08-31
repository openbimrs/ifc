mod support;

use ifc_model::Value;
use ifc_schema::ifc2x3;
use ifc_structural::{StructuralError, StructuralView};

use support::{enumeration, model, named};

#[test]
fn linear_and_planar_actions_require_projected_or_true() {
    let schema = ifc2x3();
    let mut model = model("IFC2X3");
    for (action_type, load_type) in [
        ("IfcStructuralLinearAction", "IfcStructuralLoadLinearForce"),
        ("IfcStructuralPlanarAction", "IfcStructuralLoadPlanarForce"),
    ] {
        let load = model.push(named(schema, load_type, &[]));
        let action = model.push(named(
            schema,
            action_type,
            &[
                ("AppliedLoad", Value::Ref(load)),
                ("GlobalOrLocal", enumeration("GLOBAL_COORDS")),
                ("DestabilizingLoad", Value::Bool(false)),
            ],
        ));
        assert!(matches!(
            StructuralView::new(&model, schema)
                .action(action)
                .unwrap()
                .projected_or_true(),
            Err(StructuralError::InvalidValue {
                attribute: "ProjectedOrTrue",
                ..
            })
        ));
    }
}
