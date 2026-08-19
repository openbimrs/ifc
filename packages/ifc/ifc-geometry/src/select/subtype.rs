//! Subtype resolution without loading the EXPRESS schema at runtime.
//!
//! # Why this table exists
//!
//! EXPRESS `SELECT` types name *abstract* supertypes. `IfcBooleanOperand`
//! permits `IfcSolidModel`, but no file contains one -- files contain
//! `IfcExtrudedAreaSolid`, four levels below it. Answering "may this entity
//! stand in for that select member" therefore needs the inheritance chain.
//!
//! `ifc-schema` can parse the official `.exp` files and answer exactly this,
//! but requiring it would mean a geometry consumer must ship a 3 MB schema
//! file to interpret a wall. The chains for the geometry-reachable entities
//! are small and change only when the IFC schema does, so they are compiled in
//! as data.
//!
//! # Keeping it honest
//!
//! Generated from `IFC4.exp`. `tests/schema_coverage.rs` cross-checks the
//! table against the same normative source, so drift fails the build rather
//! than silently misclassifying a solid.

/// `(entity, its supertype chain from immediate parent upward)`.
///
/// Uppercase because STEP type names are compared case-insensitively and
/// upper is the form the parser produces.
static SUPERTYPES: &[(&str, &[&str])] = &[
    (
        "IFCADVANCEDBREP",
        &[
            "IFCMANIFOLDSOLIDBREP",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCADVANCEDBREPWITHVOIDS",
        &[
            "IFCADVANCEDBREP",
            "IFCMANIFOLDSOLIDBREP",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCADVANCEDFACE",
        &[
            "IFCFACESURFACE",
            "IFCFACE",
            "IFCTOPOLOGICALREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCAXIS2PLACEMENT2D",
        &[
            "IFCPLACEMENT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCAXIS2PLACEMENT3D",
        &[
            "IFCPLACEMENT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBSPLINECURVE",
        &[
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBSPLINECURVEWITHKNOTS",
        &[
            "IFCBSPLINECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBSPLINESURFACE",
        &[
            "IFCBOUNDEDSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBSPLINESURFACEWITHKNOTS",
        &[
            "IFCBSPLINESURFACE",
            "IFCBOUNDEDSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBLOCK",
        &[
            "IFCCSGPRIMITIVE3D",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBOOLEANCLIPPINGRESULT",
        &[
            "IFCBOOLEANRESULT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBOOLEANRESULT",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCBOUNDARYCURVE",
        &[
            "IFCCOMPOSITECURVEONSURFACE",
            "IFCCOMPOSITECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBOUNDEDCURVE",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBOUNDEDSURFACE",
        &[
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCBOXEDHALFSPACE",
        &[
            "IFCHALFSPACESOLID",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCARTESIANPOINT",
        &[
            "IFCPOINT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCIRCLE",
        &[
            "IFCCONIC",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCLOSEDSHELL",
        &[
            "IFCCONNECTEDFACESET",
            "IFCTOPOLOGICALREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCOMPOSITECURVE",
        &[
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCOMPOSITECURVEONSURFACE",
        &[
            "IFCCOMPOSITECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCONIC",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCSGPRIMITIVE3D",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCCSGSOLID",
        &[
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCURVE",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCCURVEBOUNDEDPLANE",
        &[
            "IFCBOUNDEDSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCURVEBOUNDEDSURFACE",
        &[
            "IFCBOUNDEDSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCYLINDRICALSURFACE",
        &[
            "IFCELEMENTARYSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCDIRECTION",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCEDGECURVE",
        &[
            "IFCEDGE",
            "IFCTOPOLOGICALREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCELEMENTARYSURFACE",
        &[
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCELLIPSE",
        &[
            "IFCCONIC",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCEXTRUDEDAREASOLID",
        &[
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCEXTRUDEDAREASOLIDTAPERED",
        &[
            "IFCEXTRUDEDAREASOLID",
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCFACEBASEDSURFACEMODEL",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCFACESURFACE",
        &[
            "IFCFACE",
            "IFCTOPOLOGICALREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCFACETEDBREP",
        &[
            "IFCMANIFOLDSOLIDBREP",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCFACETEDBREPWITHVOIDS",
        &[
            "IFCFACETEDBREP",
            "IFCMANIFOLDSOLIDBREP",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCFIXEDREFERENCESWEPTAREASOLID",
        &[
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCHALFSPACESOLID",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCINDEXEDPOLYCURVE",
        &[
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCINTERSECTIONCURVE",
        &[
            "IFCSURFACECURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCLINE",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCMANIFOLDSOLIDBREP",
        &[
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCOFFSETCURVE2D",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCOFFSETCURVE3D",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCOUTERBOUNDARYCURVE",
        &[
            "IFCBOUNDARYCURVE",
            "IFCCOMPOSITECURVEONSURFACE",
            "IFCCOMPOSITECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPCURVE",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPLANE",
        &[
            "IFCELEMENTARYSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPOINT",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCPOINTONCURVE",
        &[
            "IFCPOINT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPOINTONSURFACE",
        &[
            "IFCPOINT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPOLYGONALBOUNDEDHALFSPACE",
        &[
            "IFCHALFSPACESOLID",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPOLYGONALFACESET",
        &[
            "IFCTESSELLATEDFACESET",
            "IFCTESSELLATEDITEM",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPOLYLINE",
        &[
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCRATIONALBSPLINECURVEWITHKNOTS",
        &[
            "IFCBSPLINECURVEWITHKNOTS",
            "IFCBSPLINECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCRATIONALBSPLINESURFACEWITHKNOTS",
        &[
            "IFCBSPLINESURFACEWITHKNOTS",
            "IFCBSPLINESURFACE",
            "IFCBOUNDEDSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCRECTANGULARPYRAMID",
        &[
            "IFCCSGPRIMITIVE3D",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCRECTANGULARTRIMMEDSURFACE",
        &[
            "IFCBOUNDEDSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCREVOLVEDAREASOLID",
        &[
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCREVOLVEDAREASOLIDTAPERED",
        &[
            "IFCREVOLVEDAREASOLID",
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCRIGHTCIRCULARCONE",
        &[
            "IFCCSGPRIMITIVE3D",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCRIGHTCIRCULARCYLINDER",
        &[
            "IFCCSGPRIMITIVE3D",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSEAMCURVE",
        &[
            "IFCSURFACECURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSOLIDMODEL",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCSPHERE",
        &[
            "IFCCSGPRIMITIVE3D",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSPHERICALSURFACE",
        &[
            "IFCELEMENTARYSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSURFACE",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCSURFACECURVE",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSURFACECURVESWEPTAREASOLID",
        &[
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSURFACEOFLINEAREXTRUSION",
        &[
            "IFCSWEPTSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSURFACEOFREVOLUTION",
        &[
            "IFCSWEPTSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSWEPTAREASOLID",
        &[
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSWEPTDISKSOLID",
        &[
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSWEPTDISKSOLIDPOLYGONAL",
        &[
            "IFCSWEPTDISKSOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSWEPTSURFACE",
        &[
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCTESSELLATEDFACESET",
        &[
            "IFCTESSELLATEDITEM",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCTOROIDALSURFACE",
        &[
            "IFCELEMENTARYSURFACE",
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCTRIANGULATEDFACESET",
        &[
            "IFCTESSELLATEDFACESET",
            "IFCTESSELLATEDITEM",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCTRIMMEDCURVE",
        &[
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCVECTOR",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCVERTEXPOINT",
        &[
            "IFCVERTEX",
            "IFCTOPOLOGICALREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
];

/// Is `entity` the named type, or any subtype of it?
///
/// The question every EXPRESS `SELECT` resolution reduces to. Comparing type
/// names directly instead of calling this rejects every real file, because
/// select members are usually abstract.
///
/// ```
/// use ifc_geometry::select::is_a;
/// // An extruded area solid IS a solid model, four levels up.
/// assert!(is_a("IFCEXTRUDEDAREASOLID", "IFCSOLIDMODEL"));
/// assert!(!is_a("IFCCARTESIANPOINT", "IFCSOLIDMODEL"));
/// ```
pub fn is_a(entity: &str, ancestor: &str) -> bool {
    if entity.eq_ignore_ascii_case(ancestor) {
        return true;
    }
    supertypes_of(entity)
        .iter()
        .any(|s| s.eq_ignore_ascii_case(ancestor))
}

/// The supertype chain of an entity, immediate parent first.
///
/// Empty for an unknown entity: a type from a newer schema is not an error,
/// it simply matches no select, which is the correct conservative answer.
pub fn supertypes_of(entity: &str) -> &'static [&'static str] {
    SUPERTYPES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(entity))
        .map(|(_, chain)| *chain)
        .unwrap_or(&[])
}

/// Every entity this table knows, for cross-checking against the schema.
pub fn known_entities() -> impl Iterator<Item = &'static str> {
    SUPERTYPES.iter().map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case that motivates the whole table.
    #[test]
    fn a_concrete_solid_satisfies_the_abstract_select_member() {
        assert!(is_a("IFCEXTRUDEDAREASOLID", "IFCSOLIDMODEL"));
        assert!(is_a("IFCFACETEDBREP", "IFCSOLIDMODEL"));
        assert!(is_a("IFCCSGSOLID", "IFCSOLIDMODEL"));
        assert!(is_a("IFCSWEPTDISKSOLID", "IFCSOLIDMODEL"));
    }

    #[test]
    fn an_entity_is_a_itself() {
        assert!(is_a("IFCSOLIDMODEL", "IFCSOLIDMODEL"));
    }

    #[test]
    fn unrelated_entities_do_not_match() {
        assert!(!is_a("IFCCARTESIANPOINT", "IFCSOLIDMODEL"));
        assert!(!is_a("IFCCIRCLE", "IFCSURFACE"));
    }

    /// STEP type names arrive uppercase, but callers may not.
    #[test]
    fn matching_ignores_case() {
        assert!(is_a("IfcExtrudedAreaSolid", "IfcSolidModel"));
        assert!(is_a("ifcextrudedareasolid", "IFCSOLIDMODEL"));
    }

    /// A type from a future schema matches nothing rather than erroring.
    #[test]
    fn unknown_entities_match_nothing_instead_of_panicking() {
        assert!(supertypes_of("IFCFROMTHEFUTURE").is_empty());
        assert!(!is_a("IFCFROMTHEFUTURE", "IFCSOLIDMODEL"));
        assert!(
            is_a("IFCFROMTHEFUTURE", "IFCFROMTHEFUTURE"),
            "identity still holds"
        );
    }

    /// Deep chains must resolve all the way to the root.
    #[test]
    fn chains_reach_the_representation_item_root() {
        assert!(is_a("IFCEXTRUDEDAREASOLID", "IFCREPRESENTATIONITEM"));
        assert!(is_a("IFCADVANCEDBREPWITHVOIDS", "IFCMANIFOLDSOLIDBREP"));
    }
}
