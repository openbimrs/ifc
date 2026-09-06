//! Dimensional `WHERE` rules on curves, transformation operators and sets.
//!
//! Every rule here is a statement about `Dim`, which is DERIVED. See
//! [`super::dimension`] for the derivation itself; this module only decides
//! what to do when the derived answer disagrees with the schema.

use ifc_model::{Entity, EntityId, Model, Value};

use super::dimension::{dim_of, first_dim_disagreement, list_refs};
use super::violation::{RuleViolation, ViolationKind};

/// The entity a rule is being evaluated against.
///
/// These three travel together through every helper; passing them as one
/// value keeps the signatures readable and satisfies the argument-count lint.
#[derive(Clone, Copy)]
struct Subject<'a> {
    id: EntityId,
    entity: &'a Entity,
    type_name: &'a str,
}

/// Run the dimensional curve/operator rules that apply to this entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let name = entity.type_name.to_ascii_uppercase();
    let subject = Subject {
        id,
        entity,
        type_name: &name,
    };
    match name.as_str() {
        "IFCPOLYLINE" => same_dim_list(model, subject, 0, "SameDim", "Points", out),
        "IFCGEOMETRICSET" | "IFCGEOMETRICCURVESET" => {
            same_dim_list(model, subject, 0, "ConsistentDim", "Elements", out)
        }
        "IFCLINE" => line_same_dim(model, subject, out),
        "IFCPCURVE" => fixed_dim_ref(model, subject, 1, 2, "DimIs2D", out),
        "IFCOFFSETCURVE2D" => fixed_dim_ref(model, subject, 0, 2, "DimIs2D", out),
        "IFCOFFSETCURVE3D" => fixed_dim_ref(model, subject, 0, 3, "DimIs2D", out),
        "IFCSURFACECURVE" | "IFCINTERSECTIONCURVE" | "IFCSEAMCURVE" => {
            fixed_dim_ref(model, subject, 0, 3, "CurveIs3D", out)
        }
        "IFCSWEPTDISKSOLID" | "IFCSWEPTDISKSOLIDPOLYGONAL" => {
            fixed_dim_ref(model, subject, 0, 3, "DirectrixDim", out)
        }
        "IFCSECTIONEDSPINE" => fixed_dim_ref(model, subject, 0, 3, "SpineCurveDim", out),
        "IFCPOLYGONALBOUNDEDHALFSPACE" => fixed_dim_ref(model, subject, 3, 2, "BoundaryDim", out),
        _ => {}
    }
    if crate::select::is_a(&name, "IFCBSPLINECURVE") {
        // ControlPointsList is slot 1.
        same_dim_list(model, subject, 1, "SameDim", "ControlPointsList", out);
    }
    if crate::select::is_a(&name, "IFCCOMPOSITECURVE") {
        same_dim_list(model, subject, 0, "SameDim", "Segments", out);
    }
    // The subtype table does not carry the transformation-operator family,
    // so match the name directly rather than through `is_a`: an unknown
    // entity has an empty supertype chain and would silently never dispatch.
    if name.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR") {
        transformation_operator(model, subject, out);
    }
}

/// Every member of a list-valued slot must share one dimensionality.
fn same_dim_list(
    model: &Model,
    subject: Subject<'_>,
    slot: usize,
    rule: &'static str,
    label: &str,
    out: &mut Vec<RuleViolation>,
) {
    let Subject {
        id,
        entity,
        type_name,
    } = subject;
    let ids = list_refs(entity, slot);
    if let Some((expected, found, offender)) = first_dim_disagreement(model, &ids) {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            rule,
            ViolationKind::Disagreement,
            format!("{label} starts {expected}D but {offender} is {found}D"),
        ));
    }
}

/// A referenced entity must have exactly the dimensionality the schema fixes.
fn fixed_dim_ref(
    model: &Model,
    subject: Subject<'_>,
    slot: usize,
    want: usize,
    rule: &'static str,
    out: &mut Vec<RuleViolation>,
) {
    let Subject {
        id,
        entity,
        type_name,
    } = subject;
    let Some(Value::Ref(target)) = entity.attribute(slot).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(found) = dim_of(model, *target) else {
        return;
    };
    if found != want {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            rule,
            ViolationKind::Dimensionality,
            format!("{target} is {found}D, must be {want}D"),
        ));
    }
}

/// `IfcLine.SameDim`: the direction and the point must agree.
fn line_same_dim(model: &Model, subject: Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Subject {
        id,
        entity,
        type_name,
    } = subject;
    let (Some(Value::Ref(pnt)), Some(Value::Ref(dir))) = (
        entity.attribute(0).map(|v| v.unwrap_typed()),
        entity.attribute(1).map(|v| v.unwrap_typed()),
    ) else {
        return;
    };
    let (Some(pd), Some(dd)) = (dim_of(model, *pnt), dim_of(model, *dir)) else {
        return;
    };
    if pd != dd {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            "SameDim",
            ViolationKind::Disagreement,
            format!("Pnt {pnt} is {pd}D but Dir {dir} is {dd}D"),
        ));
    }
}

/// The 2D/3D transformation operators constrain their own `Dim` and each
/// optional axis.
///
/// `Dim` here is `LocalOrigin.Dim`, so `DimEqual2`/`DimIs3D` are really
/// statements about the local origin point.
fn transformation_operator(model: &Model, subject: Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Subject {
        id,
        entity,
        type_name,
    } = subject;
    // Same reason as the dispatch above: name-prefix, not the subtype table.
    // The NONUNIFORM subtypes end in 2DNONUNIFORM / 3DNONUNIFORM, so test the
    // dimension marker rather than a suffix.
    let subject = Subject {
        id,
        entity,
        type_name,
    };
    let (want, dim_rule) = if type_name.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR3D") {
        (3usize, "DimIs3D")
    } else if type_name.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR2D") {
        (2usize, "DimEqual2")
    } else {
        return;
    };

    // LocalOrigin is slot 2 and carries the operator's own Dim.
    fixed_dim_ref(model, subject, 2, want, dim_rule, out);

    // Axis1/Axis2 are optional; Axis3 exists only on the 3D operator.
    let axes: &[(usize, &'static str)] = if want == 3 {
        &[(0, "Axis1Is3D"), (1, "Axis2Is3D"), (4, "Axis3Is3D")]
    } else {
        &[(0, "Axis1Is2D"), (1, "Axis2Is2D")]
    };
    for (slot, rule) in axes {
        fixed_dim_ref(model, subject, *slot, want, rule, out);
    }
}
