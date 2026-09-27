//! `IfcGeometricModelResource`: solids, tessellation and booleans.
//!
//! # What this module is
//!
//! Read-only typed views over the 42 entities of the IFC4 ADD2 TC1
//! `IfcGeometricModelResource` schema. Each view is a newtype over
//! `(EntityId, &Entity)` with named accessors and a `mod slot` block citing the
//! EXPRESS declaration. Views borrow and own nothing, so constructing one is
//! free and the model remains the single source of truth.
//!
//! # What this module deliberately does NOT do
//!
//! It does not triangulate, evaluate booleans, build meshes, or resolve units.
//! Those belong to a geometry kernel and to [`crate::units`]. A view's job is
//! to say faithfully what the file contains, including the parts a naive reader
//! gets wrong.
//!
//! # The three counting conventions that must not be confused
//!
//! 1. **Attribute slots are absolute.** Inherited attributes come first, so
//!    `IfcExtrudedAreaSolid.SweptArea` is slot 0 (inherited from
//!    `IfcSweptAreaSolid`) and `Depth` is slot 3. Local, per-subtype numbering
//!    is never used anywhere in this module.
//! 2. **Vertex indices are 1-based.** `CoordIndex`, `PnIndex` and
//!    `InnerCoordIndices` all count from 1. See [`tessellated`].
//! 3. **Measures are in file units.** Lengths and angles are returned raw;
//!    `IfcRevolvedAreaSolid.Angle` is very often in degrees.
//!
//! # The map from schema to module
//!
//! | Module | Entities |
//! | --- | --- |
//! | [`swept`] | the 9 swept solids plus `IfcSectionedSpine` |
//! | [`brep`] | the 5 `IfcManifoldSolidBrep` types |
//! | [`csg`] | `IfcCsgSolid` plus the 6 primitive types |
//! | [`halfspace`] | the 3 half spaces |
//! | [`boolean`] | `IfcBooleanResult`, `IfcBooleanClippingResult` |
//! | [`tessellated`] | the 6 tessellation types |
//! | [`surface_model`] | the 2 surface models and 2 geometric sets |
//! | [`bbox`] | `IfcBoundingBox` |
//!
//! # Volume semantics, in one place
//!
//! Only some of these entities enclose a volume, and confusing the categories
//! is the most consequential mistake a consumer can make:
//!
//! - **Finite solids**: swept solids, breps, CSG solids and primitives.
//! - **Infinite**: `IfcHalfSpaceSolid` and its subtypes. Only ever a boolean
//!   operand; never tessellate one on its own. See [`halfspace`].
//! - **Conditionally solid**: the tessellated face sets, but only when
//!   `Closed` is TRUE.
//! - **Never solid**: the surface models and geometric sets in
//!   [`surface_model`], and `IfcBoundingBox`, which is a proxy extent.
//!
//! [`SolidKind::classify`] turns that table into a dispatch a caller can use
//! rather than re-deriving from type names.

pub mod bbox;
pub mod boolean;
pub mod brep;
pub mod csg;
pub mod halfspace;
pub mod surface_model;
pub mod swept;
pub mod tessellated;

#[cfg(test)]
pub(crate) mod testkit;

pub use bbox::BoundingBox;
pub use boolean::{
    BooleanClippingResult, BooleanOperator, BooleanResult, IfcBooleanOperator, OperandKind,
    ParseIfcBooleanOperatorError,
};
pub use brep::{
    AdvancedBrep, AdvancedBrepWithVoids, FacetedBrep, FacetedBrepWithVoids, ManifoldSolidBrep,
};
pub use csg::{
    Block, CsgPrimitive3D, CsgSolid, RectangularPyramid, RightCircularCone, RightCircularCylinder,
    Sphere,
};
pub use halfspace::{BoxedHalfSpace, HalfSpaceSolid, PolygonalBoundedHalfSpace};
pub use surface_model::{
    FaceBasedSurfaceModel, GeometricCurveSet, GeometricSet, ShellBasedSurfaceModel,
};
pub use swept::{
    ExtrudedAreaSolid, ExtrudedAreaSolidTapered, FixedReferenceSweptAreaSolid, RevolvedAreaSolid,
    RevolvedAreaSolidTapered, SectionedSpine, SurfaceCurveSweptAreaSolid, SweptAreaSolid,
    SweptDiskSolid, SweptDiskSolidPolygonal,
};
pub use tessellated::{
    IndexedPolygonalFace, IndexedPolygonalFaceWithVoids, PolygonalFaceSet, TessellatedFaceSet,
    TessellatedItem, TriangulatedFaceSet,
};

