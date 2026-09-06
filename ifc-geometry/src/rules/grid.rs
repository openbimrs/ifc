//! `IfcGridAxis` WHERE rules.
//!
//! Both rules the schema declares for a grid axis, named as it names
//! them. `WR1` keeps an axis curve in the 2D grid plane; `WR2` keeps an
//! axis in exactly one of the U/V/W lists.

use ifc_model::{Entity, EntityId, Model};

use super::violation::{RuleViolation, ViolationKind};
use crate::constraint::grid::GridAxis;
use crate::resource::point::CartesianPoint;

/// Dispatch grid rules for one entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    if entity.type_name.eq_ignore_ascii_case("IFCGRIDAXIS") {
        grid_axis(model, id, entity, out);
    }
}

/// `IfcGridAxis`: `WR1` (AxisCurve.Dim = 2) and `WR2` (exactly one of
/// PartOfU / PartOfV / PartOfW).
///
/// `WR2` is stated over inverse attributes, which a Part 21 file does not
/// carry: an instance lists no PartOfU/V/W, the grid lists its axes. It is
/// checked by counting the grid axis lists that reference this axis, which
/// is the same relation read from the owning side.
fn grid_axis(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCGRIDAXIS";
    let view = GridAxis::new(id, entity);
    if let Ok(curve) = view.curve() {
        if let Some(dim) = curve_dim(model, curve) {
            if dim != 2 {
                out.push(RuleViolation::new(
                    id,
                    TYPE,
                    "WR1",
                    ViolationKind::Dimensionality,
                    format!("AxisCurve {curve} is {dim}D, must be 2D"),
                ));
            }
        }
    }
    grid_axis_membership(model, id, out);
}

/// `WR2`: an axis belongs to exactly one of a grid's U/V/W lists.
///
/// An axis referenced by two lists is ambiguous -- the same line would be
/// both a U and a V axis -- and one referenced by none is unreachable from
/// any grid. Both are reported, because both make the axis unusable.
fn grid_axis_membership(model: &Model, id: EntityId, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCGRIDAXIS";
    let mut memberships = 0usize;
    for (grid_id, grid) in model.iter() {
        if !grid.type_name.eq_ignore_ascii_case("IFCGRID") {
            continue;
        }
        let _ = grid_id;
        for slot in [grid_slot::U_AXES, grid_slot::V_AXES, grid_slot::W_AXES] {
            let Some(list) = grid.attributes.get(slot).and_then(|value| value.as_list()) else {
                continue;
            };
            if list
                .iter()
                .filter_map(ifc_model::Value::as_ref_id)
                .any(|a| a == id)
            {
                memberships += 1;
            }
        }
    }
    if memberships != 1 {
        out.push(RuleViolation::new(
            id,
            TYPE,
            "WR2",
            ViolationKind::Disagreement,
            format!("axis belongs to {memberships} grid axis lists, must belong to exactly 1"),
        ));
    }
}

/// `IfcGrid` axis-list slots.
///
/// Verified against the schema: IfcRoot 4 + IfcObject 1 + IfcProduct 2 = 7
/// inherited slots, so IfcGrid's own attributes start at 7.
mod grid_slot {
    /// `UAxes`.
    pub const U_AXES: usize = 7;
    /// `VAxes`.
    pub const V_AXES: usize = 8;
    /// `WAxes`.
    pub const W_AXES: usize = 9;
}

