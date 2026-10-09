//! `IfcGridAxis` WHERE rules.
//!
//! Both rules the schema declares for a grid axis, named as it names
//! them. `WR1` keeps an axis curve in the 2D grid plane; `WR2` keeps an
//! axis in exactly one of the U/V/W lists.

use ifc_model::Model;

use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};
use crate::constraint::grid::GridAxis;

/// Dispatch grid rules for one entity.
///
/// IFC2X3 TC1 to IFC4X3 ADD2 all declare `WR1` and `WR2` with the same
/// text and the same labels.
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    if let Some(rule) = s.rule("IFCGRIDAXIS", "WR1") {
        axis_curve_is_2d(s, rule, out);
    }
    if let Some(rule) = s.rule("IFCGRIDAXIS", "WR2") {
        grid_axis_membership(s, rule, out);
    }
}

/// `WR1`: `AxisCurve.Dim = 2`, with `Dim` derived by the release's
/// `IfcCurveDim`.
fn axis_curve_is_2d(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    let view = GridAxis::new(s.id, s.entity);
    let Ok(curve) = view.curve() else {
        return;
    };
    if let Some(dim) = super::dimension::dim_of(s.release, s.model, curve) {
        if dim != 2 {
            out.push(s.violation(
                rule,
                ViolationKind::Dimensionality,
                format!("AxisCurve {curve} is {dim}D, must be 2D"),
            ));
        }
    }
}

/// `WR2`: an axis belongs to exactly one of a grid's U/V/W lists.
///
/// `WR2` is stated over inverse attributes, which a Part 21 file does not
/// carry: an instance lists no PartOfU/V/W, the grid lists its axes. It is
/// checked by counting the grid axis lists that reference this axis, which
/// is the same relation read from the owning side.
///
/// An axis referenced by two lists is ambiguous -- the same line would be
/// both a U and a V axis -- and one referenced by none is unreachable from
/// any grid. Both are reported, because both make the axis unusable.
fn grid_axis_membership(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    let model: &Model = s.model;
    let mut memberships = 0usize;
    for (_, grid) in model.iter() {
        if !s.release.is_a(&grid.type_name, "IFCGRID") {
            continue;
        }
        for slot in [grid_slot::U_AXES, grid_slot::V_AXES, grid_slot::W_AXES] {
            let Some(list) = grid.attributes.get(slot).and_then(|value| value.as_list()) else {
                continue;
            };
            if list
                .iter()
                .filter_map(ifc_model::Value::as_ref_id)
                .any(|a| a == s.id)
            {
                memberships += 1;
            }
        }
    }
    if memberships != 1 {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("axis belongs to {memberships} grid axis lists, must belong to exactly 1"),
        ));
    }
}

/// `IfcGrid` axis-list slots.
///
/// Verified against the schema: IfcRoot 4 + IfcObject 1 + IfcProduct 2 = 7
/// inherited slots, so IfcGrid's own attributes start at 7.
pub(crate) mod grid_slot {
    /// `UAxes`.
    pub const U_AXES: usize = 7;
    /// `VAxes`.
    pub const V_AXES: usize = 8;
    /// `WAxes`.
    pub const W_AXES: usize = 9;
}

#[cfg(test)]
mod tests {
    use ifc_model::{Entity, EntityId, Model, Value};

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
