//! The inventory gate: is the schema coverage claim actually true?
//!
//! # Why this test exists
//!
//! "We support the IFC geometry resources" is easy to say and hard to verify.
//! Test counts do not prove it: 300 tests can all exercise the same ten
//! entities. This test enumerates **every concrete entity** in the three
//! geometry schemas and fails if the crate has no view for one.
//!
//! The list is generated from the normative EXPRESS source
//! (`references/ifc-spec/ifc4-add2-tc1/IFC4.exp`), not written by hand, so it
//! cannot drift toward what happens to be implemented.
//!
//! # What "covered" means here
//!
//! That the crate names the entity: a typed view exists, or a dispatcher
//! recognises it. It does NOT mean the geometry is fully evaluated -- that is
//! a kernel's job, and this crate deliberately stops at interpretation. A view
//! that reads an entity's attributes and reports `Unsupported` for the parts
//! needing a kernel still counts, because the IFC side is done.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Every concrete (non-ABSTRACT) entity in IfcGeometryResource,
/// IfcGeometricModelResource and IfcGeometricConstraintResource.
///
/// Generated from IFC4 ADD2 TC1. 89 entities.
const CONCRETE_ENTITIES: &[&str] = &[
    "IfcAdvancedBrep",
    "IfcAdvancedBrepWithVoids",
    "IfcAxis1Placement",
    "IfcAxis2Placement2D",
    "IfcAxis2Placement3D",
    "IfcBSplineCurveWithKnots",
    "IfcBSplineSurfaceWithKnots",
    "IfcBlock",
    "IfcBooleanClippingResult",
    "IfcBooleanResult",
    "IfcBoundaryCurve",
    "IfcBoundingBox",
    "IfcBoxedHalfSpace",
    "IfcCartesianPoint",
    "IfcCartesianPointList2D",
    "IfcCartesianPointList3D",
    "IfcCartesianTransformationOperator2D",
    "IfcCartesianTransformationOperator2DnonUniform",
    "IfcCartesianTransformationOperator3D",
    "IfcCartesianTransformationOperator3DnonUniform",
    "IfcCircle",
    "IfcCompositeCurve",
    "IfcCompositeCurveOnSurface",
    "IfcCompositeCurveSegment",
    "IfcConnectionCurveGeometry",
    "IfcConnectionPointEccentricity",
    "IfcConnectionPointGeometry",
    "IfcConnectionSurfaceGeometry",
    "IfcConnectionVolumeGeometry",
    "IfcCsgSolid",
    "IfcCurveBoundedPlane",
    "IfcCurveBoundedSurface",
    "IfcCylindricalSurface",
    "IfcDirection",
    "IfcEllipse",
    "IfcExtrudedAreaSolid",
    "IfcExtrudedAreaSolidTapered",
    "IfcFaceBasedSurfaceModel",
    "IfcFacetedBrep",
    "IfcFacetedBrepWithVoids",
    "IfcFixedReferenceSweptAreaSolid",
    "IfcGeometricCurveSet",
    "IfcGeometricSet",
    "IfcGridAxis",
    "IfcGridPlacement",
    "IfcHalfSpaceSolid",
    "IfcIndexedPolyCurve",
    "IfcIndexedPolygonalFace",
    "IfcIndexedPolygonalFaceWithVoids",
    "IfcIntersectionCurve",
    "IfcLine",
    "IfcLocalPlacement",
    "IfcMappedItem",
    "IfcOffsetCurve2D",
    "IfcOffsetCurve3D",
    "IfcOuterBoundaryCurve",
    "IfcPcurve",
    "IfcPlane",
    "IfcPointOnCurve",
    "IfcPointOnSurface",
    "IfcPolygonalBoundedHalfSpace",
    "IfcPolygonalFaceSet",
    "IfcPolyline",
    "IfcRationalBSplineCurveWithKnots",
    "IfcRationalBSplineSurfaceWithKnots",
    "IfcRectangularPyramid",
    "IfcRectangularTrimmedSurface",
    "IfcReparametrisedCompositeCurveSegment",
    "IfcRepresentationMap",
    "IfcRevolvedAreaSolid",
    "IfcRevolvedAreaSolidTapered",
    "IfcRightCircularCone",
    "IfcRightCircularCylinder",
    "IfcSeamCurve",
    "IfcSectionedSpine",
    "IfcShellBasedSurfaceModel",
    "IfcSphere",
    "IfcSphericalSurface",
    "IfcSurfaceCurve",
    "IfcSurfaceCurveSweptAreaSolid",
    "IfcSurfaceOfLinearExtrusion",
    "IfcSurfaceOfRevolution",
    "IfcSweptDiskSolid",
    "IfcSweptDiskSolidPolygonal",
    "IfcToroidalSurface",
    "IfcTriangulatedFaceSet",
    "IfcTrimmedCurve",
    "IfcVector",
    "IfcVirtualGridIntersection",
];

