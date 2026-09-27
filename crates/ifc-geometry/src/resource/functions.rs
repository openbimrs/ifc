//! EXPRESS function coverage for the three IFC geometry resources.
//!
//! `Scaffolded` assigns an implementation owner but does not claim the
//! function is executable yet.

/// Current implementation state of one normative EXPRESS function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionStatus {
    /// Rust's format-neutral math primitive already provides the operation.
    NativePrimitive,
    /// An owner module and test target exist; semantics remain to implement.
    Scaffolded,
    /// The owner module executes the function's semantics, and a test
    /// exercises them. Distinct from `Scaffolded` so the registry cannot
    /// keep understating what the crate does.
    Implemented,
    /// The function is EXPRESS plumbing with no Rust counterpart: it exists
    /// only to re-index a LIST into an ARRAY with a different lower bound.
    /// A `Vec` already is that array, so implementing it would add a
    /// conversion that converts nothing. The `owner` names the module that
    /// consumes the underlying attribute directly.
    NotApplicable,
}

/// Auditable owner for one EXPRESS function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionSupport {
    /// Case-preserving EXPRESS function name.
    pub name: &'static str,
    /// Rust module or primitive responsible for the semantics.
    pub owner: &'static str,
    /// Honest implementation state.
    pub status: FunctionStatus,
}

const NATIVE: FunctionStatus = FunctionStatus::NativePrimitive;
/// No row currently uses this: every function is implemented, native, or
/// not applicable. It stays because `Scaffolded` is the honest default for
/// the next function added, and deleting it would push a future author
/// toward over-claiming instead.
#[allow(dead_code)]
const SCAFFOLDED: FunctionStatus = FunctionStatus::Scaffolded;
const NOT_APPLICABLE: FunctionStatus = FunctionStatus::NotApplicable;
const IMPLEMENTED: FunctionStatus = FunctionStatus::Implemented;

/// All 28 normative functions in deterministic schema order.
pub const FUNCTIONS: &[FunctionSupport] = &[
    FunctionSupport {
        name: "IfcAssociatedSurface",
        owner: "surface::basis",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcBaseAxis",
        owner: "resource::axes",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcBuild2Axes",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcBuildAxes",
        owner: "transform",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcConsecutiveSegments",
        owner: "rules::express",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcConstraintsParamBSpline",
        owner: "rules::express",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcCrossProduct",
        owner: "axiolid_core::Vec3::cross",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcCurveDim",
        owner: "rules::dimension",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcCurveWeightsPositive",
        owner: "rules::bspline",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcDotProduct",
        owner: "axiolid_core::Vec3::dot",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcFirstProjAxis",
        owner: "transform",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcGetBasisSurface",
        owner: "surface::basis",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcListToArray",
        owner: "curve::bspline",
        status: NOT_APPLICABLE,
    },
    FunctionSupport {
        name: "IfcMakeArrayOfArray",
        owner: "surface::bspline",
        status: NOT_APPLICABLE,
    },
    FunctionSupport {
        name: "IfcNormalise",
        owner: "axiolid_core::Vec3::normalize",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcOrthogonalComplement",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcSameAxis2Placement",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcSameCartesianPoint",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcSameDirection",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcSameValue",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcScalarTimesVector",
        owner: "axiolid_core::Vec3::mul",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcSecondProjAxis",
        owner: "transform",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcSurfaceWeightsPositive",
        owner: "rules::bspline",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcVectorDifference",
        owner: "axiolid_core::Vec3::sub",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcVectorSum",
        owner: "axiolid_core::Vec3::add",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcPointListDim",
        owner: "resource::functions",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcTaperedSweptAreaProfiles",
        owner: "rules::surface",
        status: IMPLEMENTED,
    },
    FunctionSupport {
        name: "IfcCorrectLocalPlacement",
        owner: "rules::placement",
        status: IMPLEMENTED,
    },
];

/// The schema's default comparison tolerance, from `IfcSameValue`.
const DEFAULT_EPSILON: f64 = 0.000_001;

