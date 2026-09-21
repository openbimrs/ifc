//! Hard-coded arities are checked against the shipped schemas.
//!
//! This crate writes positional slots from constants rather than resolving
//! names at runtime. That is only safe while the constants match the
//! normative schema, so the schema is asked directly here. A silent arity
//! change would otherwise surface as a misplaced value in a written file.

use ifc_schema::{ifc4, ifc4x3};

/// The arities this crate writes, per entity.
const EXPECTED: &[(&str, usize)] = &[
    ("IfcTable", 3),
    ("IfcTableRow", 2),
    ("IfcTableColumn", 5),
    ("IfcRegularTimeSeries", 10),
    ("IfcIrregularTimeSeries", 9),
    ("IfcTimeSeriesValue", 1),
    ("IfcIrregularTimeSeriesValue", 2),
];

#[test]
fn written_arities_match_the_schema() {
    for schema in [ifc4(), ifc4x3()] {
        for (entity, arity) in EXPECTED {
            let attributes = schema.attributes(entity);
            if attributes.is_empty() {
                continue;
            }
            assert_eq!(
                attributes.len(),
                *arity,
                "{entity} arity drifted in {}",
                schema.name()
            );
        }
    }
}

/// IfcTable's derived counts are not instance slots.
///
/// The distinction is easy to get wrong: `IfcSIUnit` DERIVEs
/// `SELF\IfcNamedUnit.Dimensions`, a redeclaration that DOES occupy slot 0
/// and is written as `*`. IfcTable's three DERIVEs introduce new names, so
/// they occupy nothing. Writing them would push Rows and Columns out of
/// position for every reader.
#[test]
fn derived_counts_are_absent_from_the_slot_list() {
    for schema in [ifc4(), ifc4x3()] {
        let names: Vec<String> = schema
            .attributes("IfcTable")
            .iter()
            .map(|attribute| attribute.name.to_ascii_lowercase())
            .collect();
        if names.is_empty() {
            continue;
        }
        for derived in ["numberofcellsinrow", "numberofheadings", "numberofdatarows"] {
            assert!(
                !names.contains(&derived.to_owned()),
                "{derived} must not be an instance slot in {}",
                schema.name()
            );
        }
    }
}