/// Which family of `IfcGeometricModelResource` an entity belongs to.
///
/// # Why classify at all
///
/// A representation item arrives as an untyped entity, and the correct handling
/// differs by family in ways that are not recoverable later: a half space must
/// be routed into a boolean, a boolean must be walked as a tree, a face set
/// must have its 1-based indices converted. Deciding once, by name, keeps that
/// dispatch out of every call site and keeps the volume semantics table in a
/// single place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidKind {
    /// A swept solid: extrusion, revolution, directrix sweep or swept disk.
    Swept,
    /// A boundary representation.
    Brep,
    /// A CSG solid or one of the analytic primitives.
    Csg,
    /// A half space. **Infinite**; only valid as a boolean operand.
    HalfSpace,
    /// A boolean result. Its operands form a tree and may nest.
    Boolean,
    /// A tessellated face set. Solid only when its `Closed` flag is TRUE.
    Tessellated,
    /// A surface model or geometric set. Never encloses a volume.
    SurfaceModel,
    /// An `IfcBoundingBox`: a proxy extent, not geometry to build.
    BoundingBox,
    /// `IfcSectionedSpine`: cross sections along a spine.
    ///
    /// Kept apart from [`Self::Swept`] because it subtypes
    /// `IfcGeometricRepresentationItem` directly, has a different slot layout,
    /// and needs interpolation a plain sweep does not.
    SectionedSpine,
}

impl SolidKind {
    /// Classify an IFC type name, case-insensitively.
    ///
    /// Returns `None` for anything outside this schema, so a caller can pass
    /// any representation item and route the rest elsewhere.
    pub fn classify(type_name: &str) -> Option<Self> {
        let n = type_name.to_ascii_uppercase();
        let kind = match n.as_str() {
            "IFCSWEPTAREASOLID"
            | "IFCEXTRUDEDAREASOLID"
            | "IFCEXTRUDEDAREASOLIDTAPERED"
            | "IFCREVOLVEDAREASOLID"
            | "IFCREVOLVEDAREASOLIDTAPERED"
            | "IFCSURFACECURVESWEPTAREASOLID"
            | "IFCFIXEDREFERENCESWEPTAREASOLID"
            | "IFCSWEPTDISKSOLID"
            | "IFCSWEPTDISKSOLIDPOLYGONAL" => Self::Swept,
            "IFCSECTIONEDSPINE" => Self::SectionedSpine,
            "IFCMANIFOLDSOLIDBREP"
            | "IFCFACETEDBREP"
            | "IFCFACETEDBREPWITHVOIDS"
            | "IFCADVANCEDBREP"
            | "IFCADVANCEDBREPWITHVOIDS" => Self::Brep,
            "IFCCSGSOLID"
            | "IFCCSGPRIMITIVE3D"
            | "IFCBLOCK"
            | "IFCRECTANGULARPYRAMID"
            | "IFCRIGHTCIRCULARCONE"
            | "IFCRIGHTCIRCULARCYLINDER"
            | "IFCSPHERE" => Self::Csg,
            "IFCHALFSPACESOLID" | "IFCBOXEDHALFSPACE" | "IFCPOLYGONALBOUNDEDHALFSPACE" => {
                Self::HalfSpace
            }
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => Self::Boolean,
            "IFCTESSELLATEDITEM"
            | "IFCTESSELLATEDFACESET"
            | "IFCTRIANGULATEDFACESET"
            | "IFCPOLYGONALFACESET"
            | "IFCINDEXEDPOLYGONALFACE"
            | "IFCINDEXEDPOLYGONALFACEWITHVOIDS" => Self::Tessellated,
            "IFCSHELLBASEDSURFACEMODEL"
            | "IFCFACEBASEDSURFACEMODEL"
            | "IFCGEOMETRICSET"
            | "IFCGEOMETRICCURVESET" => Self::SurfaceModel,
            "IFCBOUNDINGBOX" => Self::BoundingBox,
            _ => return None,
        };
        Some(kind)
    }

    /// Does this family enclose a finite volume on its own?
    ///
    /// `false` for half spaces (infinite), booleans (depends on the operands),
    /// tessellated sets (depends on `Closed`), surface models and bounding
    /// boxes. A `true` here is the only safe licence to compute a volume
    /// without further checks.
    pub fn is_finite_solid(self) -> bool {
        matches!(
            self,
            Self::Swept | Self::Brep | Self::Csg | Self::SectionedSpine
        )
    }

