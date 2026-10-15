//! Ledger consistency of `IMPLEMENTED` and `PLANNED`.

use super::*;

#[test]
fn implemented_and_planned_families_do_not_overlap() {
    for name in IMPLEMENTED {
        assert!(
            !PLANNED.iter().any(|(planned, _)| planned == name),
            "{name} is listed as both implemented and planned"
        );
    }
}

#[test]
fn every_planned_family_states_a_concrete_reason() {
    for (name, detail) in PLANNED {
        assert!(!detail.is_empty(), "{name} has no stated reason");
        assert_ne!(
            *detail, "unsupported",
            "{name} must say what specifically is missing"
        );
    }
}
