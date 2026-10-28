//! Test-only unit assumptions for models that declare none (#585).
//!
//! A committed fixture or hand-built model without a project unit assignment
//! is refused by `units::resolve`. Tests that are about something else say
//! here, in one place, that they read such a model in metres and radians.
#![allow(dead_code)]

use ifc_geometry::{units, GeometryError, UnitScale};
use ifc_model::{Entity, Model, Value};

/// The model's resolved scale; metres and radians when it declares no length
/// unit. Any other refusal fails the test.
pub fn scale_or_si(model: &Model) -> UnitScale {
    match units::resolve(model) {
        Ok(scale) => scale,
        Err(GeometryError::LengthUnitUndeclared { .. }) => UnitScale::SI,
        Err(error) => panic!("units unreadable: {error}"),
    }
}

/// [`declare_metres_and_radians`] when the model declares no length unit;
/// a model that declares one, or whose units are unreadable, is left as it is.
pub fn declare_if_missing(model: &mut Model) {
    if matches!(
        units::resolve(model),
        Err(GeometryError::LengthUnitUndeclared { .. })
    ) {
        declare_metres_and_radians(model);
    }
}

/// Declare metres and radians on a model that has no unit assignment: push
/// the two `IfcSIUnit`s and an `IfcUnitAssignment`, then set
/// `IfcProject.UnitsInContext`, or push a project when there is none.
pub fn declare_metres_and_radians(model: &mut Model) {
    let si = |unit_type: &str, name: &str| {
        Entity::new(
            "IFCSIUNIT",
            vec![
                Value::Derived,
                Value::Enum(unit_type.into()),
                Value::Null,
                Value::Enum(name.into()),
            ],
        )
    };
    let metre = model.push(si("LENGTHUNIT", "METRE"));
    let radian = model.push(si("PLANEANGLEUNIT", "RADIAN"));
    let assignment = model.push(Entity::new(
        "IFCUNITASSIGNMENT",
        vec![Value::List(vec![Value::Ref(metre), Value::Ref(radian)])],
    ));
    match model.ids_of_type("IFCPROJECT").first().copied() {
        Some(project) => {
            let mut entity = model.get(project).expect("project").clone();
            assert!(
                matches!(entity.attributes.get(8), None | Some(Value::Null)),
                "{project} already declares UnitsInContext"
            );
            entity.attributes.resize(9, Value::Null);
            entity.attributes[8] = Value::Ref(assignment);
            model.insert(project, entity);
        }
        None => {
            let mut slots = vec![Value::Null; 9];
            slots[8] = Value::Ref(assignment);
            model.push(Entity::new("IFCPROJECT", slots));
        }
    }
}
