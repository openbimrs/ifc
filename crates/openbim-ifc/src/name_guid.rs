//! Name-based `GlobalId`s, shared by the property edit (#123) and checked
//! creation (#330).
//!
//! The library has no source of randomness that builds for every binding
//! target (the browser has none without a global build flag; see
//! `ifc-model`'s hashing note), so a `GlobalId` the library writes is
//! derived from a list of names: an RFC 9562 version-8 UUID over the parts,
//! packed into IFC's 22-character form. The callers choose parts that are
//! unique on their own (an object's `GlobalId`, a caller-supplied seed) and
//! add the model's next free id, so no two records collide.
//!
//! The hash is FNV-1a over 128 bits with a final avalanche; it is a
//! name-to-identity mapping, not a security boundary.
#![cfg(any(feature = "properties", feature = "authoring"))]

use ifc_model::guid::Guid;

const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// The `GlobalId` named by `parts`, in order.
pub(crate) fn name_based_guid(parts: &[&str]) -> String {
    let mut hash = FNV_OFFSET;
    for part in parts {
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
pub(crate) fn avalanche(hash: u128) -> u128 {
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