    /// Must this be routed through a boolean rather than built directly?
    ///
    /// `true` only for half spaces. Anything else may be built standalone,
    /// though not everything built will have a volume.
    pub fn requires_boolean_context(self) -> bool {
        matches!(self, Self::HalfSpace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every entity this module claims to cover must classify. The list is the
    /// module's contract against the schema, so a gap here is a gap in the
    /// coverage claim.
    #[test]
    fn every_geometric_model_resource_entity_classifies() {
        let expected: &[(&str, SolidKind)] = &[
            // swept: 10
            ("IfcSweptAreaSolid", SolidKind::Swept),
            ("IfcExtrudedAreaSolid", SolidKind::Swept),
            ("IfcExtrudedAreaSolidTapered", SolidKind::Swept),
            ("IfcRevolvedAreaSolid", SolidKind::Swept),
            ("IfcRevolvedAreaSolidTapered", SolidKind::Swept),
            ("IfcSurfaceCurveSweptAreaSolid", SolidKind::Swept),
            ("IfcFixedReferenceSweptAreaSolid", SolidKind::Swept),
            ("IfcSweptDiskSolid", SolidKind::Swept),
            ("IfcSweptDiskSolidPolygonal", SolidKind::Swept),
            ("IfcSectionedSpine", SolidKind::SectionedSpine),
            // brep: 5
            ("IfcManifoldSolidBrep", SolidKind::Brep),
            ("IfcFacetedBrep", SolidKind::Brep),
            ("IfcFacetedBrepWithVoids", SolidKind::Brep),
            ("IfcAdvancedBrep", SolidKind::Brep),
            ("IfcAdvancedBrepWithVoids", SolidKind::Brep),
            // csg: 7
            ("IfcCsgSolid", SolidKind::Csg),
            ("IfcCsgPrimitive3D", SolidKind::Csg),
            ("IfcBlock", SolidKind::Csg),
            ("IfcRectangularPyramid", SolidKind::Csg),
            ("IfcRightCircularCone", SolidKind::Csg),
            ("IfcRightCircularCylinder", SolidKind::Csg),
            ("IfcSphere", SolidKind::Csg),
            // halfspace: 3
            ("IfcHalfSpaceSolid", SolidKind::HalfSpace),
            ("IfcBoxedHalfSpace", SolidKind::HalfSpace),
            ("IfcPolygonalBoundedHalfSpace", SolidKind::HalfSpace),
            // boolean: 2
            ("IfcBooleanResult", SolidKind::Boolean),
            ("IfcBooleanClippingResult", SolidKind::Boolean),
            // tessellated: 6
            ("IfcTessellatedItem", SolidKind::Tessellated),
            ("IfcTessellatedFaceSet", SolidKind::Tessellated),
            ("IfcTriangulatedFaceSet", SolidKind::Tessellated),
            ("IfcPolygonalFaceSet", SolidKind::Tessellated),
            ("IfcIndexedPolygonalFace", SolidKind::Tessellated),
            ("IfcIndexedPolygonalFaceWithVoids", SolidKind::Tessellated),
            // surface models and sets: 4
            ("IfcShellBasedSurfaceModel", SolidKind::SurfaceModel),
            ("IfcFaceBasedSurfaceModel", SolidKind::SurfaceModel),
            ("IfcGeometricSet", SolidKind::SurfaceModel),
            ("IfcGeometricCurveSet", SolidKind::SurfaceModel),
            // bbox: 1
            ("IfcBoundingBox", SolidKind::BoundingBox),
        ];

        for (name, kind) in expected {
            assert_eq!(
                SolidKind::classify(name),
                Some(*kind),
                "{name} must classify"
            );
            // STEP is case-insensitive, and files are written upper-cased.
            assert_eq!(
                SolidKind::classify(&name.to_ascii_uppercase()),
                Some(*kind),
                "{name} must classify upper-cased"
            );
        }

        // 38 concrete plus abstract types here; the schema's 42 entities also
        // include IfcSolidModel and the abstract roots handled by their
        // subtypes, plus IfcCartesianPointList2D/3D which belong to
        // IfcGeometryResource in this crate's split.
        assert_eq!(expected.len(), 38, "coverage list must not shrink silently");
    }

    #[test]
    fn entities_outside_this_schema_do_not_classify() {
        for name in ["IfcWall", "IfcPolyline", "IfcCartesianPoint", "IfcPlane"] {
            assert_eq!(SolidKind::classify(name), None, "{name}");
        }
    }

    /// The volume-semantics table from the module docs, as an assertion.
    #[test]
    fn only_genuinely_bounded_families_report_a_finite_volume() {
        for kind in [
            SolidKind::Swept,
            SolidKind::Brep,
            SolidKind::Csg,
            SolidKind::SectionedSpine,
        ] {
            assert!(kind.is_finite_solid(), "{kind:?}");
        }
        for kind in [
            SolidKind::HalfSpace,
            SolidKind::Boolean,
            SolidKind::Tessellated,
            SolidKind::SurfaceModel,
            SolidKind::BoundingBox,
        ] {
            assert!(!kind.is_finite_solid(), "{kind:?}");
        }
    }

    /// A half space is the only thing that cannot stand alone at all.
    #[test]
    fn only_half_spaces_require_a_boolean_context() {
        assert!(SolidKind::HalfSpace.requires_boolean_context());
        for kind in [
            SolidKind::Swept,
            SolidKind::Brep,
            SolidKind::Csg,
            SolidKind::Boolean,
            SolidKind::Tessellated,
            SolidKind::SurfaceModel,
            SolidKind::BoundingBox,
            SolidKind::SectionedSpine,
        ] {
            assert!(!kind.requires_boolean_context(), "{kind:?}");
        }
    }

    /// IfcSectionedSpine is a swept-ish solid with a different slot layout, so
    /// it must not be lumped in with the IfcSweptAreaSolid branch.
    #[test]
    fn sectioned_spine_is_classified_apart_from_the_swept_area_family() {
        assert_ne!(
            SolidKind::classify("IfcSectionedSpine"),
            SolidKind::classify("IfcExtrudedAreaSolid")
        );
    }
}
