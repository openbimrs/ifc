//! A local placement: `IfcLocalPlacement` over an `IfcAxis2Placement3D`.
//!
//! Every entity is written by name through the checked create, so the
//! release's own layout holds (IFC4X3 moved `PlacementRelTo` up into
//! `IfcObjectPlacement`). The checks here are the placement's own rules,
//! which the tables do not carry: a 3D `Location` (`LocationIs3D`), axes
//! given both or neither (`AxisAndRefDirProvision`), and a local Z not
//! parallel to the local X (`AxisToRefDirPosition`), and the direction's
//! rule that a zero vector denotes no direction.

use ifc_model::{EntityId, Value};

use super::plan::Planner;
use super::AuthoringFailure as F;

impl Planner<'_, '_> {
    pub(super) fn placement(
        &mut self,
        relative_to: Option<EntityId>,
        location: [f64; 3],
        axes: Option<([f64; 3], [f64; 3])>,
    ) -> Result<EntityId, F> {
        finite("location", &location)?;
        if let Some((axis, ref_direction)) = axes {
            finite("axis", &axis)?;
            finite("ref_direction", &ref_direction)?;
            let length = |v: [f64; 3]| v.iter().map(|c| c * c).sum::<f64>().sqrt();
            if length(axis) == 0.0 || length(ref_direction) == 0.0 {
                return Err(invalid("a zero direction denotes no direction"));
            }
            let cross = [
                axis[1] * ref_direction[2] - axis[2] * ref_direction[1],
                axis[2] * ref_direction[0] - axis[0] * ref_direction[2],
                axis[0] * ref_direction[1] - axis[1] * ref_direction[0],
            ];
            if length(cross) <= 1e-9 * length(axis) * length(ref_direction) {
                return Err(invalid("axis and ref_direction are parallel"));
            }
        }
        let point = self.create(
            "IfcCartesianPoint",
            vec![("Coordinates".to_owned(), reals(&location))],
        )?;
        let mut attributes = vec![("Location".to_owned(), Value::Ref(point))];
        if let Some((axis, ref_direction)) = axes {
            let axis = self.direction(axis)?;
            let ref_direction = self.direction(ref_direction)?;
            attributes.push(("Axis".to_owned(), Value::Ref(axis)));
            attributes.push(("RefDirection".to_owned(), Value::Ref(ref_direction)));
        }
        let relative = self.create("IfcAxis2Placement3D", attributes)?;
        let mut attributes = vec![("RelativePlacement".to_owned(), Value::Ref(relative))];
        if let Some(parent) = relative_to {
            attributes.push(("PlacementRelTo".to_owned(), Value::Ref(parent)));
        }
        self.create("IfcLocalPlacement", attributes)
    }

    fn direction(&mut self, ratios: [f64; 3]) -> Result<EntityId, F> {
        self.create(
            "IfcDirection",
            vec![("DirectionRatios".to_owned(), reals(&ratios))],
        )
    }
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

fn finite(what: &str, values: &[f64; 3]) -> Result<(), F> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(invalid(&format!("{what} has a non-finite coordinate")))
    }
}

fn invalid(detail: &str) -> F {
    F::InvalidPlacement {
        detail: detail.to_owned(),
    }
}
