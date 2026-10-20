//! Convert a model's length unit (#469): rescale every length-dimensioned
//! value to a new project length unit, in one checked transaction.
//!
//! IFC states lengths as bare numbers in the project's `LENGTHUNIT`, areas
//! in its `AREAUNIT` and volumes in its `VOLUMEUNIT`. Changing the unit
//! without rescaling the numbers resizes the building; this operation
//! assigns the new unit and rescales the numbers so the model means what it
//! meant before.
//!
//! # Which values are lengths
//!
//! The schema decides, not a hand list. Every attribute of every entity is
//! read against the declared release's table (IFC2X3 TC1, IFC4 ADD2 TC1 or
//! IFC4X3 ADD2; any other release is refused): a value whose declared type
//! is a measure taking a `LENGTHUNIT` (`IfcLengthMeasure`,
//! `IfcPositiveLengthMeasure`, `IfcNonNegativeLengthMeasure`, ...), an
//! `AREAUNIT` or a `VOLUMEUNIT`, as `ifc_properties::measure_unit_type`
//! derives it from the release, is rescaled; so is a value in a `SELECT`
//! whose typed wrapper names such a measure (`IFCLENGTHMEASURE(2.5)` in a
//! property, `IfcSizeSelect`, `IfcCurveMeasureSelect`). Aggregates are
//! rescaled element by element (`IfcCartesianPointList3D.CoordList`).
//! Points, distances, extrusion depths, profile dimensions, radii,
//! placements, half-space boundaries, alignment parameters, quantities and
//! property values are all reached this way.
//!
//! A value with its own unit keeps it and is not rescaled: a property or
//! quantity with a `Unit` (directly, or through its `IfcPropertyEnumeration`,
//! its time series or its table column), and an `IfcMeasureWithUnit`. The
//! replaced unit entity stays in the model while anything still refers to
//! it, so those values keep their meaning.
//!
//! # Values that are not measures
//!
//! Every `REAL` and `IfcParameterValue` attribute of the three releases is
//! reviewed in `classify.rs`, and a test fails when a release declares one
//! the table does not list:
//!
//! - **Unitless, kept:** direction ratios, the `Scale` of a
//!   `IfcCartesianTransformationOperator`, B-spline weights and knots,
//!   triangulation normals, texture coordinates, refraction indices.
//! - **Lengths stated as `REAL`, rescaled:**
//!   `IfcGeometricRepresentationContext.Precision`, a distance ("the maximum
//!   distance from an edge curve to the underlying face surface").
//! - **Inverse lengths, rescaled inversely:** `IfcLightSourcePositional`
//!   `DistanceAttenuation` and `QuadricAttenuation`, proportional to the
//!   distance and its square.
//! - **Refused by name:** `IfcPolynomialCurve` coefficients (their unit
//!   depends on a parameterisation IFC4.3 does not state) and a
//!   `IfcTextureCoordinateGenerator` with parameters (their meaning depends
//!   on the X3D mode).
//!
//! # Parameter values
//!
//! An `IfcParameterValue` is a coordinate in its curve's or surface's own
//! parameter space (ISO 10303-42, as the IFC entity definitions adopt it),
//! so the curve decides. Unitless, and kept, on `IfcLine` (a multiple of its
//! `IfcVector`, whose `Magnitude` is a length and is rescaled), on conics
//! (an angle), polylines and indexed poly curves (an ordinal), B-splines (the
//! knot space), clothoids (`s = A u √π`), composite curves of
//! `IfcCompositeCurveSegment`s (the accumulated parametric lengths of those),
//! and trimmed and offset curves (their basis). A length on `IfcPlane` (both
//! directions) and in the axial direction of `IfcCylindricalSurface`, and
//! rescaled there. Anything else (other spirals, `IfcCurveSegment`s,
//! p-curves, offset curves by distances) has no parameterisation this
//! conversion can carry and is refused. So is an `IfcPcurve` on any surface
//! but a plane: its reference curve's coordinates are `(u, v)` parameters
//! written as lengths, and on a cylinder `u` is an angle.
//!
//! # Georeferencing
//!
//! `IfcMapConversion` places the project in a map CRS: `Eastings`,
//! `Northings` and `OrthogonalHeight` are in the target CRS's `MapUnit`, or
//! the project length unit when it states none, and `Scale` takes project
//! lengths to map lengths. With a `MapUnit`, the map side is untouched and
//! `Scale` is divided by the length factor; without one, the map
//! coordinates follow the project unit and are rescaled. `IfcRigidOperation`
//! coordinates and height follow the same rule (the `HeightUnit` of a
//! geographic CRS for the height). IFC2X3's `ePSet_MapConversion` and
//! `ePSet_ProjectedCRS` property sets are read the way `ifc-georef` reads
//! them, with `ePSet_ProjectedCRS.MapUnit`.
//!
//! # Areas and volumes
//!
//! Areas are in the project `AREAUNIT`, not in the length unit squared, and
//! volumes in the `VOLUMEUNIT`. By default
//! ([`LengthUnitOptions::area_and_volume`]) both follow the length unit:
//! they are replaced by its square and cube (`SQUARE_METRE` with the same
//! prefix, or `square foot`), and their values are rescaled by the exact
//! ratio of old and new unit, which is 1 when a millimetre project kept its
//! areas in square metres and converts to metres. A project that assigns no
//! area or volume unit has none assigned afterwards either, and its area or
//! volume values are left as they are (reported as kept).
//!
//! # The transaction
//!
//! [`stage_length_unit_conversion`] plans everything against the model as it
//! stands and refuses with every reason at once
//! ([`LengthUnitError::Refused`]); nothing is staged until the whole model
//! is understood. [`convert_length_unit`] commits the plan as one
//! [`Transaction`](ifc_model::Transaction): all or nothing. The new units are
//! written with `ifc-properties`' unit writers; a unit the model already has
//! (an unprefixed `METRE` for a foot's conversion factor) is reused. The
//! replaced units are removed when nothing else refers to them
//! ([`LengthUnitOptions::remove_replaced_units`]).
//!
//! Converting millimetres to metres and back restores every value to within
//! one rounding of the original; units that differ only by prefixes convert
//! by an exact power of ten.
#![cfg(feature = "properties")]

