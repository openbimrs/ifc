//! Name-based `GlobalId`s for the sets and relationships an edit creates.
//!
//! A new record's `GlobalId` is derived, not drawn (see
//! `crate::name_guid`): from the batch seed, the edited object's
//! `GlobalId`, the set name, the record's role and a serial (the count of
//! earlier records of that object, set and role in the batch); never from
//! an entity id (#559). The object's `GlobalId` is unique by the schema and
//! a set name is unique per object, so no two records of one batch collide,
//! and the same edits under the same seed give the same `GlobalId`s after a
//! write/read cycle that shifts entity ids.

use crate::name_guid::name_based_guid;

/// A `GlobalId` for the record playing `role` in set `set` of the object
/// with `GlobalId` `owner`, in the batch named by `seed`; `serial` tells
/// apart several records of that object, set and role in the batch.
pub(super) fn derived_global_id(
    seed: u128,
    owner: &str,
    set: &str,
    role: &str,
    serial: u32,
) -> String {
    name_based_guid(&[
        "openbim-ifc/property-edit/seed",
        &format!("{seed:032x}"),
        owner,
        set,
        role,
        &serial.to_string(),
    ])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::derived_global_id;
    use ifc_model::guid::Guid;

    const OWNER: &str = "2YvctVUKr0kugbFTf53O9L";

    #[test]
    fn a_derived_global_id_is_valid_stable_and_distinct() {
        let one = derived_global_id(7, OWNER, "Pset_WallCommon", "set", 0);
        assert!(Guid::parse(&one).is_some(), "{one}");
        assert_eq!(
            one,
            derived_global_id(7, OWNER, "Pset_WallCommon", "set", 0)
        );
        let others = [
            derived_global_id(8, OWNER, "Pset_WallCommon", "set", 0),
            derived_global_id(7, "3YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 0),
            derived_global_id(7, OWNER, "Pset_WallCommo", "nset", 0),
            derived_global_id(7, OWNER, "Pset_WallCommon", "rel", 0),
            derived_global_id(7, OWNER, "Pset_WallCommon", "set", 1),
        ];
        for other in &others {
            assert_ne!(&one, other);
        }
    }

    // pinned: a change here changes every property-edit GlobalId and is a breaking release
    #[test]
    fn the_derivation_is_pinned() {
        assert_eq!(
            derived_global_id(7, OWNER, "Pset_WallCommon", "set", 0),
            "3Vfgvh5D6E9gQN0JkTbI1V"
        );
    }
}
