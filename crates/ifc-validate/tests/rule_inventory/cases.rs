//! The case table, and the header cases.

use ifc_model::Model;
use ifc_validate::Report;

use super::fixtures::{declared, ifc4};

/// One adversarial pair for one rule id.
pub struct Case {
    /// The rule id both fixtures are judged on.
    pub rule: &'static str,
    /// Which written form this pair exercises, for the failure message.
    pub form: &'static str,
    /// Must produce `rule` as an error or warning.
    pub fails: fn() -> Report,
    /// Must not produce `rule` at all.
    pub passes: fn() -> Report,
}

/// Every case, across all families.
pub fn all() -> Vec<&'static Case> {
    HEADER
        .iter()
        .chain(super::structure::CASES)
        .chain(super::types::CASES)
        .chain(super::forms::CASES)
        .chain(super::rules::CASES)
        .collect()
}

const HEADER: &[Case] = &[
    Case {
        rule: "header.schema.missing",
        form: "no FILE_SCHEMA token",
        fails: || ifc4(&Model::new()),
        passes: || ifc4(&declared("IFC4")),
    },
    Case {
        rule: "header.schema.unknown",
        form: "a token no release uses",
        fails: || ifc4(&declared("IFC9_DRAFT")),
        passes: || ifc4(&declared("IFC4")),
    },
    Case {
        rule: "header.implementation_level",
        form: "a level other than 2;1",
        fails: || {
            let mut model = declared("IFC4");
            model.header_mut().implementation_level = "3;1".into();
            ifc4(&model)
        },
        passes: || ifc4(&declared("IFC4")),
    },
];