/// Read every Rust source file in the crate, **excluding test modules**.
///
/// Test code names entities in fixtures, so counting it would let a crate
/// "cover" an entity it only ever constructs in a test. Verified by mutation:
/// renaming the string in the production view must fail this gate.
fn crate_source() -> String {
    fn strip_test_modules(text: &str) -> String {
        // Drop everything from `#[cfg(test)]` to the end of the file. Test
        // modules are conventionally last in this crate, and a brace-counting
        // parser would be more machinery than the gate warrants.
        match text.find("#[cfg(test)]") {
            Some(i) => text[..i].to_string(),
            None => text.to_string(),
        }
    }

    fn walk(dir: &std::path::Path, out: &mut String) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                // testkit modules exist to build fixtures; they are not views.
                if path.file_stem().is_some_and(|s| s == "testkit") {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push_str(&strip_test_modules(&text));
                    out.push('\n');
                }
            }
        }
    }
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = String::new();
    walk(&src, &mut out);
    out
}

/// The claim, made falsifiable.
///
/// An entity counts as covered when the crate declares a view type for it --
/// `pub struct Vector` for `IfcVector` -- or names its STEP type string in
/// production code (how dispatchers match). Both forms are real coverage;
/// naming it only inside a `#[cfg(test)]` fixture is not.
#[test]
fn every_concrete_geometry_entity_is_covered() {
    let source = crate_source();
    let upper = source.to_ascii_uppercase();

    let missing: BTreeSet<&str> = CONCRETE_ENTITIES
        .iter()
        .copied()
        .filter(|entity| !is_covered(entity, &source, &upper))
        .collect();

    assert!(
        missing.is_empty(),
        "{} of {} concrete geometry entities have no view in this crate:\n{}",
        missing.len(),
        CONCRETE_ENTITIES.len(),
        missing
            .iter()
            .map(|m| format!("  {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Does production code declare a view for this entity, or match its type?
fn is_covered(entity: &str, source: &str, upper_source: &str) -> bool {
    // A dispatcher matching on the STEP type name, e.g. "IFCEXTRUDEDAREASOLID".
    if upper_source.contains(&format!("\"{}\"", entity.to_ascii_uppercase())) {
        return true;
    }
    // A view type named after the entity minus its `Ifc` prefix.
    let view = entity.strip_prefix("Ifc").unwrap_or(entity);
    source.contains(&format!("pub struct {view}"))
        || source.contains(&format!("pub enum {view}"))
        || source.contains(&format!("pub struct {view}<"))
}

/// The inventory itself must stay honest.
///
/// If someone trims the list to make the gate pass, this catches it.
#[test]
fn the_inventory_matches_the_published_schema_counts() {
    assert_eq!(
        CONCRETE_ENTITIES.len(),
        89,
        "IFC4 ADD2 TC1 declares 89 concrete entities across the three geometry \
         schemas (112 total, 23 abstract). Changing this number means the \
         inventory was edited rather than the code fixed."
    );

    let unique: BTreeSet<&&str> = CONCRETE_ENTITIES.iter().collect();
    assert_eq!(unique.len(), CONCRETE_ENTITIES.len(), "duplicate entries");
}

/// A spot check that the generated list is the real schema, not a guess.
#[test]
fn inventory_contains_the_entities_that_matter_most() {
    for expected in [
        "IfcExtrudedAreaSolid",
        "IfcPolygonalBoundedHalfSpace",
        "IfcBooleanClippingResult",
        "IfcMappedItem",
        "IfcLocalPlacement",
        "IfcTriangulatedFaceSet",
        "IfcTrimmedCurve",
    ] {
        assert!(
            CONCRETE_ENTITIES.contains(&expected),
            "{expected} missing from the inventory"
        );
    }
}

/// Every TYPE (enum, select, defined) in the three geometry schemas.
///
/// Generated from IFC4 ADD2 TC1: 23 types.
const SCHEMA_TYPES: &[(&str, &str)] = &[
    ("IfcArcIndex", "DEFINED"),
    ("IfcAxis2Placement", "SELECT"),
    ("IfcBSplineCurveForm", "ENUM"),
    ("IfcBSplineSurfaceForm", "ENUM"),
    ("IfcBooleanOperand", "SELECT"),
    ("IfcBooleanOperator", "ENUM"),
    ("IfcCsgSelect", "SELECT"),
    ("IfcCurveOnSurface", "SELECT"),
    ("IfcCurveOrEdgeCurve", "SELECT"),
    ("IfcDimensionCount", "DEFINED"),
    ("IfcGeometricSetSelect", "SELECT"),
    ("IfcGridPlacementDirectionSelect", "SELECT"),
    ("IfcKnotType", "ENUM"),
    ("IfcLineIndex", "DEFINED"),
    ("IfcPointOrVertexPoint", "SELECT"),
    ("IfcPreferredSurfaceCurveRepresentation", "ENUM"),
    ("IfcSegmentIndexSelect", "SELECT"),
    ("IfcSolidOrShell", "SELECT"),
    ("IfcSurfaceOrFaceSurface", "SELECT"),
    ("IfcTransitionCode", "ENUM"),
    ("IfcTrimmingPreference", "ENUM"),
    ("IfcTrimmingSelect", "SELECT"),
    ("IfcVectorOrDirection", "SELECT"),
];

/// The contract requires select/enum/defined types, not only entities.
///
/// A select is the part consumers most often skip, because a STEP attribute
/// declared as `IfcBooleanOperand` looks like an ordinary reference. Skipping
/// it produces ad-hoc type-name matching that drifts from the schema.
#[test]
fn every_schema_type_is_modelled() {
    let source = crate_source();

    let missing: Vec<&(&str, &str)> = SCHEMA_TYPES
        .iter()
        .filter(|(name, _)| {
            let stem = name.strip_prefix("Ifc").unwrap_or(name);
            // A Rust type named after it, or the STEP tag matched in code.
            !(source.contains(&format!("pub enum {stem}"))
                || source.contains(&format!("pub struct {stem}"))
                || source.contains(&format!("pub type {stem}"))
                || source.contains(&format!("pub enum {name}"))
                || source.contains(&format!("pub struct {name}"))
                || source.contains(&format!("pub type {name}"))
                || source
                    .to_ascii_uppercase()
                    .contains(&format!("\"{}\"", name.to_ascii_uppercase())))
        })
        .collect();

    assert!(
        missing.is_empty(),
        "{} of {} schema types are not modelled:\n{}",
        missing.len(),
        SCHEMA_TYPES.len(),
        missing
            .iter()
            .map(|(n, f)| format!("  {f:8} {n}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The type inventory must match the published count.
#[test]
fn the_type_inventory_matches_the_schema() {
    assert_eq!(
        SCHEMA_TYPES.len(),
        23,
        "IFC4 ADD2 TC1 declares 23 types across the three geometry schemas \
         (14 + 4 + 5). Editing this number means the inventory was trimmed \
         rather than the code fixed."
    );
    let selects = SCHEMA_TYPES.iter().filter(|(_, f)| *f == "SELECT").count();
    let enums = SCHEMA_TYPES.iter().filter(|(_, f)| *f == "ENUM").count();
    let defined = SCHEMA_TYPES.iter().filter(|(_, f)| *f == "DEFINED").count();
    assert_eq!(
        (selects, enums, defined),
        (13, 7, 3),
        "flavour split per IFC4.exp"
    );
}

/// The compiled-in subtype table must agree with the normative schema.
///
/// `select::subtype` hardcodes inheritance chains so a consumer need not ship
/// a 3 MB `.exp` file. That is only safe while the table matches the source it
/// was generated from, so the relationships selects depend on are re-asserted
/// here against known-correct facts from IFC4.exp.
#[test]
fn the_compiled_subtype_table_agrees_with_the_schema() {
    use ifc_geometry::select::is_a;

    // Concrete solids must satisfy the abstract IfcSolidModel select member.
    for solid in [
        "IFCEXTRUDEDAREASOLID",
        "IFCREVOLVEDAREASOLID",
        "IFCFACETEDBREP",
        "IFCADVANCEDBREP",
        "IFCCSGSOLID",
        "IFCSWEPTDISKSOLID",
        "IFCSURFACECURVESWEPTAREASOLID",
        "IFCFIXEDREFERENCESWEPTAREASOLID",
    ] {
        assert!(is_a(solid, "IFCSOLIDMODEL"), "{solid} is a solid model");
    }

    // Half spaces are NOT solid models: they are a separate select branch.
    for half in [
        "IFCHALFSPACESOLID",
        "IFCPOLYGONALBOUNDEDHALFSPACE",
        "IFCBOXEDHALFSPACE",
    ] {
        assert!(
            !is_a(half, "IFCSOLIDMODEL"),
            "{half} must not classify as a solid model"
        );
    }

    // Curve and surface families.
    for curve in [
        "IFCPOLYLINE",
        "IFCCIRCLE",
        "IFCTRIMMEDCURVE",
        "IFCINDEXEDPOLYCURVE",
    ] {
        assert!(is_a(curve, "IFCCURVE"), "{curve} is a curve");
    }
    for surface in [
        "IFCPLANE",
        "IFCCYLINDRICALSURFACE",
        "IFCRECTANGULARTRIMMEDSURFACE",
    ] {
        assert!(is_a(surface, "IFCSURFACE"), "{surface} is a surface");
    }

    // Cross-family negatives: the table must not over-match.
    assert!(!is_a("IFCPLANE", "IFCCURVE"));
    assert!(!is_a("IFCPOLYLINE", "IFCSURFACE"));
    assert!(!is_a("IFCCARTESIANPOINT", "IFCCURVE"));
}
