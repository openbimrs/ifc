//! Curves that live on a surface, and the segment forms.
//!
//! # A boundary curve must be closed
//!
//! `IfcBoundaryCurve` carries `IsClosed : SELF\IfcCompositeCurve.ClosedCurve`.
//! Everywhere else in this crate `ClosedCurve` is written UNKNOWN,
//! because deciding it needs an evaluator -- but here the schema
//! *requires* it true, so a boundary curve that admits UNKNOWN is
//! non-conforming. The writer states TRUE and the caller is telling
//! it the curve closes; that is a claim the file makes either way.
//!
//! # Two pcurves, and whether their surfaces match
//!
//! `IfcIntersectionCurve` and `IfcSeamCurve` both require exactly two
//! associated pcurves. They differ in what the surfaces must be: an
//! intersection needs two *distinct* surfaces, a seam needs the *same*
//! surface twice -- it is the join where a closed surface meets
//! itself. The count is checkable here; which surface each pcurve
//! sits on requires resolving the reference, so that is left to the
//! validator.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::curve::composite::{curve_slot, segment_slot};
use crate::curve::TransitionCode;
use crate::error::GeometryError;

use super::{invalid, refs, require_finite};

/// Which representation of a surface curve is authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCurveRepresentation {
    /// The 3D curve is the master.
    Curve3D,
    /// The first parameter-space curve is the master.
    PCurveS1,
    /// The second parameter-space curve is the master.
    PCurveS2,
}

impl SurfaceCurveRepresentation {
    /// The EXPRESS token.
    fn token(self) -> &'static str {
        match self {
            Self::Curve3D => "CURVE3D",
            Self::PCurveS1 => "PCURVE_S1",
            Self::PCurveS2 => "PCURVE_S2",
        }
    }
}

/// Stage an `IfcPcurve`: a curve in a surface's parameter space.
pub fn pcurve(
    tx: &mut Transaction,
    basis_surface: EntityId,
    reference_curve: EntityId,
) -> EntityId {
    let attrs = vec![Value::Ref(basis_surface), Value::Ref(reference_curve)];
    tx.create(Entity::new("IFCPCURVE", attrs))
}

/// Which surface curve flavour to author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCurveKind {
    /// `IfcSurfaceCurve`: a curve lying on one or two surfaces.
    Plain,
    /// `IfcIntersectionCurve`: where two *distinct* surfaces meet.
    Intersection,
    /// `IfcSeamCurve`: where one closed surface meets itself.
    Seam,
}

impl SurfaceCurveKind {
    /// The entity type name.
    fn type_name(self) -> &'static str {
        match self {
            Self::Plain => "IFCSURFACECURVE",
            Self::Intersection => "IFCINTERSECTIONCURVE",
            Self::Seam => "IFCSEAMCURVE",
        }
    }

    /// Does this flavour require exactly two pcurves?
    fn needs_two(self) -> bool {
        matches!(self, Self::Intersection | Self::Seam)
    }
}

/// Stage a surface curve.
///
/// # Errors
///
/// Refuses an empty or over-long pcurve list -- `LIST [1:2]` -- and
/// refuses anything but exactly two for the intersection and seam
/// forms, whose `TwoPCurves` rule demands both.
pub fn surface_curve(
    tx: &mut Transaction,
    kind: SurfaceCurveKind,
    curve_3d: EntityId,
    associated_geometry: &[EntityId],
    master: SurfaceCurveRepresentation,
) -> Result<EntityId, GeometryError> {
    let type_name = kind.type_name();
    if associated_geometry.is_empty() || associated_geometry.len() > 2 {
        return Err(invalid(
            type_name,
            "AssociatedGeometry",
            format!("expected 1 or 2 pcurves, got {}", associated_geometry.len()),
        ));
    }
    if kind.needs_two() && associated_geometry.len() != 2 {
        return Err(invalid(
            type_name,
            "AssociatedGeometry",
            "TwoPCurves: this form needs a pcurve on each surface",
        ));
    }
    let attrs = vec![
        Value::Ref(curve_3d),
        refs(associated_geometry),
        Value::Enum(master.token().into()),
    ];
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// Which composite-curve-on-surface flavour to author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnSurfaceKind {
    /// `IfcCompositeCurveOnSurface`: no closure requirement.
    Composite,
    /// `IfcBoundaryCurve`: must be closed.
    Boundary,
    /// `IfcOuterBoundaryCurve`: the outermost boundary, also closed.
    OuterBoundary,
}

impl OnSurfaceKind {
    /// The entity type name.
    fn type_name(self) -> &'static str {
        match self {
            Self::Composite => "IFCCOMPOSITECURVEONSURFACE",
            Self::Boundary => "IFCBOUNDARYCURVE",
            Self::OuterBoundary => "IFCOUTERBOUNDARYCURVE",
        }
    }

    /// Does the schema require `ClosedCurve` to be true?
    fn must_close(self) -> bool {
        matches!(self, Self::Boundary | Self::OuterBoundary)
    }
}

