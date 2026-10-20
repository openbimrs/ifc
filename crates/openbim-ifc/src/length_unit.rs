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
#[cfg(feature = "properties")]
mod classify;
#[cfg(feature = "properties")]
mod georef;
#[cfg(feature = "properties")]
mod ops;
#[cfg(feature = "properties")]
mod report;
#[cfg(feature = "properties")]
mod scope;
#[cfg(feature = "properties")]
mod space;
#[cfg(feature = "properties")]
mod target;
#[cfg(feature = "properties")]
mod walk;

#[cfg(feature = "properties")]
pub use ops::{convert_length_unit, stage_length_unit_conversion};
#[cfg(feature = "properties")]
pub use report::{
    ConvertedAttribute, ConvertedKind, KeptAttribute, KeptReason, LengthUnitError,
    LengthUnitOptions, LengthUnitRefusal, LengthUnitReport, RefusalReason,
    StagedLengthUnitConversion,
};