/// `IfcSameValue`: are two reals equal within `epsilon`?
///
/// The schema writes this as `(v1 + eps > v2) AND (v1 < v2 + eps)`, which is
/// an *exclusive* band: values exactly `epsilon` apart are NOT equal. A
/// symmetric `(a - b).abs() <= eps` would accept that boundary, so the
/// strict comparison is kept deliberately.
///
/// `None` selects the schema default of 1e-6, matching EXPRESS `NVL`.
#[must_use]
pub fn same_value(a: f64, b: f64, epsilon: Option<f64>) -> bool {
    let eps = epsilon.unwrap_or(DEFAULT_EPSILON);
    a + eps > b && a < b + eps
}

/// `IfcSameCartesianPoint` / `IfcSameDirection` over raw component lists.
///
/// Both schema functions have identical bodies apart from the attribute they
/// read, so they share one implementation. A missing third component reads
/// as zero: the schema initialises `cp1z`/`dir1z` to 0 and only overwrites
/// when `SIZEOF > 2`, which makes a 2D point equal to its 3D lift.
#[must_use]
pub fn same_components(a: &[f64], b: &[f64], epsilon: Option<f64>) -> bool {
    let at = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(0.0);
    (0..3).all(|i| same_value(at(a, i), at(b, i), epsilon))
}

/// `IfcPointListDim`: the dimensionality a point list declares by its type.
///
/// `None` for anything that is not one of the two concrete list types, which
/// is the schema's `?` return rather than a guess.
#[must_use]
pub fn point_list_dim(type_name: &str) -> Option<usize> {
    match type_name.to_ascii_uppercase().as_str() {
        "IFCCARTESIANPOINTLIST2D" => Some(2),
        "IFCCARTESIANPOINTLIST3D" => Some(3),
        _ => None,
    }
}

/// `IfcOrthogonalComplement`: a 2D direction turned a quarter turn CCW.
///
/// The schema returns `?` for anything not 2D; that is the `None` here.
#[must_use]
pub fn orthogonal_complement(v: &[f64]) -> Option<[f64; 2]> {
    match v {
        [x, y] => Some([-*y, *x]),
        _ => None,
    }
}

/// `IfcBuild2Axes`: a 2D frame from an optional reference direction.
///
/// Defaults to +X when the direction is absent or degenerate, per the
/// schema's `NVL(IfcNormalise(RefDirection), IfcDirection([1.0, 0.0]))`.
#[must_use]
pub fn build_2_axes(ref_direction: Option<[f64; 2]>) -> ([f64; 2], [f64; 2]) {
    let x = ref_direction
        .and_then(|d| {
            let len = d[0].hypot(d[1]);
            (len.is_finite() && len > 0.0).then(|| [d[0] / len, d[1] / len])
        })
        .unwrap_or([1.0, 0.0]);
    (x, [-x[1], x[0]])
}

/// `IfcSameAxis2Placement`: do two placements describe the same frame?
///
/// Compares the two derived axes `P[1]`/`P[2]` and the location, each with
/// [`same_components`].
///
/// # The schema compares `ap1.Location` with itself
///
/// The published function reads
/// `IfcSameCartesianPoint(ap1.Location, ap1.Location, Epsilon)` -- `ap1`
/// twice, so the location is never actually tested and two placements that
/// differ only in origin compare equal. That is a defect, not an intent:
/// the surrounding function exists to decide frame identity, and the other
/// two terms both compare `ap1` against `ap2`. This implementation compares
/// `ap2.Location`, and the test below pins that difference so it stays a
/// deliberate divergence rather than drifting back.
#[must_use]
pub fn same_axis2_placement(
    a_axes: (&[f64], &[f64]),
    a_location: &[f64],
    b_axes: (&[f64], &[f64]),
    b_location: &[f64],
    epsilon: Option<f64>,
) -> bool {
    same_components(a_axes.0, b_axes.0, epsilon)
        && same_components(a_axes.1, b_axes.1, epsilon)
        && same_components(a_location, b_location, epsilon)
}

