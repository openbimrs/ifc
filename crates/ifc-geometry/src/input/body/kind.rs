//! How a body representation item is modelled.
//!
//! The table below classifies every representation-item family the lowering
//! dispatcher (`lower::dispatch::IMPLEMENTED`) accepts, except
//! `IfcMappedItem`, which is not a kind: [`crate::body_description`] resolves
//! it to the items it maps. `tests/body_description.rs` fails if the two
//! lists drift, so a family cannot become lowerable without also becoming
//! describable.

use crate::select::is_a;

/// How one representation item models its shape.
///
/// Finer than [`crate::solid::SolidKind`], which groups every sweep as
/// `Swept`: a rule check that allows extrusions but not directrix sweeps needs
/// the distinction. The concrete IFC type is reported beside the kind, so a
/// caller that needs more detail than the kind states can read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BodyKind {
    /// `IfcExtrudedAreaSolid`: a profile swept along a straight direction.
    Extrusion,
    /// `IfcExtrudedAreaSolidTapered`: an extrusion lofted to an end profile.
    TaperedExtrusion,
    /// `IfcRevolvedAreaSolid`: a profile swept about an axis.
    Revolution,
    /// `IfcRevolvedAreaSolidTapered`: a revolution lofted to an end profile.
    TaperedRevolution,
    /// `IfcFixedReferenceSweptAreaSolid` or `IfcSurfaceCurveSweptAreaSolid`:
    /// a profile swept along a directrix curve.
    DirectrixSweep,
    /// `IfcSweptDiskSolid` or `IfcSweptDiskSolidPolygonal`: a circular disk
    /// swept along a curve.
    SweptDisk,
    /// `IfcSectionedSpine`: cross sections interpolated along a spine.
    SectionedSpine,
    /// A faceted or advanced boundary representation, with or without voids.
    Brep,
    /// `IfcCsgSolid`, `IfcBooleanResult` or `IfcBooleanClippingResult`: a
    /// boolean tree.
    Csg,
    /// A CSG primitive: `IfcBlock`, `IfcSphere`, `IfcRightCircularCylinder`,
    /// `IfcRightCircularCone` or `IfcRectangularPyramid`.
    CsgPrimitive,
    /// A half space. Infinite; only meaningful as a boolean operand.
    HalfSpace,
    /// `IfcBoundingBox`: a proxy extent, not a shape.
    BoundingBox,
    /// `IfcTriangulatedFaceSet` or `IfcPolygonalFaceSet`.
    Tessellated,
    /// `IfcShellBasedSurfaceModel` or `IfcFaceBasedSurfaceModel`. Never a
    /// solid, even when every shell is closed.
    SurfaceModel,
    /// `IfcFaceSurface` or `IfcAdvancedFace`: one bounded face on a carrier
    /// surface. Never a solid: a single face encloses nothing.
    Face,
    /// `IfcGeometricSet` or `IfcGeometricCurveSet`.
    GeometricSet,
    /// Any `IfcCurve`.
    Curve,
    /// Any `IfcSurface`.
    Surface,
    /// `IfcPointOnCurve` or `IfcPointOnSurface`.
    Point,
}

/// Concrete families by kind. Curves and surfaces are classified by
/// inheritance in [`BodyKind::classify`] instead, as the dispatcher does.
const FAMILIES: &[(&str, BodyKind)] = &[
    ("IFCEXTRUDEDAREASOLID", BodyKind::Extrusion),
    ("IFCEXTRUDEDAREASOLIDTAPERED", BodyKind::TaperedExtrusion),
    ("IFCREVOLVEDAREASOLID", BodyKind::Revolution),
    ("IFCREVOLVEDAREASOLIDTAPERED", BodyKind::TaperedRevolution),
    ("IFCFIXEDREFERENCESWEPTAREASOLID", BodyKind::DirectrixSweep),
    ("IFCSURFACECURVESWEPTAREASOLID", BodyKind::DirectrixSweep),
    ("IFCSWEPTDISKSOLID", BodyKind::SweptDisk),
    ("IFCSWEPTDISKSOLIDPOLYGONAL", BodyKind::SweptDisk),
    ("IFCSECTIONEDSPINE", BodyKind::SectionedSpine),
    ("IFCFACETEDBREP", BodyKind::Brep),
    ("IFCFACETEDBREPWITHVOIDS", BodyKind::Brep),
    ("IFCADVANCEDBREP", BodyKind::Brep),
    ("IFCADVANCEDBREPWITHVOIDS", BodyKind::Brep),
    ("IFCCSGSOLID", BodyKind::Csg),
    ("IFCBOOLEANRESULT", BodyKind::Csg),
    ("IFCBOOLEANCLIPPINGRESULT", BodyKind::Csg),
    ("IFCBLOCK", BodyKind::CsgPrimitive),
    ("IFCSPHERE", BodyKind::CsgPrimitive),
    ("IFCRIGHTCIRCULARCYLINDER", BodyKind::CsgPrimitive),
    ("IFCRIGHTCIRCULARCONE", BodyKind::CsgPrimitive),
    ("IFCRECTANGULARPYRAMID", BodyKind::CsgPrimitive),
    ("IFCHALFSPACESOLID", BodyKind::HalfSpace),
    ("IFCBOXEDHALFSPACE", BodyKind::HalfSpace),
    ("IFCPOLYGONALBOUNDEDHALFSPACE", BodyKind::HalfSpace),
    ("IFCBOUNDINGBOX", BodyKind::BoundingBox),
    ("IFCTRIANGULATEDFACESET", BodyKind::Tessellated),
    // IFC4X3 subtype; lowered through `lower::dispatch::SPECIALISATIONS`.
    ("IFCTRIANGULATEDIRREGULARNETWORK", BodyKind::Tessellated),
    ("IFCPOLYGONALFACESET", BodyKind::Tessellated),
    ("IFCSHELLBASEDSURFACEMODEL", BodyKind::SurfaceModel),
    ("IFCFACEBASEDSURFACEMODEL", BodyKind::SurfaceModel),
    ("IFCFACESURFACE", BodyKind::Face),
    ("IFCADVANCEDFACE", BodyKind::Face),
    ("IFCGEOMETRICSET", BodyKind::GeometricSet),
    ("IFCGEOMETRICCURVESET", BodyKind::GeometricSet),
    ("IFCPOINTONCURVE", BodyKind::Point),
    ("IFCPOINTONSURFACE", BodyKind::Point),
];

