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
const SCAFFOLDED: FunctionStatus = FunctionStatus::Scaffolded;

/// All 28 normative functions in deterministic schema order.
pub const FUNCTIONS: &[FunctionSupport] = &[
    FunctionSupport {
        name: "IfcAssociatedSurface",
        owner: "curve::offset",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcBaseAxis",
        owner: "resource::axes",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcBuild2Axes",
        owner: "resource::axes",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcBuildAxes",
        owner: "resource::axes",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcConsecutiveSegments",
        owner: "curve::composite",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcConstraintsParamBSpline",
        owner: "curve::bspline",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcCrossProduct",
        owner: "axiolid_core::Vec3::cross",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcCurveDim",
        owner: "curve",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcCurveWeightsPositive",
        owner: "curve::bspline",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcDotProduct",
        owner: "axiolid_core::Vec3::dot",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcFirstProjAxis",
        owner: "resource::axes",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcGetBasisSurface",
        owner: "surface",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcListToArray",
        owner: "resource::functions",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcMakeArrayOfArray",
        owner: "resource::functions",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcNormalise",
        owner: "axiolid_core::Vec3::normalize",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcOrthogonalComplement",
        owner: "resource::axes",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcSameAxis2Placement",
        owner: "resource::placement",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcSameCartesianPoint",
        owner: "resource::point",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcSameDirection",
        owner: "resource::direction",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcSameValue",
        owner: "rules",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcScalarTimesVector",
        owner: "axiolid_core::Vec3::mul",
        status: NATIVE,
    },
    FunctionSupport {
        name: "IfcSecondProjAxis",
        owner: "resource::axes",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcSurfaceWeightsPositive",
        owner: "surface::bspline",
        status: SCAFFOLDED,
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
        owner: "resource::point",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcTaperedSweptAreaProfiles",
        owner: "lower::swept",
        status: SCAFFOLDED,
    },
    FunctionSupport {
        name: "IfcCorrectLocalPlacement",
        owner: "constraint::local",
        status: SCAFFOLDED,
    },
];

/// Look up a function case-insensitively.
pub fn function_support(name: &str) -> Option<&'static FunctionSupport> {
    FUNCTIONS
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(name))
}