/// Look up a function case-insensitively.
pub fn function_support(name: &str) -> Option<&'static FunctionSupport> {
    FUNCTIONS
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod function_tests {
    use super::*;

    /// The schema's band is exclusive: `(v1 + eps > v2) AND (v1 < v2 + eps)`.
    /// Values exactly `epsilon` apart are NOT equal, which a symmetric
    /// `abs() <= eps` would wrongly accept.
    #[test]
    fn same_value_boundary_is_exclusive() {
        assert!(same_value(1.0, 1.0 + 0.5e-6, None));
        assert!(!same_value(1.0, 1.0 + 1e-6, None), "exactly eps apart");
        assert!(!same_value(1.0, 1.0 + 2e-6, None));
        // Symmetric in its arguments.
        assert!(!same_value(1.0 + 1e-6, 1.0, None));
    }

    /// `None` must select the schema default, not zero tolerance.
    #[test]
    fn same_value_default_epsilon_is_not_exact_equality() {
        assert!(same_value(2.0, 2.0 + 1e-9, None));
        assert!(!same_value(2.0, 2.0 + 1e-9, Some(1e-12)));
    }

    /// A missing third component reads as zero, so a 2D point equals its
    /// 3D lift but not a genuinely different Z.
    #[test]
    fn missing_third_component_is_zero() {
        assert!(same_components(&[1.0, 2.0], &[1.0, 2.0, 0.0], None));
        assert!(!same_components(&[1.0, 2.0], &[1.0, 2.0, 5.0], None));
    }

    #[test]
    fn point_list_dim_refuses_unknown_types() {
        assert_eq!(point_list_dim("IfcCartesianPointList2D"), Some(2));
        assert_eq!(point_list_dim("IFCCARTESIANPOINTLIST3D"), Some(3));
        assert_eq!(point_list_dim("IFCCARTESIANPOINT"), None);
    }

    /// The complement is a quarter turn counter-clockwise, and the schema
    /// returns `?` for anything that is not 2D.
    #[test]
    fn orthogonal_complement_is_2d_only() {
        assert_eq!(orthogonal_complement(&[1.0, 0.0]), Some([0.0, 1.0]));
        assert_eq!(orthogonal_complement(&[0.0, 1.0]), Some([-1.0, 0.0]));
        assert_eq!(orthogonal_complement(&[1.0, 0.0, 0.0]), None);
    }

    /// A degenerate or absent reference direction falls back to +X.
    #[test]
    fn build_2_axes_defaults_and_normalises() {
        assert_eq!(build_2_axes(None), ([1.0, 0.0], [0.0, 1.0]));
        assert_eq!(build_2_axes(Some([0.0, 0.0])), ([1.0, 0.0], [0.0, 1.0]));
        let (x, y) = build_2_axes(Some([5.0, 0.0]));
        assert!((x[0] - 1.0).abs() < 1e-12, "normalised: {x:?}");
        assert!((y[1] - 1.0).abs() < 1e-12, "perpendicular: {y:?}");
    }

    /// The schema's own body compares `ap1.Location` with `ap1.Location`,
    /// so locations are never tested. We compare `ap2` instead; this test
    /// pins that divergence.
    #[test]
    fn same_axis2_placement_actually_compares_locations() {
        let x = [1.0, 0.0, 0.0];
        let y = [0.0, 1.0, 0.0];
        assert!(same_axis2_placement(
            (&x, &y),
            &[0.0, 0.0, 0.0],
            (&x, &y),
            &[0.0, 0.0, 0.0],
            None
        ));
        // Same axes, different origin: equal under the schema as written,
        // NOT equal here.
        assert!(!same_axis2_placement(
            (&x, &y),
            &[0.0, 0.0, 0.0],
            (&x, &y),
            &[5.0, 0.0, 0.0],
            None
        ));
        // A differing axis is caught too.
        assert!(!same_axis2_placement(
            (&x, &y),
            &[0.0, 0.0, 0.0],
            (&y, &x),
            &[0.0, 0.0, 0.0],
            None
        ));
    }
}
