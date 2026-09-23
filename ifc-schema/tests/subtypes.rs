//! Downward subtype queries against the three bundled IFC schemas.
//!
//! The synthetic-schema tests in `openbim-step` prove the walk is correct;
//! these prove it answers the question a consumer actually asks, "every
//! `IfcElement`", on the real inheritance trees, which are deep (IfcWall is
//! five levels under IfcRoot) and differ between versions.

use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};

fn bundled() -> [(&'static str, &'static Schema); 3] {
    [("IFC2X3", ifc2x3()), ("IFC4", ifc4()), ("IFC4X3", ifc4x3())]
}

/// `subtypes` must be exactly `{ y != x : is_a(y, x) }` on real schemas, for
/// every ordered pair. On IFC4X3 that is roughly 770,000 pairs.
#[test]
fn subtypes_is_the_inverse_of_is_a_on_every_bundled_schema() {
    for (label, schema) in bundled() {
        let names: Vec<&str> = schema.entity_names().collect();
        for &ancestor in &names {
            let down: std::collections::HashSet<&str> =
                schema.subtypes(ancestor).into_iter().collect();
            for &candidate in &names {
                let expected = candidate != ancestor && schema.is_a(candidate, ancestor);
                assert_eq!(
                    down.contains(candidate),
                    expected,
                    "{label}: {candidate} under {ancestor}"
                );
            }
        }
    }
}

#[test]
fn every_element_includes_walls_doors_and_their_subtypes() {
    for (label, schema) in bundled() {
        let elements = schema.subtypes("IfcElement");
        for expected in ["IfcWall", "IfcWallStandardCase", "IfcDoor", "IfcBeam"] {
            assert!(
                elements
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(expected)),
                "{label}: {expected} must be an IfcElement"
            );
        }
        assert!(
            !elements
                .iter()
                .any(|name| name.eq_ignore_ascii_case("IfcProject")),
            "{label}: IfcProject is not an element"
        );
    }
}

/// The tree genuinely differs by version, so a query must use the schema the
/// file declares. `IfcBuiltElement` was introduced in IFC4X3.
#[test]
fn the_answer_depends_on_the_schema_version() {
    assert!(ifc4x3()
        .direct_subtypes("IfcElement")
        .contains(&"IfcBuiltElement"));
    assert!(!ifc4()
        .direct_subtypes("IfcElement")
        .contains(&"IfcBuiltElement"));
}

#[test]
fn an_undeclared_name_has_no_subtypes() {
    assert!(ifc4().subtypes("IfcNotAThing").is_empty());
}