/// Dimensionality of a grid axis curve, from the geometry it is defined by.
///
/// A curve has no dimensionality attribute; EXPRESS derives `Dim` per
/// family. Only the families a grid axis actually uses are resolved, and
/// anything else returns `None` so an unknown family is reported by no
/// rule rather than by a wrong one.
fn curve_dim(model: &Model, id: EntityId) -> Option<usize> {
    let entity = model.get(id)?;
    match entity.type_name.to_ascii_uppercase().as_str() {
        "IFCPOLYLINE" => {
            let first = entity.attributes.first()?.as_list()?.first()?.as_ref_id()?;
            point_dim(model, first)
        }
        "IFCLINE" => {
            let pnt = entity.attributes.first()?.as_ref_id()?;
            point_dim(model, pnt)
        }
        "IFCCIRCLE" | "IFCELLIPSE" => {
            let position = entity.attributes.first()?.as_ref_id()?;
            let placement = model.get(position)?;
            match placement.type_name.to_ascii_uppercase().as_str() {
                "IFCAXIS2PLACEMENT2D" => Some(2),
                "IFCAXIS2PLACEMENT3D" => Some(3),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Dimensionality of a referenced `IfcCartesianPoint`.
fn point_dim(model: &Model, id: EntityId) -> Option<usize> {
    let entity = model.get(id)?;
    CartesianPoint::new(id, entity)
        .coordinates()
        .ok()
        .map(|c| c.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    fn point(model: &mut Model, id: u64, coords: &[f64]) {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(
                    coords.iter().map(|c| Value::Real(*c)).collect(),
                )],
            ),
        );
    }

    fn polyline(model: &mut Model, id: u64, pts: &[u64]) {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCPOLYLINE",
                vec![Value::List(
                    pts.iter().map(|p| Value::Ref(EntityId(*p))).collect(),
                )],
            ),
        );
    }

    fn axis(model: &mut Model, id: u64, curve: u64) {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCGRIDAXIS",
                vec![
                    Value::Text("A".into()),
                    Value::Ref(EntityId(curve)),
                    Value::Bool(true),
                ],
            ),
        );
    }

    /// A grid with one axis in exactly one list is conforming.
    fn grid(model: &mut Model, id: u64, u: &[u64], v: &[u64], w: &[u64]) {
        let list =
            |ids: &[u64]| Value::List(ids.iter().map(|a| Value::Ref(EntityId(*a))).collect());
        let mut attrs = vec![Value::Null; 7];
        attrs.push(list(u));
        attrs.push(list(v));
        attrs.push(list(w));
        model.insert(EntityId(id), Entity::new("IFCGRID", attrs));
    }

    /// A 3D axis curve cannot lie in the grid plane. This is WR1.
    #[test]
    fn a_three_dimensional_axis_curve_violates_wr1() {
        let mut model = Model::new();
        point(&mut model, 1, &[0.0, 0.0, 0.0]);
        polyline(&mut model, 2, &[1]);
        axis(&mut model, 3, 2);
        grid(&mut model, 4, &[3], &[], &[]);
        let found = crate::rules::validate(&model, EntityId(3));
        let wr1: Vec<_> = found.iter().filter(|v| v.rule == "WR1").collect();
        assert_eq!(wr1.len(), 1, "a 3D axis curve must be reported");
        assert!(wr1[0].detail.contains("3D"));
    }

    /// The conforming case must stay silent, or the rule is noise.
    #[test]
    fn a_two_dimensional_axis_in_one_list_is_silent() {
        let mut model = Model::new();
        point(&mut model, 1, &[0.0, 0.0]);
        polyline(&mut model, 2, &[1]);
        axis(&mut model, 3, 2);
        grid(&mut model, 4, &[3], &[], &[]);
        assert!(crate::rules::validate(&model, EntityId(3)).is_empty());
    }

    /// An axis in both U and V is ambiguous: WR2 is exclusive.
    #[test]
    fn an_axis_in_two_lists_violates_wr2() {
        let mut model = Model::new();
        point(&mut model, 1, &[0.0, 0.0]);
        polyline(&mut model, 2, &[1]);
        axis(&mut model, 3, 2);
        grid(&mut model, 4, &[3], &[3], &[]);
        let found = crate::rules::validate(&model, EntityId(3));
        let wr2: Vec<_> = found.iter().filter(|v| v.rule == "WR2").collect();
        assert_eq!(wr2.len(), 1);
        assert!(wr2[0].detail.contains("2 grid axis lists"));
    }

    /// An orphan axis belongs to no grid and is unreachable.
    #[test]
    fn an_axis_in_no_list_violates_wr2() {
        let mut model = Model::new();
        point(&mut model, 1, &[0.0, 0.0]);
        polyline(&mut model, 2, &[1]);
        axis(&mut model, 3, 2);
        grid(&mut model, 4, &[], &[], &[]);
        let found = crate::rules::validate(&model, EntityId(3));
        assert!(found.iter().any(|v| v.rule == "WR2"));
    }
}
