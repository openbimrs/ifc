//! New `GlobalId`s: random (version 4) and name-based (version 8), #463.
//!
//! Both pack an RFC 9562 UUID into IFC's 22-character form with
//! [`Guid::from_uuid`], the compression the IFC specification defines (the
//! alphabet `0-9 A-Z a-z _ $`, the leading digit carrying the top two bits),
//! so every value parses with [`Guid::parse`] and expands back with
//! [`Guid::to_uuid`].
//!
//! # Random: [`new_guid`]
//!
//! 122 random bits from the operating system's generator (`getrandom`),
//! with the version and variant bits RFC 9562 fixes for version 4: the
//! same kind of identifier IfcOpenShell's `guid.new()` draws.
//! `wasm32-unknown-unknown` has no generator of its own: a build that runs
//! under JavaScript turns on the `js-random` feature, which draws from the
//! host's `crypto.getRandomValues` (browsers; Node 19 and later). Without
//! it, there, the call refuses with [`GuidError::NoRandomSource`] rather
//! than returning something less unique.
//!
//! # Name-based: [`name_based_guid`]
//!
//! A version-8 (custom) UUID over a list of names, so the same names always
//! give the same `GlobalId`, on every target and in every release: a
//! reproducible export, or a record whose identity is derived from
//! another's. The hash is FNV-1a over 128 bits with a final avalanche; it is
//! a name-to-identity mapping, not a security boundary, and it is unique
//! only as far as the names are. The library's own writers (property edits,
//! checked authoring) derive the `GlobalId`s they create this way, from an
//! object's `GlobalId` or a per-batch seed plus the model's next free id.
//!
//! ```
//! use ifc::{name_based_guid, new_guid, Guid};
//!
//! let random = new_guid().expect("the OS has a random source");
//! assert_eq!(random.to_uuid()[6] >> 4, 4, "version 4");
//! assert_eq!(Guid::parse(random.as_str()), Some(random));
//!
//! let a = name_based_guid(&["my-exporter", "wall", "42"]);
//! assert_eq!(a, name_based_guid(&["my-exporter", "wall", "42"]));
//! assert_eq!(a.to_uuid()[6] >> 4, 8, "version 8");
//! assert_ne!(a, name_based_guid(&["my-exporter", "wall", "43"]));
//! ```

use ifc_model::guid::Guid;

const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// Why [`new_guid`] drew no `GlobalId`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GuidError {
    /// This build has no random source: `wasm32-unknown-unknown` without
    /// the `js-random` feature. [`name_based_guid`] needs none.
    NoRandomSource,
    /// The random source refused, for example a JavaScript host without
    /// `crypto.getRandomValues` (Node 18 and older). The text is its
    /// reason.
    RandomSourceFailed(String),
}

impl std::fmt::Display for GuidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRandomSource => f.write_str(
                "no random source in this build (wasm32-unknown-unknown without `js-random`)",
            ),
            Self::RandomSourceFailed(reason) => {
                write!(f, "the random source failed: {reason}")
            }
        }
    }
}

impl std::error::Error for GuidError {}

/// A new random `GlobalId`: an RFC 9562 version-4 UUID in IFC's form.
///
/// # Errors
///
/// [`GuidError::NoRandomSource`] on `wasm32-unknown-unknown` without the
/// `js-random` feature; [`GuidError::RandomSourceFailed`] when the
/// operating system (or the JavaScript host) refuses.
pub fn new_guid() -> Result<Guid, GuidError> {
    random_bytes().map(guid_v4)
}

#[cfg(any(
    not(all(target_arch = "wasm32", target_os = "unknown")),
    feature = "js-random"
))]
fn random_bytes() -> Result<[u8; 16], GuidError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| GuidError::RandomSourceFailed(error.to_string()))?;
    Ok(bytes)
}

#[cfg(not(any(
    not(all(target_arch = "wasm32", target_os = "unknown")),
    feature = "js-random"
)))]
fn random_bytes() -> Result<[u8; 16], GuidError> {
    Err(GuidError::NoRandomSource)
}

/// `bytes` as a version-4 UUID: version `0100` in the high nibble of octet
/// 6, variant `10` in the top bits of octet 8.
fn guid_v4(mut bytes: [u8; 16]) -> Guid {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Guid::from_uuid(bytes)
}

/// The `GlobalId` named by `parts`, in order: an RFC 9562 version-8 UUID,
/// the same for the same parts on every target and in every release.
///
/// Parts are length-prefixed before hashing, so `["ab", "c"]` and
/// `["a", "bc"]` differ. Choose parts that are unique together (a
/// namespace for your tool, then a stable key); two calls with equal parts
/// give one `GlobalId` by design.
#[must_use]
pub fn name_based_guid(parts: &[&str]) -> Guid {
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
    Guid::from_uuid(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// RFC 9562 version and variant of a GlobalId's UUID.
    fn version_and_variant(guid: Guid) -> (u8, u8) {
        let uuid = guid.to_uuid();
        (uuid[6] >> 4, uuid[8] >> 6)
    }

    #[test]
    fn random_guids_are_valid_version_4_and_unique() {
        let mut seen = HashSet::new();
        for _ in 0..10_000 {
            let guid = new_guid().expect("native targets have a random source");
            assert_eq!(Guid::parse(guid.as_str()), Some(guid));
            assert_eq!(version_and_variant(guid), (4, 0b10));
            assert!(seen.insert(guid), "{guid} drawn twice");
        }
    }

    /// The bits a version-4 UUID fixes, and nothing else, are overwritten:
    /// all-zero and all-one draws keep every other bit.
    #[test]
    fn version_4_sets_only_the_version_and_variant_bits() {
        let zero = guid_v4([0; 16]).to_uuid();
        assert_eq!(zero, [0, 0, 0, 0, 0, 0, 0x40, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]);
        let ones = guid_v4([0xff; 16]).to_uuid();
        let mut expected = [0xff; 16];
        expected[6] = 0x4f;
        expected[8] = 0xbf;
        assert_eq!(ones, expected);
    }

    /// Pinned value: a change to the hash would silently change every
    /// `GlobalId` a reproducible export writes, and every `GlobalId` the
    /// library's own writers derive.
    #[test]
    fn name_based_guids_are_reproducible_version_8() {
        let guid = name_based_guid(&["openbim-ifc", "example"]);
        assert_eq!(guid, name_based_guid(&["openbim-ifc", "example"]));
        assert_eq!(version_and_variant(guid), (8, 0b10));
        assert_eq!(guid.as_str(), PINNED);
        assert_ne!(
            name_based_guid(&["ab", "c"]),
            name_based_guid(&["a", "bc"]),
            "parts are length-prefixed"
        );
        assert_ne!(name_based_guid(&[]), name_based_guid(&[""]));
    }

    const PINNED: &str = "347eFU5Y69bPY9V9tumQuV";
}