/// Stage a composite curve lying on a surface.
///
/// For the two boundary forms `ClosedCurve` is written TRUE, because
/// `IsClosed` requires it: a boundary that is not closed bounds
/// nothing. Elsewhere this crate writes UNKNOWN, since closure needs
/// an evaluator -- here the schema has already decided.
///
/// # Errors
///
/// Refuses an empty segment list: `LIST [1:?]`.
pub fn composite_curve_on_surface(
    tx: &mut Transaction,
    kind: OnSurfaceKind,
    segments: &[EntityId],
) -> Result<EntityId, GeometryError> {
    let type_name = kind.type_name();
    if segments.is_empty() {
        return Err(invalid(
            type_name,
            "Segments",
            "expected at least one segment",
        ));
    }
    let mut attrs = vec![Value::Null; 2];
    attrs[curve_slot::SEGMENTS] = refs(segments);
    attrs[curve_slot::SELF_INTERSECT] = if kind.must_close() {
        // IsClosed: the schema requires this, so UNKNOWN is not an option.
        Value::Bool(true)
    } else {
        Value::LogicalUnknown
    };
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// Stage an `IfcReparametrisedCompositeCurveSegment`.
///
/// The plain segment's three slots plus `ParamLength`, which restates
/// the segment's parameter range so a composite can be walked without
/// evaluating each piece.
///
/// # Errors
///
/// Refuses a non-positive or non-finite parameter length.
pub fn reparametrised_composite_curve_segment(
    tx: &mut Transaction,
    transition: TransitionCode,
    same_sense: bool,
    parent_curve: EntityId,
    param_length: f64,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT";
    require_finite(T, "ParamLength", &[param_length])?;
    if param_length <= 0.0 {
        return Err(invalid(
            T,
            "ParamLength",
            format!("expected a positive parameter length, got {param_length}"),
        ));
    }
    let mut attrs = vec![Value::Null; 4];
    attrs[segment_slot::TRANSITION] = Value::Enum(transition.token().into());
    attrs[segment_slot::SAME_SENSE] = Value::Bool(same_sense);
    attrs[segment_slot::PARENT_CURVE] = Value::Ref(parent_curve);
    attrs[segment_slot::PARAM_LENGTH] = Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(Value::Real(param_length)),
    };
    Ok(tx.create(Entity::new(T, attrs)))
}

/// A length along a curve, or a parameter in its own space.
///
/// `IfcCurveMeasureSelect` admits either, and the two are not
/// interchangeable: a length is in the model's length unit, a
/// parameter is in whatever the curve's own parameterisation uses.
/// Writing one where the other is meant rescales the segment.
#[derive(Debug, Clone, Copy)]
pub enum CurveMeasure {
    /// `IfcNonNegativeLengthMeasure`: a distance along the curve.
    Length(f64),
    /// `IfcParameterValue`: a value in the curve's parameter space.
    Parameter(f64),
}

impl CurveMeasure {
    /// Encode with the measure type that says which kind this is.
    fn to_value(
        self,
        type_name: &'static str,
        attribute: &'static str,
    ) -> Result<Value, GeometryError> {
        let (measure, value) = match self {
            Self::Length(v) => ("IFCNONNEGATIVELENGTHMEASURE", v),
            Self::Parameter(v) => ("IFCPARAMETERVALUE", v),
        };
        require_finite(type_name, attribute, &[value])?;
        if matches!(self, Self::Length(_)) && value < 0.0 {
            return Err(invalid(
                type_name,
                attribute,
                format!("expected a non-negative length, got {value}"),
            ));
        }
        Ok(Value::Typed {
            type_name: measure.into(),
            value: Box::new(Value::Real(value)),
        })
    }
}

/// Stage an `IfcCurveSegment`.
///
/// The alignment-era segment: it carries its own placement and states
/// where along the parent curve it starts and how far it runs, rather
/// than relying on the parent's parameterisation alone.
///
/// # Errors
///
/// Refuses a non-finite measure, or a negative length measure.
pub fn curve_segment(
    tx: &mut Transaction,
    transition: TransitionCode,
    placement: EntityId,
    segment_start: CurveMeasure,
    segment_length: CurveMeasure,
    parent_curve: EntityId,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCCURVESEGMENT";
    let attrs = vec![
        Value::Enum(transition.token().into()),
        Value::Ref(placement),
        segment_start.to_value(T, "SegmentStart")?,
        segment_length.to_value(T, "SegmentLength")?,
        Value::Ref(parent_curve),
    ];
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcGradientCurve`.
///
/// A composite curve carrying a vertical profile over a horizontal
/// `base_curve` -- the alignment case where gradient is layered on
/// plan geometry.
///
/// # Errors
///
/// Refuses an empty segment list: `LIST [1:?]`.
pub fn gradient_curve(
    tx: &mut Transaction,
    segments: &[EntityId],
    base_curve: EntityId,
    end_point: Option<EntityId>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCGRADIENTCURVE";
    if segments.is_empty() {
        return Err(invalid(T, "Segments", "expected at least one segment"));
    }
    let attrs = vec![
        refs(segments),
        Value::LogicalUnknown,
        Value::Ref(base_curve),
        end_point.map_or(Value::Null, Value::Ref),
    ];
    Ok(tx.create(Entity::new(T, attrs)))
}
