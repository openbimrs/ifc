//! Name-based `GlobalId`s for the sets and relationships an edit creates.
//!
//! A new record's `GlobalId` is derived, not drawn (see
//! `crate::name_guid`): from the edited object's `GlobalId`, the set name,
//! the record's role and the model's next free entity id. The object's
//! `GlobalId` is unique by the schema, a set name is unique per object, and
//! the next free id moves with every edit, so no two records the library
//! creates collide, and replaying one edit on one file gives the same file.

use crate::name_guid::name_based_guid;

/// A `GlobalId` for the record playing `role` in set `set` of the object
/// with `GlobalId` `owner`, created while `next_id` is the model's next free
/// id; `serial` tells apart several records of one batch.
pub(super) fn derived_global_id(
    owner: &str,
    set: &str,
    role: &str,
    next_id: u64,
    serial: u64,
) -> String {
    name_based_guid(&[
        "openbim-ifc/property-edit",
        owner,
        set,
        role,
        &next_id.to_string(),
        &serial.to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::derived_global_id;
    use ifc_model::guid::Guid;

    #[test]
    fn a_derived_global_id_is_valid_stable_and_distinct() {
        let one = derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 40, 0);
        assert!(Guid::parse(&one).is_some(), "{one}");
        assert_eq!(
            one,
            derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 40, 0)
        );
        let others = [
            derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "rel", 40, 0),
            derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 41, 0),
            derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 40, 1),
            derived_global_id("3YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 40, 0),
            derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommo", "nset", 40, 0),
        ];
        for other in &others {
            assert_ne!(&one, other);
        }
    }

    /// Moving the hash into `name_guid` (#330) must not change one bit: the
    /// same edit of the same file still gives the same file. The value was
    /// recorded from the implementation before the move.
    #[test]
    fn the_derivation_is_unchanged() {
        assert_eq!(
            derived_global_id("2YvctVUKr0kugbFTf53O9L", "Pset_WallCommon", "set", 40, 0),
            "26hnDcHP_5WeOn50JGQiZ2"
        );
    }
}
