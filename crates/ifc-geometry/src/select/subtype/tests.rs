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
/// The declared table version must match the schema the chains came from.
///
/// Spot-checks a chain that IFC4 and IFC4X3 disagree on would be ideal,
/// but the geometry chains are stable across both. This instead proves
/// the constant names a schema whose entities really do resolve here.
#[test]
fn the_declared_table_version_resolves_the_compiled_chains() {
    let schema = ifc_schema::for_version(TABLE_SCHEMA_VERSION).expect("bundled");
    for (entity, chain) in SUPERTYPES {
        assert!(
            schema.entity(entity).is_some(),
            "{entity} is not declared by {TABLE_SCHEMA_VERSION:?}"
        );
        if let Some(parent) = chain.first() {
            assert!(
                schema.is_a(entity, parent),
                "{TABLE_SCHEMA_VERSION:?} does not make {entity} a {parent}"
            );
        }
    }
}
