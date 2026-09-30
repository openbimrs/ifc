//! `IfcSite` reference point (`RefLatitude`, `RefLongitude`, `RefElevation`)
//! and how its elevation relates to `IfcMapConversion.OrthogonalHeight`.
//!
//! # What each value means
//!
//! Quoted from the IFC4 ADD2 TC1 HTML documentation (the local references
//! checkout's `specs/ifc4-add2-tc1/dist/ifc4-add2-tc1/html/schema/`, pages
//! `ifcproductextension/lexical/ifcsite.htm` and
//! `ifcrepresentationresource/lexical/ifcmapconversion.htm`,
//! `ifccoordinatereferencesystem.htm`). No IFC2X3 or IFC4X3 HTML is checked
//! out; the IFC4 text is applied to both because the EXPRESS declarations
//! below are identical:
//!
//! - `IfcSite`: "If asserted, the Longitude, Latitude and Elevation establish
//!   the point in WGS84 where the point 0.,0.,0. of the LocalPlacement of
//!   IfcSite is situated." They are "provided for upward compatibility
//!   reasons"; "for exact georeferencing ... IfcCoordinateReferenceSystem
//!   and IfcMapConversion have to be used".
//! - `RefElevation : OPTIONAL IfcLengthMeasure`: "Datum elevation relative
//!   to sea level", "given according to the height datum used at this
//!   location". It names no datum. As an `IfcLengthMeasure` it is in the
//!   project length unit.
//! - `RefLatitude`/`RefLongitude : OPTIONAL IfcCompoundPlaneAngleMeasure`:
//!   "integer values for degrees, minutes, seconds, and, optionally,
//!   millionths of seconds with respect to the world geodetic system WGS84";
//!   latitudes north of the equator "from 0 till +90", south "from 0 till
//!   -90"; longitudes west of the zero meridian "from 0 till -180", east of
//!   it positive.
//! - `IfcMapConversion.OrthogonalHeight : IfcLengthMeasure`: "Orthogonal
//!   height relativ\[e\] to the vertical datum specified", locating the origin
//!   of the local engineering (world) coordinate system -- not the site
//!   origin. This crate reads it in the target CRS's map unit, as
//!   [`crate::ProjectToMap`] documents.
//! - `IfcCoordinateReferenceSystem.VerticalDatum : OPTIONAL IfcIdentifier`
//!   (declared on `IfcProjectedCRS` itself in IFC4X3): "the reference plane
//!   and fundamental point defining the origin of a height system"; it may be
//!   omitted when `Name` already identifies it (for example `EPSG:5555`).
//!
//! The attribute declarations are identical in `IFC2X3_TC1.exp`, `IFC4.exp`
//! and `IFC4X3_ADD2.exp`; IFC2X3 has no map conversion or CRS at all.
//!
//! # The comparison rule
//!
//! Both heights describe the same physical point only once the site origin
//! is carried through the map conversion, so [`relate_site_elevation`]
//! compares `RefElevation` (metres) with the map height of the site origin:
//! the `z` of `ProjectToMap.transform` applied to the site origin, which is
//! `OrthogonalHeight` plus the scaled height of the site origin in the world
//! coordinate system. That height is placement resolution, which this crate
//! does not do, so the caller supplies it (`0.0` for a site placed at the
//! world origin).
//!
//! `|RefElevation - map height| <= tolerance` is
//! [`SiteElevationCheck::Consistent`]; anything larger is
//! [`SiteElevationCheck::Disagreement`]. Neither value is ever preferred:
//! both, their difference, the tolerance and the target CRS's stated
//! `VerticalDatum` are returned. `IfcSite` names no datum, so agreement is
//! meaningful only when that vertical datum is the local sea-level datum;
//! the identifier is reported verbatim (`None` when unstated) for the caller
//! to judge, never interpreted. [`SITE_ELEVATION_TOLERANCE_M`] is the
//! suggested tolerance.
//!
//! # Releases
//!
//! IFC2X3, IFC4 (ADD2 TC1) and IFC4X3 are bound from the header; IFC4X1,
//! IFC4X2 and unknown tokens are refused with
//! [`crate::GeorefError::UnsupportedSchema`], as elsewhere in the crate.
//! The compound-angle WHERE rules differ per release and are checked with
//! the declared release's own rules (`angle.rs`).
//!
//! ## Internal split
//!
//! - `angle.rs`: `IfcCompoundPlaneAngleMeasure` validation and conversion.
//! - `reference.rs`: reading `IfcSite`'s reference attributes.
//! - `elevation.rs`: the `RefElevation`/`OrthogonalHeight` comparison.

mod angle;
mod elevation;
mod reference;

pub use angle::CompoundPlaneAngle;
pub use elevation::{
    relate_site_elevation, SiteElevationCheck, SiteElevationComparison, SITE_ELEVATION_TOLERANCE_M,
};
pub use reference::{site_reference, SiteReference};
