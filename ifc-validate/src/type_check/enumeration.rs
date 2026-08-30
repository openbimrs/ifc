//! Enumeration constants the schema does not declare.

use ifc_schema::{Schema, TypeKind};

/// Whether `member` is a declared member of enumeration `type_name`.
///
/// Returns `None` when the schema does not declare `type_name` as an
/// enumeration at all, so the caller can distinguish "wrong member" from
/// "not an enumeration".
#[must_use]
pub fn is_member(schema: &Schema, type_name: &str, member: &str) -> Option<bool> {
    let TypeKind::Enumeration(members) = &schema.type_def(type_name)?.kind else {
        return None;
    };
    Some(
        members
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(member)),
    )
}

/// The declared members of an enumeration, for a diagnostic.
#[must_use]
pub fn members<'a>(schema: &'a Schema, type_name: &str) -> Option<&'a [String]> {
    let TypeKind::Enumeration(members) = &schema.type_def(type_name)?.kind else {
        return None;
    };
    Some(members)
}
