//! Unit tests for compiled subtype resolution.

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
/// Every compiled chain equals the bundled schema's chain, per release.
///
/// The bundled tables are generated from the normative `.exp` files, so this
/// runs in a fresh clone; `tests/schema_coverage.rs` repeats it against the
/// `.exp` files themselves.
#[test]
fn every_verified_release_resolves_the_compiled_chains() {
    for &version in VERIFIED_SCHEMA_VERSIONS {
        let schema = ifc_schema::for_version(version).expect("bundled");
        let mut rows = 0;
        for entity in known_entities_in(version) {
            rows += 1;
            let expected: Vec<String> = schema
                .supertypes(entity)
                .iter()
                .map(|s| s.to_ascii_uppercase())
                .collect();
            assert_eq!(
                supertypes_of_in(version, entity),
                expected,
                "{version:?}: chain of {entity}"
            );
        }
        assert!(rows > 100, "{version:?}: only {rows} rows checked");
    }
    assert!(VERIFIED_SCHEMA_VERSIONS.contains(&TABLE_SCHEMA_VERSION));
}

/// The examples #293 was opened for.
#[test]
fn ifc4x3_only_entities_resolve_to_their_select_members() {
    for (entity, ancestor) in [
        ("IFCCLOTHOID", "IFCCURVE"),
        ("IFCCLOTHOID", "IFCSPIRAL"),
        ("IFCGRADIENTCURVE", "IFCCOMPOSITECURVE"),
        ("IFCSEGMENTEDREFERENCECURVE", "IFCBOUNDEDCURVE"),
        ("IFCTRIANGULATEDIRREGULARNETWORK", "IFCTESSELLATEDFACESET"),
        ("IFCOPENCROSSPROFILEDEF", "IFCPROFILEDEF"),
        ("IFCSECTIONEDSOLIDHORIZONTAL", "IFCSOLIDMODEL"),
        ("IFCSECTIONEDSURFACE", "IFCSURFACE"),
        ("IFCCURVESEGMENT", "IFCSEGMENT"),
        ("IFCAXIS2PLACEMENTLINEAR", "IFCPLACEMENT"),
        ("IFCPOINTBYDISTANCEEXPRESSION", "IFCPOINT"),
        (
            "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
            "IFCSWEPTAREASOLID",
        ),
    ] {
        assert!(is_a(entity, ancestor), "{entity} is a {ancestor}");
        assert!(
            is_a_in(SchemaVersion::Ifc4x3, entity, ancestor),
            "IFC4X3: {entity} is a {ancestor}"
        );
        assert!(
            !is_a_in(SchemaVersion::Ifc4, entity, ancestor),
            "IFC4 does not declare {entity}"
        );
    }
    assert!(
        !is_a("IFCCLOTHOID", "IFCBOUNDEDCURVE"),
        "a spiral is unbounded"
    );
    assert!(
        !is_a("IFCCURVESEGMENT", "IFCCURVE"),
        "a segment is not a curve"
    );
}

/// IFC4X3 rows never change an answer for an entity IFC4 declares.
#[test]
fn the_release_neutral_answer_is_the_ifc4_answer_for_ifc4_entities() {
    for entity in known_entities_in(SchemaVersion::Ifc4) {
        assert_eq!(
            supertypes_of(entity),
            supertypes_of_in(SchemaVersion::Ifc4, entity),
            "{entity}"
        );
    }
    // The four IFC4 entities IFC4X3 re-parents keep their IFC4 chain.
    for (entity, inserted) in [
        ("IFCOFFSETCURVE2D", "IFCOFFSETCURVE"),
        ("IFCOFFSETCURVE3D", "IFCOFFSETCURVE"),
        (
            "IFCFIXEDREFERENCESWEPTAREASOLID",
            "IFCDIRECTRIXCURVESWEPTAREASOLID",
        ),
        (
            "IFCSURFACECURVESWEPTAREASOLID",
            "IFCDIRECTRIXCURVESWEPTAREASOLID",
        ),
    ] {
        assert!(!is_a(entity, inserted), "{entity} unversioned");
        assert!(!is_a_in(SchemaVersion::Ifc4, entity, inserted), "{entity}");
        assert!(is_a_in(SchemaVersion::Ifc4x3, entity, inserted), "{entity}");
    }
}

/// Only IFC4 and IFC4X3 are verified; others fall back, never refuse.
#[test]
fn unverified_releases_fall_back_to_the_release_neutral_answer() {
    assert!(tables_are_verified_for(SchemaVersion::Ifc4));
    assert!(tables_are_verified_for(SchemaVersion::Ifc4x3));
    assert!(!tables_are_verified_for(SchemaVersion::Ifc2x3));
    assert_eq!(known_entities_in(SchemaVersion::Ifc2x3).count(), 0);
    assert!(is_a_in(
        SchemaVersion::Ifc2x3,
        "IFCEXTRUDEDAREASOLID",
        "IFCSOLIDMODEL"
    ));
}
