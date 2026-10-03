//! Name-based `GlobalId`s for the sets and relationships an edit creates.
//!
//! The library has no source of randomness that builds for every binding
//! target (the browser has none without a global build flag; see
//! `ifc-model`'s hashing note), so a new record's `GlobalId` is derived,
//! not drawn: an RFC 9562 version-8 UUID over the edited object's
//! `GlobalId`, the set name, the record's role and the model's next free
//! entity id, packed into IFC's 22-character form. The object's `GlobalId`
//! is unique by the schema, a set name is unique per object, and the next
//! free id moves with every edit, so no two records the library creates
//! collide, and replaying one edit on one file gives the same file.
//!
//! The hash is FNV-1a over 128 bits with a final avalanche; it is a
//! name-to-identity mapping, not a security boundary.

use ifc_model::guid::Guid;

const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

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
    let mut hash = FNV_OFFSET;
    for part in [
        "openbim-ifc/property-edit",
        owner,
        set,
        role,
        &next_id.to_string(),
        &serial.to_string(),
    ] {
        // A length prefix keeps ("ab", "c") apart from ("a", "bc").
        for byte in (part.len() as u64)
            .to_le_bytes()
            .iter()
            .chain(part.as_bytes())
        {
            hash ^= u128::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    let mut bytes = avalanche(hash).to_be_bytes();
    // RFC 9562: version 8 (custom) in the high nibble of octet 6, variant
    // `10` in the top bits of octet 8.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Guid::from_uuid(bytes).to_string()
}

/// Spread every input bit over the whole output (the 64-bit finalizer of
/// MurmurHash3, applied to each half after mixing the halves).
fn avalanche(hash: u128) -> u128 {
    let mix = |mut x: u64| {
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        x ^ (x >> 33)
    };
    let high = (hash >> 64) as u64;
    let low = hash as u64;
    let high = mix(high ^ low.rotate_left(29));
    let low = mix(low ^ high);
    (u128::from(high) << 64) | u128::from(low)
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
}
