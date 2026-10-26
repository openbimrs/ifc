//! The `GlobalId`s a batch derives for the `IfcRoot`s it creates (#548): name-based over
//! the batch seed, the operation, the created entity's type and its count among that
//! type in the operation, never over the entity id, so a journal of batches replays to
//! the same identities after a write/read cycle shifts the model's ids.

use crate::name_guid::name_based_guid;

/// Where in a batch a `GlobalId` is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    /// Operation `index` of an `apply_authoring` batch.
    Op(usize),
    /// An `apply_append`.
    Append,
}

/// The `GlobalId` of the `ordinal`-th entity of `type_name` (any case) that `scope`
/// derives under `seed`.
pub(super) fn derived_global_id(seed: u128, scope: Scope, type_name: &str, ordinal: u32) -> String {
    let scope = match scope {
        Scope::Op(index) => index.to_string(),
        Scope::Append => "append".to_owned(),
    };
    name_based_guid(&[
        "openbim-ifc/authoring/op-role",
        &format!("{seed:032x}"),
        &scope,
        &type_name.to_ascii_uppercase(),
        &ordinal.to_string(),
    ])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> String {
        derived_global_id(7, Scope::Op(0), "IfcSite", 0)
    }

    #[test]
    fn the_same_inputs_give_the_same_global_id() {
        assert_eq!(base(), base());
    }

    #[test]
    fn each_input_changes_the_global_id() {
        let all = [
            base(),
            derived_global_id(8, Scope::Op(0), "IfcSite", 0),
            derived_global_id(7, Scope::Op(1), "IfcSite", 0),
            derived_global_id(7, Scope::Append, "IfcSite", 0),
            derived_global_id(7, Scope::Op(0), "IfcWall", 0),
            derived_global_id(7, Scope::Op(0), "IfcSite", 1),
        ];
        let distinct: std::collections::BTreeSet<_> = all.iter().collect();
        assert_eq!(distinct.len(), all.len());
    }

    #[test]
    fn the_type_name_case_does_not_matter() {
        assert_eq!(base(), derived_global_id(7, Scope::Op(0), "IFCSITE", 0));
    }

    #[test]
    fn the_derivation_is_pinned() {
        // pinned: a change here changes every authored GlobalId and is a breaking release
        assert_eq!(base(), "29Wg4EFkQ6DuENQzQwgRtK");
    }
}