impl BodyKind {
    /// Classify a representation item's IFC type name, case-insensitively.
    ///
    /// `None` for `IfcMappedItem` (resolved, not classified) and for anything
    /// that is not a representation item family this crate interprets.
    pub fn classify(type_name: &str) -> Option<Self> {
        let upper = type_name.to_ascii_uppercase();
        // Inheritance first, exactly as `lower::dispatch` routes: every
        // IfcCurve and IfcSurface subtype is a valid item of its own.
        if is_a(&upper, "IFCCURVE") {
            return Some(Self::Curve);
        }
        if is_a(&upper, "IFCSURFACE") {
            return Some(Self::Surface);
        }
        FAMILIES
            .iter()
            .find(|(name, _)| *name == upper)
            .map(|(_, kind)| *kind)
    }

    /// Is this a swept-area family, whose profile [`crate::SweptSolid`]
    /// describes?
    pub fn is_swept_area(self) -> bool {
        matches!(
            self,
            Self::Extrusion
                | Self::TaperedExtrusion
                | Self::Revolution
                | Self::TaperedRevolution
                | Self::DirectrixSweep
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::SolidKind;

    /// The finer kind never contradicts the coarser solid classifier.
    #[test]
    fn body_kinds_refine_solid_kinds() {
        for (name, kind) in FAMILIES {
            let Some(solid) = SolidKind::classify(name) else {
                continue;
            };
            let expected = match kind {
                BodyKind::Extrusion
                | BodyKind::TaperedExtrusion
                | BodyKind::Revolution
                | BodyKind::TaperedRevolution
                | BodyKind::DirectrixSweep
                | BodyKind::SweptDisk => SolidKind::Swept,
                BodyKind::SectionedSpine => SolidKind::SectionedSpine,
                BodyKind::Brep => SolidKind::Brep,
                BodyKind::Csg if *name == "IFCCSGSOLID" => SolidKind::Csg,
                BodyKind::Csg => SolidKind::Boolean,
                BodyKind::CsgPrimitive => SolidKind::Csg,
                BodyKind::HalfSpace => SolidKind::HalfSpace,
                BodyKind::BoundingBox => SolidKind::BoundingBox,
                BodyKind::Tessellated => SolidKind::Tessellated,
                BodyKind::SurfaceModel | BodyKind::GeometricSet => SolidKind::SurfaceModel,
                other => panic!("{name} classified {other:?} is not a solid family"),
            };
            assert_eq!(solid, expected, "{name}");
        }
    }

    #[test]
    fn curves_and_surfaces_classify_by_inheritance() {
        assert_eq!(BodyKind::classify("IfcTrimmedCurve"), Some(BodyKind::Curve));
        assert_eq!(BodyKind::classify("IFCPLANE"), Some(BodyKind::Surface));
        // IfcFaceSurface is an IfcFace, not an IfcSurface, in every release.
        assert_eq!(BodyKind::classify("IfcFaceSurface"), Some(BodyKind::Face));
        assert_eq!(BodyKind::classify("IFCADVANCEDFACE"), Some(BodyKind::Face));
        assert_eq!(BodyKind::classify("IFCMAPPEDITEM"), None);
        assert_eq!(BodyKind::classify("IFCWALL"), None);
    }
}
