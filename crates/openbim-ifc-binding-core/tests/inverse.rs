//! `INVERSE` attributes by name, every referrer and the total count every
//! host binds (#462), over the IFC2X3, IFC4 and IFC4X3 relation fixtures
//! (`openbim-ifc`'s `tests/inverse_attributes.rs` maps them). The hosts'
//! own suites only check their conversion of these answers.

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{BindingError, IfcModel, InverseReference};

fn fixture(name: &str) -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

fn refs(ids: &[u64]) -> Tagged {
    Tagged::List(ids.iter().copied().map(Tagged::Ref).collect())
}

fn code<T: std::fmt::Debug>(result: Result<T, BindingError>) -> &'static str {
    result.expect_err("refused").code()
}

/// The fixtures this build bundles a release for, with whether IsDefinedBy
/// holds the type relationship (IFC2X3) or IsTypedBy does.
fn fixtures() -> Vec<(IfcModel, bool)> {
    let mut out = Vec::new();
    if cfg!(feature = "ifc2x3") {
        out.push((fixture("relations_ifc2x3.ifc"), true));
    }
    if cfg!(feature = "ifc4") {
        out.push((fixture("relations_ifc4.ifc"), false));
    }
    if cfg!(feature = "ifc4x3") {
        out.push((fixture("relations_ifc4x3.ifc"), false));
    }
    assert!(!out.is_empty(), "every build bundles a release");
    out
}

#[test]
fn inverse_names_read_through_the_declared_release() {
    for (model, ifc2x3) in fixtures() {
        if ifc2x3 {
            assert_eq!(
                model.attribute_by_name(17, "IsDefinedBy"),
                Ok(refs(&[20, 36]))
            );
            assert_eq!(
                code(model.attribute_by_name(17, "IsTypedBy")),
                "unknown-attribute"
            );
            assert_eq!(model.attribute_by_name(16, "ObjectTypeOf"), Ok(refs(&[20])));
        } else {
            assert_eq!(model.attribute_by_name(17, "isdefinedby"), Ok(refs(&[36])));
            assert_eq!(model.attribute_by_name(17, "IsTypedBy"), Ok(refs(&[20])));
            assert_eq!(model.attribute_by_name(16, "Types"), Ok(refs(&[20])));
        }
        // SET [0:1]: a list of one, or empty.
        assert_eq!(
            model.attribute_by_name(17, "ContainedInStructure"),
            Ok(refs(&[33]))
        );
        assert_eq!(model.attribute_by_name(11, "Decomposes"), Ok(refs(&[])));
        // Not an aggregate: the one referrer.
        assert_eq!(
            model.attribute_by_name(21, "VoidsElements"),
            Ok(Tagged::Ref(24))
        );
        // Explicit names are unchanged.
        assert!(matches!(
            model.attribute_by_name(17, "Name"),
            Ok(Tagged::Text(_))
        ));
    }
}

#[test]
fn a_single_inverse_reads_null_when_unreferenced_and_refuses_two() {
    for (mut model, _) in fixtures() {
        let opening = 21;
        assert_eq!(
            model.attribute_by_name(opening, "VoidsElements"),
            Ok(Tagged::Ref(24))
        );
        // Point #26 (which voids Wall B through #22) at #21 as well.
        model
            .set_attribute_by_name(26, "RelatedOpeningElement", Tagged::Ref(opening))
            .expect("write");
        assert_eq!(
            code(model.attribute_by_name(opening, "VoidsElements")),
            "invalid-model"
        );
        // The cache follows the edit: #22 is no longer voided.
        assert_eq!(
            model.attribute_by_name(22, "VoidsElements"),
            Ok(Tagged::Null)
        );
    }
}

#[test]
fn an_inverse_name_cannot_be_written() {
    for (mut model, _) in fixtures() {
        let before = model.attributes(17).unwrap();
        assert_eq!(
            code(model.set_attribute_by_name(17, "HasOpenings", refs(&[]))),
            "inverse-attribute"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(
                17,
                "HasOpenings",
                openbim_ifc_binding_core::Plain::Null
            )),
            "inverse-attribute"
        );
        assert_eq!(model.attributes(17).unwrap(), before);
    }
}

#[test]
fn references_and_totals_name_every_referrer() {
    for (model, _) in fixtures() {
        // Wall A: typed (#20 slot 4), voided (#24, #25 slot 4), contained
        // (#33 slot 4), property set (#36 slot 4).
        let referrers = model.inverse_references(17).unwrap();
        let ids: Vec<u64> = referrers.iter().map(|r| r.id).collect();
        assert_eq!(ids, [20, 24, 25, 33, 36]);
        assert!(referrers.iter().all(|r| r.index == 4));
        assert_eq!(model.total_inverses(17), Ok(5));
        assert_eq!(
            model.inverse_references(21).unwrap(),
            [
                InverseReference { id: 24, index: 5 },
                InverseReference { id: 30, index: 4 },
            ]
        );
        assert_eq!(model.total_inverses(36), Ok(0));
        assert_eq!(code(model.total_inverses(999)), "missing-entity");
        assert_eq!(code(model.inverse_references(999)), "missing-entity");
    }
}

#[test]
fn inverse_names_list_the_declaration() {
    for (model, ifc2x3) in fixtures() {
        let names = model.inverse_attribute_names(17).unwrap();
        let defined = names
            .iter()
            .find(|info| info.name == "IsDefinedBy")
            .unwrap();
        let expected = if ifc2x3 {
            "IfcRelDefines"
        } else {
            "IfcRelDefinesByProperties"
        };
        assert_eq!(defined.entity, expected);
        assert_eq!(defined.for_attribute, "RelatedObjects");
        assert!(defined.aggregate);
        assert_eq!((defined.lower, defined.upper), (0, None));
        assert_eq!(defined.declared_by, "IfcObject");
        assert_eq!(names.iter().any(|info| info.name == "IsTypedBy"), !ifc2x3);
        let voids = model.inverse_attribute_names(21).unwrap();
        let voids = voids
            .iter()
            .find(|info| info.name == "VoidsElements")
            .unwrap();
        assert!(!voids.aggregate);
        assert_eq!((voids.lower, voids.upper), (1, Some(1)));
    }
}