mod classify;
mod georef;
mod report;
mod scope;
mod space;
mod target;
mod walk;

use ifc_model::Model;
use ifc_properties::{
    exact_project_unit, exact_schema, resolve_unit, unit_conversion, ConversionUnit, ExactUnitError,
};

pub use report::{
    ConvertedAttribute, ConvertedKind, KeptAttribute, KeptReason, LengthUnitError,
    LengthUnitOptions, LengthUnitRefusal, LengthUnitReport, RefusalReason,
    StagedLengthUnitConversion,
};

use report::Tally;
use target::TargetUnits;

/// Plan the conversion of `model`'s length unit to `target` and stage it on
/// one transaction, without committing.
///
/// `target` is a length unit: an SI `METRE` with any prefix
/// (`ConversionUnit::parse("MILLIMETRE")`) or a named length unit (`inch`,
/// `foot`, `yard`, `mile`, `US survey foot`). The staged report says what the
/// commit will change; a target equal to the current unit stages nothing.
///
/// # Errors
///
/// [`LengthUnitError::Target`] for a target that is not a length unit;
/// [`LengthUnitError::Schema`] for a release other than IFC2X3, IFC4 or
/// IFC4X3; [`LengthUnitError::ProjectUnit`] when the project's length, area
/// or volume unit cannot be resolved exactly; [`LengthUnitError::Refused`]
/// with every value the conversion cannot carry; and
/// [`LengthUnitError::Authoring`] if a new unit cannot be written.
pub fn stage_length_unit_conversion(
    model: &Model,
    target: &ConversionUnit,
    options: &LengthUnitOptions,
) -> Result<StagedLengthUnitConversion, LengthUnitError> {
    if !target.is_length() || target.offset != 0.0 {
        return Err(LengthUnitError::Target {
            target: target.name.clone(),
            reason: "not a length unit",
        });
    }
    target::require_writable(target)?;
    let schema = exact_schema(model).map_err(LengthUnitError::Schema)?;
    let old = project_unit(model, "LENGTHUNIT")?.ok_or_else(|| LengthUnitError::ProjectUnit {
        unit_type: "LENGTHUNIT".into(),
        error: ifc_properties::UnitConversionError::Resolve(ExactUnitError::NoProjectUnit {
            unit_type: "LENGTHUNIT".into(),
        }),
    })?;
    let (area, volume) = if options.area_and_volume {
        (
            project_unit(model, "AREAUNIT")?,
            project_unit(model, "VOLUMEUNIT")?,
        )
    } else {
        (None, None)
    };
    let units = TargetUnits::plan(target, &old.1, area.as_ref(), volume.as_ref())?;
    let mut tally = Tally::new(&old.1, target, &units);
    let mut transaction = ifc_model::Transaction::new(model);
    if units.is_identity() {
        return Ok(tally.staged(transaction));
    }
    walk::plan(model, schema, &units, &mut tally, &mut transaction)?;
    target::stage(
        model,
        &units,
        old.0,
        area,
        volume,
        options,
        &mut transaction,
        &mut tally,
    )?;
    Ok(tally.staged(transaction))
}

/// Convert `model`'s length unit to `target` and commit, all or nothing.
///
/// # Errors
///
/// As [`stage_length_unit_conversion`], and [`LengthUnitError::Conflict`]
/// when the staged transaction fails its preflight. The model is unchanged
/// on every error.
pub fn convert_length_unit(
    model: &mut Model,
    target: &ConversionUnit,
    options: &LengthUnitOptions,
) -> Result<LengthUnitReport, LengthUnitError> {
    let staged = stage_length_unit_conversion(model, target, options)?;
    let mut report = staged.report;
    if staged.transaction.is_empty() {
        return Ok(report);
    }
    let applied = staged
        .transaction
        .commit(model)
        .map_err(LengthUnitError::Conflict)?;
    report.created = applied.created;
    report.removed = applied.removed.into_iter().map(|(id, _)| id).collect();
    report.revision = Some(applied.revision);
    report.applied = true;
    Ok(report)
}

/// The project unit of `unit_type` and its conversion, `None` when the
/// project assigns none of that type.
fn project_unit(
    model: &Model,
    unit_type: &str,
) -> Result<Option<(ifc_model::EntityId, ConversionUnit)>, LengthUnitError> {
    let failed = |error| LengthUnitError::ProjectUnit {
        unit_type: unit_type.into(),
        error,
    };
    let id = match exact_project_unit(model, unit_type) {
        Ok(id) => id,
        Err(ExactUnitError::NoProjectUnit { .. }) if unit_type != "LENGTHUNIT" => return Ok(None),
        Err(error) => return Err(failed(error.into())),
    };
    let unit = resolve_unit(model, id).map_err(failed)?;
    // The unit must measure what its type says; the resolver checks
    // `IfcCorrectDimensions`, this checks the conversion is defined.
    unit_conversion(&unit, &unit).map_err(failed)?;
    Ok(Some((id, unit)))
}
