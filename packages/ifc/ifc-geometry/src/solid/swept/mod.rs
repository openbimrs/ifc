//! Swept solids: a cross section dragged along a path.
//!
//! # The family
//!
//! `IfcSweptAreaSolid` sweeps an `IfcProfileDef`; `IfcSweptDiskSolid` sweeps a
//! circular disk along a curve. `IfcSectionedSpine` is neither -- it
//! interpolates between explicit cross sections along a spine and is a direct
//! `IfcGeometricRepresentationItem`, not an `IfcSolidModel`.
//!
//! # Why the profile is not resolved here
//!
//! `SweptArea` is an `IfcProfileDef` reference and profiles are a separate
//! resource schema owned by another module. Returning the `EntityId` keeps this
//! module a transcription of the file; the profile layer decides what a profile
//! means.
//!
//! # Units
//!
//! Every length and angle here is in the **file's** units, not metres or
//! radians. Converting is [`crate::units`]'s job and must happen exactly once,
//! at lowering time. Doing it in an accessor would double-apply the scale for
//! any caller that also converts.

pub mod area;
pub mod directrix;

pub use area::{
    ExtrudedAreaSolid, ExtrudedAreaSolidTapered, RevolvedAreaSolid, RevolvedAreaSolidTapered,
    SweptAreaSolid,
};
pub use directrix::{
    FixedReferenceSweptAreaSolid, SectionedSpine, SurfaceCurveSweptAreaSolid, SweptDiskSolid,
    SweptDiskSolidPolygonal,
};

/// `IfcSweptAreaSolid` attribute slots, inherited by every subtype.
///
/// EXPRESS (IFC4 ADD2 TC1): `IfcSweptAreaSolid` declares `SweptArea` then
/// `Position`; its supertype `IfcSolidModel` declares no explicit attributes,
/// so these are absolute slots 0 and 1 for the whole family.
mod swept_area_slot {
    /// `SweptArea : IfcProfileDef`, declared on `IfcSweptAreaSolid`.
    pub const SWEPT_AREA: usize = 0;
    /// `Position : OPTIONAL IfcAxis2Placement3D`, on `IfcSweptAreaSolid`.
    pub const POSITION: usize = 1;
}

/// `IfcExtrudedAreaSolid` own slots, after the two inherited ones.
///
/// EXPRESS: `ExtrudedDirection : IfcDirection`, `Depth :
/// IfcPositiveLengthMeasure`; then `EndSweptArea` on the tapered subtype.
mod extruded_slot {
    /// `ExtrudedDirection`, absolute slot 2.
    pub const EXTRUDED_DIRECTION: usize = 2;
    /// `Depth`, absolute slot 3.
    pub const DEPTH: usize = 3;
    /// `EndSweptArea` on `IfcExtrudedAreaSolidTapered`, absolute slot 4.
    pub const END_SWEPT_AREA: usize = 4;
}

/// `IfcRevolvedAreaSolid` own slots.
///
/// EXPRESS: `Axis : IfcAxis1Placement`, `Angle : IfcPlaneAngleMeasure`; then
/// `EndSweptArea` on the tapered subtype.
mod revolved_slot {
    /// `Axis`, absolute slot 2.
    pub const AXIS: usize = 2;
    /// `Angle`, absolute slot 3.
    pub const ANGLE: usize = 3;
    /// `EndSweptArea` on `IfcRevolvedAreaSolidTapered`, absolute slot 4.
    pub const END_SWEPT_AREA: usize = 4;
}

/// Directrix-driven swept area slots.
///
/// EXPRESS: `IfcSurfaceCurveSweptAreaSolid` and
/// `IfcFixedReferenceSweptAreaSolid` both declare `Directrix`, `StartParam`,
/// `EndParam` in that order after the inherited pair, then differ in slot 5.
mod directrix_slot {
    /// `Directrix : IfcCurve`, absolute slot 2.
    pub const DIRECTRIX: usize = 2;
    /// `StartParam : OPTIONAL IfcParameterValue`, absolute slot 3.
    pub const START_PARAM: usize = 3;
    /// `EndParam : OPTIONAL IfcParameterValue`, absolute slot 4.
    pub const END_PARAM: usize = 4;
    /// `ReferenceSurface : IfcSurface` on `IfcSurfaceCurveSweptAreaSolid`.
    pub const REFERENCE_SURFACE: usize = 5;
    /// `FixedReference : IfcDirection` on `IfcFixedReferenceSweptAreaSolid`.
    pub const FIXED_REFERENCE: usize = 5;
}

/// `IfcSweptDiskSolid` slots.
///
/// EXPRESS: it subtypes `IfcSolidModel` directly, so `Directrix` is slot 0 --
/// there is no inherited `SweptArea` here despite the family resemblance.
mod disk_slot {
    /// `Directrix : IfcCurve`, absolute slot 0.
    pub const DIRECTRIX: usize = 0;
    /// `Radius : IfcPositiveLengthMeasure`, absolute slot 1.
    pub const RADIUS: usize = 1;
    /// `InnerRadius : OPTIONAL IfcPositiveLengthMeasure`, absolute slot 2.
    pub const INNER_RADIUS: usize = 2;
    /// `StartParam : OPTIONAL IfcParameterValue`, absolute slot 3.
    pub const START_PARAM: usize = 3;
    /// `EndParam : OPTIONAL IfcParameterValue`, absolute slot 4.
    pub const END_PARAM: usize = 4;
    /// `FilletRadius` on `IfcSweptDiskSolidPolygonal`, absolute slot 5.
    pub const FILLET_RADIUS: usize = 5;
}

/// `IfcSectionedSpine` slots.
///
/// EXPRESS: subtypes `IfcGeometricRepresentationItem`, which declares no
/// explicit attributes, so all three slots are its own.
mod spine_slot {
    /// `SpineCurve : IfcCompositeCurve`, absolute slot 0.
    pub const SPINE_CURVE: usize = 0;
    /// `CrossSections : LIST [2:?] OF IfcProfileDef`, absolute slot 1.
    pub const CROSS_SECTIONS: usize = 1;
    /// `CrossSectionPositions : LIST [2:?] OF IfcAxis2Placement3D`, slot 2.
    pub const CROSS_SECTION_POSITIONS: usize = 2;
}
