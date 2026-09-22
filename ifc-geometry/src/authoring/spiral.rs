//! Authoring transition spirals.
//!
//! A spiral is an alignment transition curve: curvature varies along
//! its length so a vehicle can move between a straight and an arc
//! without an instantaneous change of steering angle.
//!
//! All six share one shape -- a `Position` placement, one required
//! leading term, then optional lower-order terms in descending degree.
//! The terms are trailing OPTIONALs, so an absent one is `Null` in its
//! own slot rather than a shortened record: writing a shorter list
//! would silently shift every following term up a degree.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;

use super::{invalid, require_finite};

/// Which spiral to stage, with its defining terms.
///
/// Each variant names its terms as the schema does, highest degree
/// first. The leading term is required because it is what makes the
/// spiral that kind: a clothoid with no constant is not a clothoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpiralKind {
    /// `IfcClothoid`: curvature proportional to arc length.
    Clothoid {
        /// `ClothoidConstant`.
        constant: f64,
    },
    /// `IfcCosineSpiral`.
    Cosine {
        /// `CosineTerm`.
        cosine: f64,
        /// `ConstantTerm`.
        constant: Option<f64>,
    },
    /// `IfcSineSpiral`.
    Sine {
        /// `SineTerm`.
        sine: f64,
        /// `LinearTerm`.
        linear: Option<f64>,
        /// `ConstantTerm`.
        constant: Option<f64>,
    },
    /// `IfcSecondOrderPolynomialSpiral`.
    SecondOrder {
        /// `QuadraticTerm`.
        quadratic: f64,
        /// `LinearTerm`.
        linear: Option<f64>,
        /// `ConstantTerm`.
        constant: Option<f64>,
    },
    /// `IfcThirdOrderPolynomialSpiral`.
    ThirdOrder {
        /// `CubicTerm`.
        cubic: f64,
        /// `QuadraticTerm`.
        quadratic: Option<f64>,
        /// `LinearTerm`.
        linear: Option<f64>,
        /// `ConstantTerm`.
        constant: Option<f64>,
    },
    /// `IfcSeventhOrderPolynomialSpiral`.
    SeventhOrder {
        /// `SepticTerm`, then lower degrees in descending order.
        septic: f64,
        /// Optional terms, sextic down to constant.
        lower: [Option<f64>; 7],
    },
}

impl SpiralKind {
    /// The IFC entity name for this spiral.
    fn entity_name(self) -> &'static str {
        match self {
            Self::Clothoid { .. } => "IFCCLOTHOID",
            Self::Cosine { .. } => "IFCCOSINESPIRAL",
            Self::Sine { .. } => "IFCSINESPIRAL",
            Self::SecondOrder { .. } => "IFCSECONDORDERPOLYNOMIALSPIRAL",
            Self::ThirdOrder { .. } => "IFCTHIRDORDERPOLYNOMIALSPIRAL",
            Self::SeventhOrder { .. } => "IFCSEVENTHORDERPOLYNOMIALSPIRAL",
        }
    }

    /// The name of the required leading term, for error reporting.
    fn leading_attribute(self) -> &'static str {
        match self {
            Self::Clothoid { .. } => "ClothoidConstant",
            Self::Cosine { .. } => "CosineTerm",
            Self::Sine { .. } => "SineTerm",
            Self::SecondOrder { .. } => "QuadraticTerm",
            Self::ThirdOrder { .. } => "CubicTerm",
            Self::SeventhOrder { .. } => "SepticTerm",
        }
    }

    /// The term slots after `Position`, in schema order.
    fn terms(self) -> (f64, Vec<Option<f64>>) {
        match self {
            Self::Clothoid { constant } => (constant, Vec::new()),
            Self::Cosine { cosine, constant } => (cosine, vec![constant]),
            Self::Sine {
                sine,
                linear,
                constant,
            } => (sine, vec![linear, constant]),
            Self::SecondOrder {
                quadratic,
                linear,
                constant,
            } => (quadratic, vec![linear, constant]),
            Self::ThirdOrder {
                cubic,
                quadratic,
                linear,
                constant,
            } => (cubic, vec![quadratic, linear, constant]),
            Self::SeventhOrder { septic, lower } => (septic, lower.to_vec()),
        }
    }
}

/// Stage a transition spiral.
///
/// # Errors
///
/// Refuses a non-finite term, and a zero leading term: the leading
/// coefficient is what gives the spiral its degree, so a zero one
/// describes a lower-order curve wearing the wrong type name.
pub fn spiral(
    tx: &mut Transaction,
    position: EntityId,
    kind: SpiralKind,
) -> Result<EntityId, GeometryError> {
    let entity = kind.entity_name();
    let (leading, lower) = kind.terms();

    require_finite(entity, kind.leading_attribute(), &[leading])?;
    if leading == 0.0 {
        return Err(invalid(
            entity,
            kind.leading_attribute(),
            "expected a non-zero leading term",
        ));
    }
    let present: Vec<f64> = lower.iter().flatten().copied().collect();
    require_finite(entity, "term", &present)?;

    let mut attrs = vec![Value::Ref(position), Value::Real(leading)];
    attrs.extend(
        lower
            .into_iter()
            .map(|t| t.map_or(Value::Null, Value::Real)),
    );
    Ok(tx.create(Entity::new(entity, attrs)))
}
